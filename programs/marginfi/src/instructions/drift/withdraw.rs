use crate::{
    bank_signer, check,
    constants::{DRIFT_PROGRAM_ID, PROGRAM_VERSION},
    events::{AccountEventHeader, LendingAccountWithdrawEvent},
    ix_utils::{get_discrim_hash, Hashable},
    state::{
        bank::{BankImpl, BankVaultType},
        marginfi_account::{
            calc_value, BankAccountWrapper, LendingAccountImpl, MarginfiAccountImpl, RiskEngine,
        },
        marginfi_group::MarginfiGroupImpl,
    },
    utils::{fetch_asset_price_for_bank, is_drift_asset_tag, validate_bank_state, InstructionKind},
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;
use anchor_lang::system_program::System;
use anchor_spl::token::accessor;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};
use bytemuck::Zeroable;
use drift_mocks::drift::cpi::accounts::{UpdateSpotMarketCumulativeInterest, Withdraw};
use drift_mocks::drift::cpi::{update_spot_market_cumulative_interest, withdraw};
use drift_mocks::state::MinimalUser;
use fixed::types::I80F48;
use marginfi_type_crate::constants::LIQUIDITY_VAULT_AUTHORITY_SEED;
use marginfi_type_crate::types::{
    Bank, HealthCache, MarginfiAccount, MarginfiGroup, ACCOUNT_DISABLED, ACCOUNT_IN_RECEIVERSHIP,
};

