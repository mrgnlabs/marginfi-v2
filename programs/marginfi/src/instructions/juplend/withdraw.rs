use crate::{
    bank_signer, check,
    events::{AccountEventHeader, LendingAccountWithdrawEvent},
    state::{
        bank::{BankImpl, BankVaultType},
        marginfi_account::{
            calc_value, BankAccountWrapper, LendingAccountImpl, MarginfiAccountImpl, RiskEngine,
        },
        marginfi_group::MarginfiGroupImpl,
    },
    utils::{
        fetch_asset_price_for_bank, is_juplend_asset_tag, validate_bank_state, InstructionKind,
    },
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;
use anchor_spl::token::accessor;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};
use bytemuck::Zeroable;
use fixed::types::I80F48;
use juplend_mocks::lending::cpi::accounts::{UpdateRate, Withdraw as WithdrawCpi};
use juplend_mocks::lending::cpi::{update_rate, withdraw as cpi_withdraw};
use juplend_mocks::state::Lending as JuplendLending;
use marginfi_type_crate::constants::LIQUIDITY_VAULT_AUTHORITY_SEED;
use marginfi_type_crate::types::{
    Bank, HealthCache, MarginfiAccount, MarginfiGroup, ACCOUNT_DISABLED, ACCOUNT_IN_RECEIVERSHIP,
};

