use crate::{
    bank_signer, check,
    constants::PROGRAM_VERSION,
    events::{AccountEventHeader, LendingAccountWithdrawEvent},
    ix_utils::{get_discrim_hash, Hashable},
    prelude::*,
    state::{
        bank::{BankImpl, BankVaultType},
        marginfi_account::{
            BankAccountWrapper, LendingAccountImpl, MarginfiAccountImpl, RiskEngine,
        },
        marginfi_group::MarginfiGroupImpl,
    },
    utils::{
        self, fetch_asset_price_for_bank, is_marginfi_asset_tag, validate_bank_state,
        InstructionKind,
    },
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{clock::Clock, sysvar::Sysvar};
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use bytemuck::Zeroable;
use fixed::types::I80F48;
use marginfi_type_crate::{
    constants::LIQUIDITY_VAULT_AUTHORITY_SEED,
    types::{
        Bank, HealthCache, MarginfiAccount, MarginfiGroup, ACCOUNT_DISABLED,
        ACCOUNT_IN_RECEIVERSHIP,
    },
};

/// 1. Accrue interest
/// 2. Find the user's existing bank account for the asset withdrawn
/// 3. Record asset decrease in the bank account
/// 4. Transfer funds from the bank's liquidity vault to the signer's token account
/// 5. Verify that the user account is in a healthy state
///
/// Will error if there is no existing asset <=> borrowing is not allowed.
pub fn lending_account_withdraw<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, LendingAccountWithdraw<'info>>,
    amount: u64,
    withdraw_all: Option<bool>,
) -> MarginfiResult {
    let LendingAccountWithdraw {
        marginfi_account: marginfi_account_loader,
        destination_token_account,
        liquidity_vault: bank_liquidity_vault,
        token_program,
        bank_liquidity_vault_authority,
        bank: bank_loader,
        group: marginfi_group_loader,
        ..
    } = ctx.accounts;
    let clock = Clock::get()?;

    let withdraw_all = withdraw_all.unwrap_or(false);
    let mut marginfi_account = marginfi_account_loader.load_mut()?;

    check!(
        !marginfi_account.get_flag(ACCOUNT_DISABLED),
        MarginfiError::AccountDisabled
    );

    let bank_key = bank_loader.key();
    let maybe_bank_mint;

    {
        let bank = bank_loader.load()?;

        maybe_bank_mint =
            utils::maybe_take_bank_mint(&mut ctx.remaining_accounts, &bank, token_program.key)?;
        let in_liquidation = marginfi_account.get_flag(ACCOUNT_IN_RECEIVERSHIP);
        if in_liquidation {
            // Note: we don't care about the price value, just validating it's non-zero
            fetch_asset_price_for_bank(&bank_key, &bank, &clock, ctx.remaining_accounts)?;
        }
        validate_bank_state(&bank, InstructionKind::FailsInPausedState)?;
    } // release immutable borrow of bank

    {
        let group = &marginfi_group_loader.load()?;

        let mut bank = bank_loader.load_mut()?;

        bank.accrue_interest(
            clock.unix_timestamp,
            group,
            #[cfg(not(feature = "client"))]
            bank_loader.key(),
        )?;

        let liquidity_vault_authority_bump = bank.liquidity_vault_authority_bump;

        let lending_account = &mut marginfi_account.lending_account;
        let mut bank_account =
            BankAccountWrapper::find(&bank_loader.key(), &mut bank, lending_account)?;

        let amount_pre_fee = if withdraw_all {
            // Note: In liquidation, we still want this passed on the books
            bank_account.withdraw_all()?
        } else {
            let amount_pre_fee = maybe_bank_mint
                .as_ref()
                .map(|mint| {
                    utils::calculate_pre_fee_spl_deposit_amount(
                        mint.to_account_info(),
                        amount,
                        clock.epoch,
                    )
                })
                .transpose()?
                .unwrap_or(amount);

            bank_account.withdraw(I80F48::from_num(amount_pre_fee))?;

            amount_pre_fee
        };

        marginfi_account.last_update = clock.unix_timestamp as u64;

        bank.withdraw_spl_transfer(
            amount_pre_fee,
            bank_liquidity_vault.to_account_info(),
            destination_token_account.to_account_info(),
            bank_liquidity_vault_authority.to_account_info(),
            maybe_bank_mint.as_ref(),
            token_program.to_account_info(),
            bank_signer!(
                BankVaultType::Liquidity,
                bank_loader.key(),
                liquidity_vault_authority_bump
            ),
            ctx.remaining_accounts,
        )?;

        emit!(LendingAccountWithdrawEvent {
            header: AccountEventHeader {
                signer: Some(ctx.accounts.authority.key()),
                marginfi_account: marginfi_account_loader.key(),
                marginfi_account_authority: marginfi_account.authority,
                marginfi_group: marginfi_account.group,
            },
            bank: bank_loader.key(),
            mint: bank.mint,
            amount: amount_pre_fee,
            close_balance: withdraw_all,
        });
    }

    let mut health_cache = HealthCache::zeroed();
    health_cache.timestamp = clock.unix_timestamp;

    marginfi_account.lending_account.sort_balances();

    // Note: during liquidating, we skip all health checks until the end of the transaction.
    if !marginfi_account.get_flag(ACCOUNT_IN_RECEIVERSHIP) {
        // Check account health, if below threshold fail transaction
        // Assuming `ctx.remaining_accounts` holds only oracle accounts
        let (risk_result, risk_engine) = RiskEngine::check_account_init_health(
            &marginfi_account,
            ctx.remaining_accounts,
            &mut Some(&mut health_cache),
        );
        risk_result?;
        health_cache.program_version = PROGRAM_VERSION;

        if let Some(engine) = risk_engine {
            if let Ok(price) = engine.get_unbiased_price_for_bank(&bank_key) {
                let group = &marginfi_group_loader.load()?;
                bank_loader
                    .load_mut()?
                    .update_bank_cache(group, Some(price))?;
            }
        }
        health_cache.set_engine_ok(true);
        marginfi_account.health_cache = health_cache;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountWithdraw<'info> {
    #[account(
        constraint = (
            !group.load()?.is_protocol_paused()
        ) @ MarginfiError::ProtocolPaused
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group,
        constraint = {
            let a = marginfi_account.load()?;
            a.authority == authority.key() || a.get_flag(ACCOUNT_IN_RECEIVERSHIP)
        } @MarginfiError::Unauthorized
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    /// Must be marginfi_account's authority, unless in liquidation receivership
    ///
    /// Note: during liquidation, there are no signer checks whatsoever: any key can repay as
    /// long as the invariants checked at the end of liquidation are met.
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = liquidity_vault,
        constraint = is_marginfi_asset_tag(bank.load()?.config.asset_tag)
            @ MarginfiError::WrongAssetTagForStandardInstructions,
        // We want to block withdraw of assets with no weight (e.g. isolated) otherwise the
        // liquidator can just take all of them and the user gets nothing back, which is unfair. For
        // assets with any nominal weight, e.g. 10%, caveat emptor
        constraint = {
            let a = marginfi_account.load()?;
            let b = bank.load()?;
            let weight: I80F48 = b.config.asset_weight_init.into();
            !(a.get_flag(ACCOUNT_IN_RECEIVERSHIP) && weight == I80F48::ZERO)
        } @MarginfiError::LiquidationPremiumTooHigh
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Seed constraint check
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.liquidity_vault_authority_bump,
    )]
    pub bank_liquidity_vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub liquidity_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl Hashable for LendingAccountWithdraw<'_> {
    fn get_hash() -> [u8; 8] {
        get_discrim_hash("global", "lending_account_withdraw")
    }
}