/// Withdraw from a Drift spot market through a marginfi account
///
/// This function performs the following steps:
/// 1. Updates spot market cumulative interest to ensure calcs are fresh
/// 2. Calculates the scaled balance decrement for the requested token amount
/// 3. Calls bank_account.withdraw() with the scaled amount
/// 4. Performs CPI call to Drift to withdraw the actual token amount
/// 5. Verifies the scaled balance decreased by the expected amount
/// 6. Verifies the liquidity vault received the expected tokens
/// 7. Transfers tokens from liquidity vault to user's destination account
/// 8. Updates health cache and emits events
pub fn drift_withdraw<'info>(
    ctx: Context<'_, '_, 'info, 'info, DriftWithdraw<'info>>,
    amount: u64,
    withdraw_all: Option<bool>,
) -> MarginfiResult {
    let withdraw_all = withdraw_all.unwrap_or(false);
    let authority_bump: u8;
    let market_index: u16;

    ctx.accounts.cpi_update_spot_market_cumulative_interest()?;

    let bank_key = ctx.accounts.bank.key();
    let (token_amount, expected_scaled_balance_change) = {
        let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
        let mut bank = ctx.accounts.bank.load_mut()?;
        let mut group = ctx.accounts.group.load_mut()?;
        let clock = Clock::get()?;
        authority_bump = bank.liquidity_vault_authority_bump;

        validate_bank_state(&bank, InstructionKind::FailsInPausedState)?;

        check!(
            !marginfi_account.get_flag(ACCOUNT_DISABLED),
            MarginfiError::AccountDisabled
        );

        // Validate price is non-zero during liquidation/deleverage to prevent exploits with stale oracles
        let in_receivership = marginfi_account.get_flag(ACCOUNT_IN_RECEIVERSHIP);
        let price = if in_receivership {
            let price =
                fetch_asset_price_for_bank(&bank_key, &bank, &clock, ctx.remaining_accounts)?;

            // Validate price is non-zero during liquidation/deleverage to prevent exploits with stale oracles
            check!(price > I80F48::ZERO, MarginfiError::ZeroAssetPrice);

            price
        } else {
            I80F48::ZERO
        };

        let drift_spot_market = ctx.accounts.drift_spot_market.load()?;
        market_index = drift_spot_market.market_index;

        let mut bank_account = BankAccountWrapper::find(
            &ctx.accounts.bank.key(),
            &mut bank,
            &mut marginfi_account.lending_account,
        )?;

        let (token_amount, expected_scaled_balance_change) = if withdraw_all {
            let scaled_balance = bank_account.withdraw_all()?;

            let token_amount = drift_spot_market.get_withdraw_token_amount(scaled_balance)?;
            let actual_scaled_balance_decrememt =
                drift_spot_market.get_scaled_balance_decrement(token_amount)?;

            // Sanity check
            require_gte!(
                scaled_balance,
                actual_scaled_balance_decrememt,
                MarginfiError::MathError
            );

            (token_amount, actual_scaled_balance_decrememt)
        } else {
            let mut scaled_decrement = drift_spot_market.get_scaled_balance_decrement(amount)?;
            let mut token_amount = amount;

            let asset_shares_i80f48: I80F48 = bank_account.balance.asset_shares.into();
            let asset_shares = asset_shares_i80f48.to_num::<u64>();

            // Handle edge case where scaled_decrement is exactly 1 more than asset_shares
            // This happens when depositing and immediately withdrawing with no interest earned
            // due to Drift rounding up the scaled decrement. We round down manually.
            if scaled_decrement == asset_shares + 1 {
                token_amount = drift_spot_market.get_withdraw_token_amount(asset_shares)?;
                scaled_decrement = drift_spot_market.get_scaled_balance_decrement(token_amount)?;
            }

            bank_account.withdraw(I80F48::from_num(scaled_decrement))?;

            (token_amount, scaled_decrement)
        };

        // Track withdrawal limit for risk admin during deleverage
        if ctx.accounts.authority.key() == group.risk_admin {
            let withdrawn_equity = calc_value(
                I80F48::from_num(expected_scaled_balance_change),
                price,
                bank.mint_decimals,
                None,
            )?;
            group.update_withdrawn_equity(withdrawn_equity, clock.unix_timestamp)?;
        }

        (token_amount, expected_scaled_balance_change)
    };

    // When calling withdraw_all, it's possible that the remaining scaled
    // balance is worth less than 1 unit of token. In this case we skip the
    // withdrawal process and leave the dust inside of drift.
    let actual_amount_received = if withdraw_all && token_amount == 0 {
        // No actual withdrawal occurs, so no tokens received
        0
    } else {
        let initial_scaled_balance = {
            let drift_user = ctx.accounts.drift_user.load()?;
            drift_user.get_scaled_balance(market_index)
        };
        let pre_transfer_vault_balance =
            accessor::amount(&ctx.accounts.liquidity_vault.to_account_info())?;

        ctx.accounts
            .cpi_drift_withdraw(market_index, token_amount, authority_bump)?;

        let final_scaled_balance = {
            let drift_user = ctx.accounts.drift_user.load()?;
            drift_user.get_scaled_balance(market_index)
        };
        let post_transfer_vault_balance =
            accessor::amount(&ctx.accounts.liquidity_vault.to_account_info())?;

        let actual_amount_received = post_transfer_vault_balance - pre_transfer_vault_balance;
        let actual_scaled_balance_change = initial_scaled_balance - final_scaled_balance;

        // We replicate drift math exactly so the numbers should match 1:1
        require_eq!(
            actual_amount_received,
            token_amount,
            MarginfiError::DriftWithdrawFailed
        );
        require_eq!(
            actual_scaled_balance_change,
            expected_scaled_balance_change,
            MarginfiError::DriftScaledBalanceMismatch
        );

        ctx.accounts
            .cpi_transfer_liquidity_vault_to_destination(actual_amount_received)?;
        actual_amount_received
    };

    {
        let mut bank = ctx.accounts.bank.load_mut()?;
        let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
        let group = &ctx.accounts.group.load()?;

        // Update bank cache after modifying balances
        bank.update_bank_cache(group)?;

        marginfi_account.last_update = Clock::get()?.unix_timestamp as u64;

        emit!(LendingAccountWithdrawEvent {
            header: AccountEventHeader {
                signer: Some(ctx.accounts.authority.key()),
                marginfi_account: ctx.accounts.marginfi_account.key(),
                marginfi_account_authority: marginfi_account.authority,
                marginfi_group: marginfi_account.group,
            },
            bank: ctx.accounts.bank.key(),
            mint: bank.mint,
            amount: actual_amount_received,
            close_balance: withdraw_all,
        });

        let mut health_cache = HealthCache::zeroed();
        health_cache.timestamp = Clock::get()?.unix_timestamp;

        marginfi_account.lending_account.sort_balances();

        // Drop the bank mutable borrow before health check (bank is in remaining_accounts)
        drop(bank);

        // Note: during liquidation, we skip all health checks until the end of the transaction.
        if !marginfi_account.get_flag(ACCOUNT_IN_RECEIVERSHIP) {
            let (risk_result, _engine) = RiskEngine::check_account_init_health(
                &marginfi_account,
                ctx.remaining_accounts,
                &mut Some(&mut health_cache),
            );
            risk_result?;

            health_cache.program_version = PROGRAM_VERSION;
            health_cache.set_engine_ok(true);
            marginfi_account.health_cache = health_cache;
        }
    }

    Ok(())
}