/// Withdraw underlying tokens from a JupLend lending pool through a marginfi account.
///
/// Flow (program-first, exact-math):
/// 1. CPI `update_rate` to refresh `token_exchange_price`.
/// 2. Enforce same-slot freshness (`last_update_timestamp == Clock::unix_timestamp`).
/// 3. Compute expected fTokens burned: `ceil(assets * 1e12 / token_exchange_price)`.
/// 4. Call `bank_account.withdraw()` for the expected burned shares.
/// 5. CPI `withdraw` (burn fTokens, receive underlying into liquidity vault).
/// 6. Verify received underlying == requested and burned fTokens == expected.
/// 7. Transfer underlying from liquidity vault -> destination token account.
/// 8. Update health cache (unless receivership).
pub fn juplend_withdraw<'info>(
    ctx: Context<'_, '_, 'info, 'info, JuplendWithdraw<'info>>,
    amount: u64,
    withdraw_all: Option<bool>,
) -> MarginfiResult {
    let withdraw_all = withdraw_all.unwrap_or(false);
    // Match marginfi withdraw semantics: withdrawing 0 is a no-op.
    if amount == 0 && !withdraw_all {
        return Ok(());
    }

    // Enforce canonical fToken vault (ATA of liquidity_vault_authority for f_token_mint).
    ctx.accounts.validate_f_token_vault_ata()?;

    // Refresh exchange pricing (interest/rewards) and require it is updated for this slot.
    ctx.accounts.cpi_update_rate()?;
    ctx.accounts.juplend_lending.reload()?;

    let clock = Clock::get()?;
    require!(
        !ctx.accounts.juplend_lending.is_stale(clock.unix_timestamp),
        MarginfiError::JuplendLendingStale
    );

    let bank_key = ctx.accounts.bank.key();
    let authority_bump: u8;

    // Update marginfi internal balances first (tx will revert if CPI fails later).
    //
    // For `withdraw_all`, we:
    // - call `bank_account.withdraw_all()` to close the marginfi position and obtain the full fToken share balance
    // - compute the redeemable underlying = floor(shares * exchange_rate)
    // - CPI JupLend `withdraw` for that underlying amount
    //
    // For partial withdraw, we:
    // - compute shares_to_burn = ceil(assets / exchange_rate)
    // - call `bank_account.withdraw(shares_to_burn)`
    // - CPI JupLend `withdraw` for the requested underlying `amount`
    let (token_amount, shares_to_burn) = {
        let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
        let mut bank = ctx.accounts.bank.load_mut()?;
        let mut group = ctx.accounts.group.load_mut()?;

        authority_bump = bank.liquidity_vault_authority_bump;
        validate_bank_state(&bank, InstructionKind::FailsInPausedState)?;

        check!(
            !marginfi_account.get_flag(ACCOUNT_DISABLED),
            MarginfiError::AccountDisabled
        );

        // Validate price is non-zero during liquidation/deleverage to prevent exploits with stale oracles.
        let in_receivership = marginfi_account.get_flag(ACCOUNT_IN_RECEIVERSHIP);
        let price = if in_receivership {
            let price =
                fetch_asset_price_for_bank(&bank_key, &bank, &clock, ctx.remaining_accounts)?;
            check!(price > I80F48::ZERO, MarginfiError::ZeroAssetPrice);
            price
        } else {
            I80F48::ZERO
        };

        let mut bank_account = BankAccountWrapper::find(
            &ctx.accounts.bank.key(),
            &mut bank,
            &mut marginfi_account.lending_account,
        )?;

        let (token_amount, shares_to_burn) = if withdraw_all {
            // `withdraw_all` returns the user's full fToken share balance (u64).
            let shares_to_burn = bank_account.withdraw_all()?;
            // Redeemable underlying = floor(shares * price / 1e12)
            let token_amount = ctx
                .accounts
                .juplend_lending
                .expected_assets_for_redeem(shares_to_burn)
                .map_err(|_| error!(MarginfiError::MathError))?;

            (token_amount, shares_to_burn)
        } else {
            // shares = ceil(assets * 1e12 / token_exchange_price)
            let shares_to_burn = ctx
                .accounts
                .juplend_lending
                .expected_shares_for_withdraw(amount)
                .map_err(|_| error!(MarginfiError::MathError))?;

            bank_account.withdraw(I80F48::from_num(shares_to_burn))?;

            (amount, shares_to_burn)
        };

        // Track withdrawal limit for risk admin during deleverage.
        if ctx.accounts.authority.key() == group.risk_admin {
            let withdrawn_equity = calc_value(
                I80F48::from_num(shares_to_burn),
                price,
                bank.mint_decimals,
                None,
            )?;
            group.update_withdrawn_equity(withdrawn_equity, clock.unix_timestamp)?;
        }

        (token_amount, shares_to_burn)
    };

    // Record balances to verify exact deltas.
    let pre_liquidity_vault_balance =
        accessor::amount(&ctx.accounts.liquidity_vault.to_account_info())?;
    let pre_f_token_balance = accessor::amount(&ctx.accounts.f_token_vault.to_account_info())?;

    // When calling withdraw_all, it's possible (in theory) that the remaining share balance is
    // worth less than 1 unit of underlying. In this case, we skip the external withdraw and leave
    // the dust inside of JupLend (mirrors the Drift withdraw_all dust behavior).
    let received_underlying = if withdraw_all && token_amount == 0 {
        0
    } else {
        // CPI withdraw: burns fTokens and credits underlying into liquidity vault.
        ctx.accounts
            .cpi_juplend_withdraw(token_amount, authority_bump)?;

        let post_liquidity_vault_balance =
            accessor::amount(&ctx.accounts.liquidity_vault.to_account_info())?;
        let post_f_token_balance = accessor::amount(&ctx.accounts.f_token_vault.to_account_info())?;

        let received_underlying = post_liquidity_vault_balance
            .checked_sub(pre_liquidity_vault_balance)
            .ok_or_else(|| error!(MarginfiError::MathError))?;
        require_eq!(
            received_underlying,
            token_amount,
            MarginfiError::JuplendWithdrawFailed
        );

        let burned_shares = pre_f_token_balance
            .checked_sub(post_f_token_balance)
            .ok_or_else(|| error!(MarginfiError::MathError))?;
        require_eq!(
            burned_shares,
            shares_to_burn,
            MarginfiError::JuplendWithdrawFailed
        );

        // Transfer underlying from liquidity vault -> destination.
        ctx.accounts
            .cpi_transfer_liquidity_vault_to_destination(received_underlying, authority_bump)?;

        received_underlying
    };

    // Post-withdraw accounting + health check.
    {
        let mut bank = ctx.accounts.bank.load_mut()?;
        let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
        let group = &ctx.accounts.group.load()?;

        bank.update_bank_cache(group)?;

        marginfi_account.last_update = clock.unix_timestamp as u64;

        emit!(LendingAccountWithdrawEvent {
            header: AccountEventHeader {
                signer: Some(ctx.accounts.authority.key()),
                marginfi_account: ctx.accounts.marginfi_account.key(),
                marginfi_account_authority: marginfi_account.authority,
                marginfi_group: marginfi_account.group,
            },
            bank: ctx.accounts.bank.key(),
            mint: bank.mint,
            amount: received_underlying,
            close_balance: withdraw_all,
        });

        let mut health_cache = HealthCache::zeroed();
        health_cache.timestamp = clock.unix_timestamp;

        marginfi_account.lending_account.sort_balances();

        // Drop bank mutable borrow before health check (bank is in remaining_accounts).
        drop(bank);

        // Skip health checks during liquidation; checked at end of tx.
        if !marginfi_account.get_flag(ACCOUNT_IN_RECEIVERSHIP) {
            let (risk_result, _engine) = RiskEngine::check_account_init_health(
                &marginfi_account,
                ctx.remaining_accounts,
                &mut Some(&mut health_cache),
            );
            risk_result?;
            health_cache.program_version = crate::constants::PROGRAM_VERSION;
            health_cache.set_engine_ok(true);
            marginfi_account.health_cache = health_cache;
        }
    }

    Ok(())
}

