use crate::bank_signer;
use crate::check;
use crate::ix_utils;
use crate::state::bank::BankImpl;
use crate::utils::NumTraitsWithTolerance;
use crate::{MarginfiError, MarginfiResult};
use anchor_lang::prelude::*;
use anchor_spl::token::accessor;
use anchor_spl::token_interface::{close_account, CloseAccount};
use fixed::types::I80F48;
use marginfi_type_crate::{
    constants::{CLOSE_ENABLED_FLAG, ZERO_AMOUNT_THRESHOLD},
    types::{Bank, BankVaultType, MarginfiGroup},
};

/// `[token_program, liquidity_vault, liquidity_vault_authority, insurance_vault,
/// insurance_vault_authority, fee_vault, fee_vault_authority]`
const CLOSE_VAULT_ACCOUNTS: usize = 7;

/// * force_close - (admin escape hatch) when `Some(true)`, skips the `CLOSE_ENABLED_FLAG` and
///   open-position checks. Intended for legacy pre-0.1.4 banks whose position count is
///   non-authoritative. The zero-shares/zero-emissions checks are always enforced, so a bank still
///   holding value can never be force-closed.
///
/// Remaining accounts (optional, all or none): the bank's token program, then each of its three
/// vaults followed by that vault's authority, in liquidity, insurance, fee order (see
/// `CLOSE_VAULT_ACCOUNTS`). When passed, the vaults are closed with the bank and their rent goes to
/// the admin. Every vault must be empty, otherwise the close fails. Without them the vaults are
/// left open, as before.
pub fn lending_pool_close_bank<'info>(
    ctx: Context<'info, LendingPoolCloseBank<'info>>,
    force_close: Option<bool>,
) -> MarginfiResult {
    ix_utils::check_no_durable_nonce(&ctx.accounts.instruction_sysvar)?;

    let mut group = ctx.accounts.group.load_mut()?;
    // Note: Groups created prior to 0.1.2 have a non-authoritative count here, so subtraction
    // without saturation could reduce the count below zero.
    group.banks = group.banks.saturating_sub(1);

    let bank = ctx.accounts.bank.load()?;

    if !force_close.unwrap_or(false) {
        // banks created prior to 0.1.4 can never be closed because we cannot guarantee an accurate
        // position count for those banks.
        check!(
            bank.get_flag(CLOSE_ENABLED_FLAG),
            MarginfiError::BankCannotClose,
            "Only banks created in 0.1.4 and later can close"
        );
        check!(
            bank.lending_position_count == 0 && bank.borrowing_position_count == 0,
            MarginfiError::BankCannotClose,
            "Only banks with no open positions can close"
        );
    }
    check!(
        I80F48::from(bank.total_asset_shares).is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD)
            && I80F48::from(bank.total_liability_shares)
                .is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
        MarginfiError::BankCannotClose
    );
    check!(
        I80F48::from(bank.emissions_remaining).is_zero_with_tolerance(ZERO_AMOUNT_THRESHOLD),
        MarginfiError::BankCannotClose
    );

    close_vaults(
        &bank,
        ctx.accounts.bank.key(),
        ctx.remaining_accounts,
        &ctx.accounts.admin.to_account_info(),
    )?;

    drop(bank);
    Ok(())
}

/// Closes the bank's three vaults into `admin`, if the caller passed them (see
/// `lending_pool_close_bank`). Fails on a malformed list or a vault that still holds tokens.
fn close_vaults<'info>(
    bank: &Bank,
    bank_key: Pubkey,
    remaining: &[AccountInfo<'info>],
    admin: &AccountInfo<'info>,
) -> MarginfiResult {
    if remaining.is_empty() {
        return Ok(());
    }
    check!(
        remaining.len() == CLOSE_VAULT_ACCOUNTS,
        MarginfiError::BankCannotClose,
        "expected {} vault accounts, got {}",
        CLOSE_VAULT_ACCOUNTS,
        remaining.len()
    );

    let token_program = remaining[0].key();
    check!(
        token_program == anchor_spl::token::ID || token_program == anchor_spl::token_2022::ID,
        MarginfiError::BankCannotClose,
        "first vault account must be the token program"
    );

    let vaults = [
        (
            BankVaultType::Liquidity,
            bank.liquidity_vault,
            bank.liquidity_vault_authority_bump,
        ),
        (
            BankVaultType::Insurance,
            bank.insurance_vault,
            bank.insurance_vault_authority_bump,
        ),
        (
            BankVaultType::Fee,
            bank.fee_vault,
            bank.fee_vault_authority_bump,
        ),
    ];
    for (i, (vault_type, vault_key, authority_bump)) in vaults.into_iter().enumerate() {
        let vault = &remaining[1 + 2 * i];
        let authority = &remaining[2 + 2 * i];

        check!(
            vault.key() == vault_key,
            MarginfiError::BankCannotClose,
            "wrong {:?} vault",
            vault_type
        );
        let expected_authority = Pubkey::create_program_address(
            &[
                vault_type.get_authority_seed(),
                bank_key.as_ref(),
                &[authority_bump],
            ],
            &crate::ID,
        )
        .map_err(|_| MarginfiError::BankCannotClose)?;
        check!(
            authority.key() == expected_authority,
            MarginfiError::BankCannotClose,
            "wrong {:?} vault authority",
            vault_type
        );
        check!(
            *vault.owner == token_program,
            MarginfiError::BankCannotClose,
            "{:?} vault is not owned by the passed token program",
            vault_type
        );
        let amount = accessor::amount(vault)?;
        check!(
            amount == 0,
            MarginfiError::BankCannotClose,
            "{:?} vault still holds {} tokens",
            vault_type,
            amount
        );

        close_account(CpiContext::new_with_signer(
            token_program,
            CloseAccount {
                account: vault.clone(),
                destination: admin.clone(),
                authority: authority.clone(),
            },
            bank_signer!(vault_type, bank_key, authority_bump),
        ))?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolCloseBank<'info> {
    #[account(
        mut,
        has_one = admin @ MarginfiError::Unauthorized,
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroup,
        close = admin,
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: instruction sysvar
    #[account(address = solana_instructions_sysvar::id())]
    pub instruction_sysvar: UncheckedAccount<'info>,
}