#[derive(Accounts)]
pub struct DriftWithdraw<'info> {
    #[account(
        mut,
        constraint = (
            !group.load()?.is_protocol_paused()
        ) @ MarginfiError::ProtocolPaused
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroup,
        constraint = {
            let a = marginfi_account.load()?;
            a.authority == authority.key() || a.get_flag(ACCOUNT_IN_RECEIVERSHIP)
        } @ MarginfiError::Unauthorized
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroup,
        has_one = liquidity_vault @ MarginfiError::InvalidLiquidityVault,
        has_one = drift_spot_market @ MarginfiError::InvalidDriftSpotMarket,
        has_one = drift_user @ MarginfiError::InvalidDriftUser,
        has_one = drift_user_stats @ MarginfiError::InvalidDriftUserStats,
        has_one = mint @ MarginfiError::InvalidMint,
        constraint = is_drift_asset_tag(bank.load()?.config.asset_tag)
            @ MarginfiError::WrongBankAssetTagForDriftOperation,
        // Block withdraw of zero-weight assets during receivership - prevents unfair liquidation
        constraint = {
            let a = marginfi_account.load()?;
            let b = bank.load()?;
            let weight: I80F48 = b.config.asset_weight_init.into();
            !(a.get_flag(ACCOUNT_IN_RECEIVERSHIP) && weight == I80F48::ZERO)
        } @ MarginfiError::LiquidationPremiumTooHigh
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// The oracle account for the asset (not needed if using oracle type QuoteAsset)
    /// CHECK: validated by Drift program
    pub drift_oracle: Option<UncheckedAccount<'info>>,

    /// The bank's liquidity vault authority, which owns the Drift user account
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref()
        ],
        bump = bank.load()?.liquidity_vault_authority_bump
    )]
    pub liquidity_vault_authority: SystemAccount<'info>,

    /// Receives tokens from Drift withdrawal
    #[account(mut)]
    pub liquidity_vault: InterfaceAccount<'info, TokenAccount>,

    /// Token account that will receive the withdrawn tokens
    /// CHECK: Authority is completely unchecked, user controls destination
    #[account(mut)]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    /// The Drift state account
    /// CHECK: validated by the Drift program
    pub drift_state: UncheckedAccount<'info>,

    /// The Drift user account owned by liquidity_vault_authority
    #[account(
        mut,
        constraint = {
            let user = drift_user.load()?;
            let spot_market = drift_spot_market.load()?;
            user.validate_spot_position(spot_market.market_index).is_ok()
        } @ MarginfiError::DriftInvalidSpotPositions,
        constraint = {
            let user = drift_user.load()?;
            user.validate_reward_accounts(
                drift_reward_spot_market.is_none(),
                drift_reward_spot_market_2.is_none(),
            ).is_ok()
        } @ MarginfiError::DriftMissingRewardAccounts,
        constraint = drift_user.load()?.validate_not_bricked_by_admin_deposits().is_ok() @ MarginfiError::DriftBrickedAccount
    )]
    pub drift_user: AccountLoader<'info, MinimalUser>,

    /// The Drift user stats account owned by liquidity_vault_authority
    /// CHECK: validated by the Drift program
    #[account(mut)]
    pub drift_user_stats: UncheckedAccount<'info>,

    /// The Drift spot market for this asset
    #[account(
        mut,
        constraint = drift_spot_market.load()?.mint == mint.key()
            @ MarginfiError::DriftSpotMarketMintMismatch
    )]
    pub drift_spot_market: AccountLoader<'info, drift_mocks::state::MinimalSpotMarket>,

    /// The Drift spot market vault that holds tokens
    /// CHECK: validated by the Drift program
    #[account(mut)]
    pub drift_spot_market_vault: UncheckedAccount<'info>,

    /// Optional: Oracle for first reward asset (only needed if rewards exist)
    /// CHECK: validated by Drift program
    pub drift_reward_oracle: Option<UncheckedAccount<'info>>,

    /// Optional: Spot market for first reward asset (only needed if rewards exist)
    /// CHECK: validated by Drift program
    pub drift_reward_spot_market: Option<UncheckedAccount<'info>>,

    /// Optional: Mint for first reward asset (only needed if rewards exist)
    /// CHECK: validated by Drift program
    pub drift_reward_mint: Option<UncheckedAccount<'info>>,

    /// Optional: Oracle for second reward asset (backup in case multiple rewards)
    /// CHECK: validated by Drift program
    pub drift_reward_oracle_2: Option<UncheckedAccount<'info>>,

    /// Optional: Spot market for second reward asset (backup in case multiple rewards)
    /// CHECK: validated by Drift program
    pub drift_reward_spot_market_2: Option<UncheckedAccount<'info>>,

    /// Optional: Mint for second reward asset (backup in case multiple rewards)
    /// CHECK: validated by Drift program
    pub drift_reward_mint_2: Option<UncheckedAccount<'info>>,

    /// The Drift signer PDA
    /// CHECK: validated by the Drift program
    pub drift_signer: UncheckedAccount<'info>,

    /// Bank's liquidity token mint
    pub mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: validated against hardcoded program id
    #[account(address = DRIFT_PROGRAM_ID)]
    pub drift_program: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> DriftWithdraw<'info> {
    pub fn cpi_update_spot_market_cumulative_interest(&self) -> MarginfiResult {
        let accounts = UpdateSpotMarketCumulativeInterest {
            state: self.drift_state.to_account_info(),
            spot_market: self.drift_spot_market.to_account_info(),
            oracle: self
                .drift_oracle
                .as_ref()
                .map(|o| o.to_account_info())
                .unwrap_or(self.system_program.to_account_info()),
            spot_market_vault: self.drift_spot_market_vault.to_account_info(),
        };

        let program = self.drift_program.to_account_info();
        let cpi_ctx = CpiContext::new(program, accounts);

        update_spot_market_cumulative_interest(cpi_ctx)?;
        Ok(())
    }

    pub fn cpi_drift_withdraw(
        &self,
        market_index: u16,
        amount: u64,
        authority_bump: u8,
    ) -> MarginfiResult {
        let accounts = Withdraw {
            state: self.drift_state.to_account_info(),
            user: self.drift_user.to_account_info(),
            user_stats: self.drift_user_stats.to_account_info(),
            authority: self.liquidity_vault_authority.to_account_info(),
            spot_market_vault: self.drift_spot_market_vault.to_account_info(),
            drift_signer: self.drift_signer.to_account_info(),
            user_token_account: self.liquidity_vault.to_account_info(),
            token_program: self.token_program.to_account_info(),
        };

        let program = self.drift_program.to_account_info();
        let signer_seeds: &[&[&[u8]]] =
            bank_signer!(BankVaultType::Liquidity, self.bank.key(), authority_bump);
        let mut cpi_ctx = CpiContext::new_with_signer(program, accounts, signer_seeds);

        // Construct remaining accounts in the required order for Drift:
        // 1. Oracle accounts (if provided) - main oracle first, then reward oracle
        // 2. Spot market accounts - main spot market first, then reward spot market
        // 3. Token mint (required for Token-2022, harmless to include for regular mints)
        //
        // IMPORTANT: If admin deposits exist in other markets (rewards), you MUST:
        // 1. Include the reward oracle and spot market accounts
        // 2. Harvest the rewards immediately after withdrawal
        // Drift typically only has one reward asset at a time
        let mut remaining_accounts = Vec::new();

        // Add main oracle if provided (not needed if using oracle type QuoteAsset)
        if let Some(oracle) = &self.drift_oracle {
            remaining_accounts.push(oracle.to_account_info());
        }

        // Add first reward oracle if provided (for admin deposits)
        if let Some(reward_oracle) = &self.drift_reward_oracle {
            remaining_accounts.push(reward_oracle.to_account_info());
        }

        // Add second reward oracle if provided (backup for multiple rewards)
        if let Some(reward_oracle_2) = &self.drift_reward_oracle_2 {
            remaining_accounts.push(reward_oracle_2.to_account_info());
        }

        // Always add main spot market account
        remaining_accounts.push(self.drift_spot_market.to_account_info());

        // Add first reward spot market if provided (for admin deposits)
        if let Some(reward_spot_market) = &self.drift_reward_spot_market {
            remaining_accounts.push(reward_spot_market.to_account_info());
        }

        // Add second reward spot market if provided (backup for multiple rewards)
        if let Some(reward_spot_market_2) = &self.drift_reward_spot_market_2 {
            remaining_accounts.push(reward_spot_market_2.to_account_info());
        }

        // Always add main token mint (needed for Token-2022 support)
        remaining_accounts.push(self.mint.to_account_info());

        if let Some(reward_mint) = &self.drift_reward_mint {
            remaining_accounts.push(reward_mint.to_account_info());
        }

        if let Some(reward_mint_2) = &self.drift_reward_mint_2 {
            remaining_accounts.push(reward_mint_2.to_account_info());
        }

        cpi_ctx = cpi_ctx.with_remaining_accounts(remaining_accounts);

        // Call drift withdraw with reduce_only = true (don't allow borrowing)
        withdraw(cpi_ctx, market_index, amount, true)?;
        Ok(())
    }

    pub fn cpi_transfer_liquidity_vault_to_destination(&self, amount: u64) -> MarginfiResult {
        let program = self.token_program.to_account_info();
        let accounts = TransferChecked {
            from: self.liquidity_vault.to_account_info(),
            to: self.destination_token_account.to_account_info(),
            authority: self.liquidity_vault_authority.to_account_info(),
            mint: self.mint.to_account_info(),
        };
        let bank_key = self.bank.key();
        let bump = self.bank.load()?.liquidity_vault_authority_bump;
        let seeds = &[
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank_key.as_ref(),
            &[bump],
        ];
        let signer_seeds: &[&[&[u8]]] = &[seeds];
        let cpi_ctx = CpiContext::new_with_signer(program, accounts, signer_seeds);
        let decimals = self.mint.decimals;
        transfer_checked(cpi_ctx, amount, decimals)?;
        Ok(())
    }
}

impl Hashable for DriftWithdraw<'_> {
    fn get_hash() -> [u8; 8] {
        get_discrim_hash("global", "drift_withdraw")
    }
}