#[derive(Accounts)]
pub struct JuplendWithdraw<'info> {
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
        has_one = juplend_lending @ MarginfiError::InvalidJuplendLending,
        has_one = mint @ MarginfiError::InvalidMint,
        constraint = is_juplend_asset_tag(bank.load()?.config.asset_tag)
            @ MarginfiError::WrongBankAssetTagForJuplendOperation,
        // Block withdraw of zero-weight assets during receivership - prevents unfair liquidation
        constraint = {
            let a = marginfi_account.load()?;
            let b = bank.load()?;
            let weight: I80F48 = b.config.asset_weight_init.into();
            !(a.get_flag(ACCOUNT_IN_RECEIVERSHIP) && weight == I80F48::ZERO)
        } @ MarginfiError::LiquidationPremiumTooHigh
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// Token account that will receive the underlying withdrawal.
    /// WARN: Completely unchecked!
    #[account(mut)]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    /// The bank's liquidity vault authority PDA (acts as signer for JupLend CPIs).
    /// NOTE: JupLend marks the signer as writable in their withdraw instruction.
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.liquidity_vault_authority_bump
    )]
    pub liquidity_vault_authority: SystemAccount<'info>,

    /// Bank liquidity vault (receives underlying from JupLend withdraw).
    #[account(mut)]
    pub liquidity_vault: InterfaceAccount<'info, TokenAccount>,

    /// Underlying mint.
    pub mint: Box<InterfaceAccount<'info, Mint>>,

    /// JupLend lending state account.
    #[account(mut)]
    pub juplend_lending: Account<'info, JuplendLending>,

    /// JupLend fToken mint.
    #[account(
        mut,
        constraint = f_token_mint.key() == juplend_lending.f_token_mint
            @ MarginfiError::InvalidJuplendLending,
    )]
    pub f_token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Bank's fToken vault (ATA of liquidity_vault_authority for f_token_mint).
    #[account(mut)]
    pub f_token_vault: InterfaceAccount<'info, TokenAccount>,

    // ---- JupLend CPI accounts ----
    /// CHECK: validated by the JupLend program
    pub lending_admin: UncheckedAccount<'info>,

    /// CHECK: validated by the JupLend program
    #[account(
        mut,
        constraint = supply_token_reserves_liquidity.key() == juplend_lending.token_reserves_liquidity
            @ MarginfiError::InvalidJuplendLending,
    )]
    pub supply_token_reserves_liquidity: UncheckedAccount<'info>,

    /// CHECK: validated by the JupLend program
    #[account(
        mut,
        constraint = lending_supply_position_on_liquidity.key() == juplend_lending.supply_position_on_liquidity
            @ MarginfiError::InvalidJuplendLending,
    )]
    pub lending_supply_position_on_liquidity: UncheckedAccount<'info>,

    /// CHECK: validated by the JupLend program
    pub rate_model: UncheckedAccount<'info>,

    /// CHECK: validated by the JupLend program
    #[account(mut)]
    pub vault: UncheckedAccount<'info>,

    /// JupLend claim account for liquidity_vault_authority.
    /// TEMPORARY: Mainnet currently requires this account (passing None causes ConstraintMut errors),
    /// but an upcoming upgrade is expected to make it truly optional. The account is never actually
    /// validated or used - you can pass any mutable account. We create the canonical PDA for consistency.
    /// Seeds: ["user_claim", liquidity_vault_authority, mint] on Liquidity program.
    /// CHECK: not validated by JupLend - any mutable account works
    #[account(mut)]
    pub claim_account: UncheckedAccount<'info>,

    /// CHECK: validated by the JupLend program
    #[account(mut)]
    pub liquidity: UncheckedAccount<'info>,

    /// CHECK: validated by the JupLend program
    #[account(mut)]
    pub liquidity_program: UncheckedAccount<'info>,

    /// CHECK: validated by the JupLend program
    pub rewards_rate_model: UncheckedAccount<'info>,

    /// CHECK: validated against hardcoded program id
    #[account(address = juplend_mocks::ID)]
    pub juplend_program: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> JuplendWithdraw<'info> {
    pub fn validate_f_token_vault_ata(&self) -> MarginfiResult {
        let expected = anchor_spl::associated_token::get_associated_token_address_with_program_id(
            &self.liquidity_vault_authority.key(),
            &self.f_token_mint.key(),
            &self.token_program.key(),
        );
        require_keys_eq!(
            self.f_token_vault.key(),
            expected,
            MarginfiError::InvalidJuplendFTokenVault
        );
        Ok(())
    }

    pub fn cpi_update_rate(&self) -> MarginfiResult {
        let accounts = UpdateRate {
            lending: self.juplend_lending.to_account_info(),
            mint: self.mint.to_account_info(),
            f_token_mint: self.f_token_mint.to_account_info(),
            supply_token_reserves_liquidity: self.supply_token_reserves_liquidity.to_account_info(),
            rewards_rate_model: self.rewards_rate_model.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(self.juplend_program.to_account_info(), accounts);
        update_rate(cpi_ctx)?;
        Ok(())
    }

    pub fn cpi_juplend_withdraw(&self, amount: u64, authority_bump: u8) -> MarginfiResult {
        let accounts = WithdrawCpi {
            signer: self.liquidity_vault_authority.to_account_info(),
            owner_token_account: self.f_token_vault.to_account_info(),
            recipient_token_account: self.liquidity_vault.to_account_info(),
            lending_admin: self.lending_admin.to_account_info(),
            lending: self.juplend_lending.to_account_info(),
            mint: self.mint.to_account_info(),
            f_token_mint: self.f_token_mint.to_account_info(),
            supply_token_reserves_liquidity: self.supply_token_reserves_liquidity.to_account_info(),
            lending_supply_position_on_liquidity: self
                .lending_supply_position_on_liquidity
                .to_account_info(),
            rate_model: self.rate_model.to_account_info(),
            vault: self.vault.to_account_info(),
            claim_account: Some(self.claim_account.to_account_info()),
            liquidity: self.liquidity.to_account_info(),
            liquidity_program: self.liquidity_program.to_account_info(),
            rewards_rate_model: self.rewards_rate_model.to_account_info(),
            token_program: self.token_program.to_account_info(),
            associated_token_program: self.associated_token_program.to_account_info(),
            system_program: self.system_program.to_account_info(),
        };

        let signer_seeds: &[&[&[u8]]] =
            bank_signer!(BankVaultType::Liquidity, self.bank.key(), authority_bump);

        let cpi_ctx = CpiContext::new_with_signer(
            self.juplend_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        cpi_withdraw(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn cpi_transfer_liquidity_vault_to_destination(
        &self,
        amount: u64,
        authority_bump: u8,
    ) -> MarginfiResult {
        let program = self.token_program.to_account_info();
        let accounts = TransferChecked {
            from: self.liquidity_vault.to_account_info(),
            to: self.destination_token_account.to_account_info(),
            authority: self.liquidity_vault_authority.to_account_info(),
            mint: self.mint.to_account_info(),
        };

        let signer_seeds: &[&[&[u8]]] =
            bank_signer!(BankVaultType::Liquidity, self.bank.key(), authority_bump);
        let cpi_ctx = CpiContext::new_with_signer(program, accounts, signer_seeds);
        transfer_checked(cpi_ctx, amount, self.mint.decimals)?;
        Ok(())
    }
}
