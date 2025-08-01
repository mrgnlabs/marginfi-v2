use crate::{
    check,
    events::{AccountEventHeader, LendingAccountDepositEvent},
    math_error,
    prelude::*,
    state::{
        bank::BankImpl,
        bank_config::BankConfigImpl,
        marginfi_account::{BankAccountWrapper, LendingAccountImpl, MarginfiAccountImpl},
    },
    utils::{self, validate_asset_tags, validate_bank_state, InstructionKind},
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;
use anchor_lang::solana_program::sysvar::Sysvar;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use fixed::types::I80F48;
use marginfi_type_crate::types::{Bank, MarginfiAccount, MarginfiGroup, ACCOUNT_DISABLED};

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
        authority: signer,
        signer_token_account,
        liquidity_vault: bank_liquidity_vault,
        token_program,
        bank: bank_loader,
        group: marginfi_group_loader,
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
    validate_bank_state(&bank, InstructionKind::FailsIfPausedOrReduceState)?;

    check!(
        !marginfi_account.get_flag(ACCOUNT_DISABLED),
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
                .ok_or_else(math_error!())?
                .checked_floor()
                .ok_or_else(math_error!())?
                .checked_to_num::<u64>()
                .ok_or_else(math_error!())?;

            std::cmp::min(amount, remaining_capacity)
        }
    } else {
        amount
    };

    if deposit_amount == 0 {
        return Ok(());
    }

    let group = &marginfi_group_loader.load()?;
    bank.accrue_interest(
        clock.unix_timestamp,
        group,
        #[cfg(not(feature = "client"))]
        bank_loader.key(),
    )?;

    let mut bank_account = BankAccountWrapper::find_or_create(
        &bank_loader.key(),
        &mut bank,
        &mut marginfi_account.lending_account,
    )?;

    bank_account.deposit(I80F48::from_num(deposit_amount))?;
    marginfi_account.last_update = bank_account.balance.last_update;

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

    bank.deposit_spl_transfer(
        amount_pre_fee,
        signer_token_account.to_account_info(),
        bank_liquidity_vault.to_account_info(),
        signer.to_account_info(),
        maybe_bank_mint.as_ref(),
        token_program.to_account_info(),
        ctx.remaining_accounts,
    )?;

    bank.update_bank_cache(group)?;

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

    marginfi_account.lending_account.sort_balances();

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountDeposit<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group,
        has_one = authority
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = liquidity_vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// CHECK: Token mint/authority are checked at transfer
    #[account(mut)]
    pub signer_token_account: AccountInfo<'info>,

    #[account(mut)]
    pub liquidity_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}
