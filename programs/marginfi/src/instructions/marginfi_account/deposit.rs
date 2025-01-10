use crate::{
    check,
    constants::LIQUIDITY_VAULT_SEED,
    events::{AccountEventHeader, LendingAccountDepositEvent},
    math_error,
    prelude::*,
    state::{
        marginfi_account::{BankAccountWrapper, MarginfiAccount, DISABLED_FLAG},
        marginfi_group::Bank,
    },
    utils::{self, validate_asset_tags},
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenInterface;
use fixed::types::I80F48;
use solana_program::clock::Clock;
use solana_program::sysvar::Sysvar;

/// 1. Accrue interest
/// 2. Create the user's bank account for the asset deposited if it does not exist yet
/// 3. Record asset increase in the bank account
/// 4. Transfer funds from the signer's token account to the bank's liquidity vault
///
/// Will error if there is an existing liability <=> repaying is not allowed.
pub fn lending_account_deposit<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, LendingAccountDeposit<'info>>,
    amount: u64,
    deposit_up_to_limit: Option<bool>,
) -> MarginfiResult {
    let LendingAccountDeposit {
        marginfi_account: marginfi_account_loader,
        signer,
        signer_token_account,
        bank_liquidity_vault,
        token_program,
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
    let deposit_up_to_limit = deposit_up_to_limit.unwrap_or(false);

    let mut bank = bank_loader.load_mut()?;
    let mut marginfi_account = marginfi_account_loader.load_mut()?;

    validate_asset_tags(&bank, &marginfi_account)?;

    check!(
        !marginfi_account.get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

    let deposit_amount = if deposit_up_to_limit && bank.config.is_deposit_limit_active() {
        let current_asset_amount = bank.get_asset_amount(bank.total_asset_shares.into())?;
        let deposit_limit = I80F48::from_num(bank.config.deposit_limit);

        if current_asset_amount >= deposit_limit {
            0
        } else {
            let remaining_capacity = deposit_limit
                .checked_sub(current_asset_amount)
                .ok_or_else(math_error!())?
                .checked_sub(I80F48::ONE) // Subtract 1 to ensure we stay under limit: total_deposits_amount < deposit_limit
                .ok_or_else(math_error!())?;

            let max_deposit = remaining_capacity
                .checked_floor()
                .ok_or_else(math_error!())?
                .checked_to_num::<u64>()
                .ok_or_else(math_error!())?;

            std::cmp::min(amount, max_deposit)
        }
    } else {
        amount
    };

    bank.accrue_interest(
        clock.unix_timestamp,
        &*marginfi_group_loader.load()?,
        #[cfg(not(feature = "client"))]
        bank_loader.key(),
    )?;

    let mut bank_account = BankAccountWrapper::find_or_create(
        &bank_loader.key(),
        &mut bank,
        &mut marginfi_account.lending_account,
    )?;

    bank_account.deposit(I80F48::from_num(deposit_amount))?;

    let amount_pre_fee = maybe_bank_mint
        .as_ref()
        .map(|mint| {
            utils::calculate_pre_fee_spl_deposit_amount(
                mint.to_account_info(),
                deposit_amount,
                clock.epoch,
            )
        })
        .transpose()?
        .unwrap_or(deposit_amount);

    bank_account.deposit_spl_transfer(
        amount_pre_fee,
        signer_token_account.to_account_info(),
        bank_liquidity_vault.to_account_info(),
        signer.to_account_info(),
        maybe_bank_mint.as_ref(),
        token_program.to_account_info(),
        ctx.remaining_accounts,
    )?;

    emit!(LendingAccountDepositEvent {
        header: AccountEventHeader {
            signer: Some(signer.key()),
            marginfi_account: marginfi_account_loader.key(),
            marginfi_account_authority: marginfi_account.authority,
            marginfi_group: marginfi_account.group,
        },
        bank: bank_loader.key(),
        mint: bank.mint,
        amount: deposit_amount,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountDeposit<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = marginfi_account.load()?.group == marginfi_group.key(),
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        address = marginfi_account.load()?.authority,
    )]
    pub signer: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// CHECK: Token mint/authority are checked at transfer
    #[account(mut)]
    pub signer_token_account: AccountInfo<'info>,

    /// CHECK: Seed constraint check
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.liquidity_vault_bump,
    )]
    pub bank_liquidity_vault: AccountInfo<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}
