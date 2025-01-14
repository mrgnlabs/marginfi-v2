use crate::{
    bank_signer, check,
    constants::{LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED},
    events::{AccountEventHeader, LendingAccountBorrowEvent},
    math_error,
    prelude::{MarginfiError, MarginfiGroup, MarginfiResult},
    state::{
        bank::Bank,
        marginfi_account::{BankAccountWrapper, MarginfiAccount, DISABLED_FLAG},
        marginfi_group::BankVaultType,
        risk_engine::RiskEngine,
    },
    utils::{self, validate_asset_tags},
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use fixed::types::I80F48;
use solana_program::{clock::Clock, sysvar::Sysvar};

/// 1. Accrue interest
/// 2. Create the user's bank account for the asset borrowed if it does not exist yet
/// 3. Record liability increase in the bank account
/// 4. Transfer funds from the bank's liquidity vault to the signer's token account
/// 5. Verify that the user account is in a healthy state
///
/// Will error if there is an existing asset <=> withdrawing is not allowed.
pub fn lending_account_borrow<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, LendingAccountBorrow<'info>>,
    amount: u64,
) -> MarginfiResult {
    let LendingAccountBorrow {
        marginfi_account: marginfi_account_loader,
        destination_token_account,
        bank_liquidity_vault,
        token_program,
        bank_liquidity_vault_authority,
        bank: bank_loader,
        marginfi_group: marginfi_group_loader,
        ..
    } = ctx.accounts;
    let clock = Clock::get()?;
    let maybe_bank_mint = utils::maybe_take_bank_mint(
        &mut ctx.remaining_accounts,
        &*bank_loader.load()?,
        token_program.key,
    )?;

    let mut marginfi_account = marginfi_account_loader.load_mut()?;
    let group = &marginfi_group_loader.load()?;
    let program_fee_rate: I80F48 = group.fee_state_cache.program_fee_rate.into();

    check!(
        !marginfi_account.get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

    bank_loader.load_mut()?.accrue_interest(
        clock.unix_timestamp,
        group,
        #[cfg(not(feature = "client"))]
        bank_loader.key(),
    )?;

    let mut origination_fee: I80F48 = I80F48::ZERO;
    {
        let mut bank = bank_loader.load_mut()?;

        validate_asset_tags(&bank, &marginfi_account)?;

        let liquidity_vault_authority_bump = bank.liquidity_vault_authority_bump;
        let origination_fee_rate: I80F48 = bank
            .config
            .interest_rate_config
            .protocol_origination_fee
            .into();

        let mut bank_account = BankAccountWrapper::find_or_create(
            &bank_loader.key(),
            &mut bank,
            &mut marginfi_account.lending_account,
        )?;

        // User needs to borrow amount + fee to receive amount
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

        let origination_fee_u64: u64;
        if !origination_fee_rate.is_zero() {
            origination_fee = I80F48::from_num(amount_pre_fee)
                .checked_mul(origination_fee_rate)
                .ok_or_else(math_error!())?;
            origination_fee_u64 = origination_fee.checked_to_num().ok_or_else(math_error!())?;

            // Incurs a borrow that includes the origination fee (but withdraws just the amt)
            bank_account.borrow(I80F48::from_num(amount_pre_fee) + origination_fee)?;
        } else {
            // Incurs a borrow for the amount without any fee
            origination_fee_u64 = 0;
            bank_account.borrow(I80F48::from_num(amount_pre_fee))?;
        }

        bank_account.withdraw_spl_transfer(
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

        emit!(LendingAccountBorrowEvent {
            header: AccountEventHeader {
                signer: Some(ctx.accounts.signer.key()),
                marginfi_account: marginfi_account_loader.key(),
                marginfi_account_authority: marginfi_account.authority,
                marginfi_group: marginfi_account.group,
            },
            bank: bank_loader.key(),
            mint: bank.mint,
            amount: amount_pre_fee + origination_fee_u64,
        });
    } // release mutable borrow of bank

    // The program and/or group fee account gains the origination fee
    {
        let mut bank = bank_loader.load_mut()?;

        if !origination_fee.is_zero() {
            let mut bank_fees_after: I80F48 = bank.collected_group_fees_outstanding.into();

            if !program_fee_rate.is_zero() {
                // Some portion of the origination fee to goes to program fees
                let program_fee_amount: I80F48 = origination_fee
                    .checked_mul(program_fee_rate)
                    .ok_or_else(math_error!())?;
                // The remainder of the origination fee goes to group fees
                bank_fees_after = bank_fees_after
                    .saturating_add(origination_fee.saturating_sub(program_fee_amount));

                // Update the bank's program fees
                let program_fees_before: I80F48 = bank.collected_program_fees_outstanding.into();
                bank.collected_program_fees_outstanding = program_fees_before
                    .saturating_add(program_fee_amount)
                    .into();
            } else {
                // If program fee rate is zero, add the full origination fee to group fees
                bank_fees_after = bank_fees_after.saturating_add(origination_fee);
            }

            // Update the bank's group fees
            bank.collected_group_fees_outstanding = bank_fees_after.into();
        }
    }

    // Check account health, if below threshold fail transaction
    // Assuming `ctx.remaining_accounts` holds only oracle accounts
    RiskEngine::check_account_init_health(&marginfi_account, ctx.remaining_accounts)?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountBorrow<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = marginfi_account.load() ?.group == marginfi_group.key(),
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        address = marginfi_account.load() ?.authority,
    )]
    pub signer: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load() ?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Seed constraint check
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load() ?.liquidity_vault_authority_bump,
    )]
    pub bank_liquidity_vault_authority: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load() ?.liquidity_vault_bump,
    )]
    pub bank_liquidity_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}
