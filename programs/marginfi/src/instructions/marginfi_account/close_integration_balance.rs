use anchor_lang::prelude::*;
use marginfi_type_crate::types::{
    Bank, MarginfiAccount, MarginfiGroup, ACCOUNT_DISABLED, ACCOUNT_IN_RECEIVERSHIP,
};

use crate::{
    check,
    prelude::*,
    state::{
        bank::BankImpl,
        marginfi_account::{
            account_not_frozen_for_authority, is_signer_authorized, BankAccountWrapper,
            LendingAccountImpl, MarginfiAccountImpl,
        },
    },
    utils::is_integration_asset_tag,
};

/// Closes an integration (Kamino/Drift/Solend/JupLend) balance.
///
/// `lending_account_close_balance` is restricted to `ASSET_TAG_DEFAULT`, `ASSET_TAG_SOL`, and
/// `ASSET_TAG_STAKED` via `is_marginfi_asset_tag`, so it cannot close an integration venue. When
/// such a venue is emptied but still holds dust asset shares below `ZERO_AMOUNT_THRESHOLD`, the
/// balance is inert yet still occupies one of the account's limited balance slots. With no way to
/// withdraw the dust (it is below the withdraw tolerance) and no way to repay (there is no
/// liability), the slot stays occupied for the lifetime of the account unless the whole account is
/// closed. This instruction exists solely to release that slot.
///
/// The integration bank remains subject to every check `lending_account_close_balance` performs
/// (not disabled, no outstanding debt, no material assets); the only difference is the asset-tag
/// gate, which is widened to include integration tags. Callers still must first withdraw every
/// withdrawable token through the venue's own withdraw instruction — an integration position with
/// a real (non-dust) balance cannot be closed here.
pub fn lending_account_close_integration_balance(
    ctx: Context<LendingAccountCloseIntegrationBalance>,
) -> MarginfiResult {
    let LendingAccountCloseIntegrationBalance {
        marginfi_account,
        bank: bank_loader,
        group: marginfi_group_loader,
        ..
    } = ctx.accounts;

    let mut marginfi_account = marginfi_account.load_mut()?;
    let mut bank = bank_loader.load_mut()?;

    check!(
        !marginfi_account.get_flag(ACCOUNT_DISABLED),
        MarginfiError::AccountDisabled
    );

    // An integration venue must be emptied through its own withdraw path; this instruction only
    // releases the slot once nothing but dust remains, so a genuinely non-empty position is a
    // caller bug rather than a state to silently tolerate.
    let has_integration_position = marginfi_account
        .lending_account
        .balances
        .iter()
        .any(|balance| balance.is_active() && balance.bank_pk == bank_loader.key());
    check!(has_integration_position, MarginfiError::BankAccountNotFound);

    let group = &*marginfi_group_loader.load()?;
    bank.accrue_interest(
        Clock::get()?.unix_timestamp,
        group,
        #[cfg(not(feature = "client"))]
        bank_loader.key(),
    )?;

    bank.update_bank_cache(group)?;

    let in_receivership = marginfi_account.get_flag(ACCOUNT_IN_RECEIVERSHIP);
    let lending_account: &mut marginfi_type_crate::types::LendingAccount =
        &mut marginfi_account.lending_account;
    let mut bank_account =
        BankAccountWrapper::find(&bank_loader.key(), &mut bank, lending_account)?;

    bank_account.close_balance(in_receivership)?;
    lending_account.sort_balances();
    marginfi_account.sync_indexer_flags();
    marginfi_account.last_update = Clock::get()?.unix_timestamp as u64;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountCloseIntegrationBalance<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroup,
        constraint = {
            let a = marginfi_account.load()?;
            account_not_frozen_for_authority(&a, authority.key())
        } @ MarginfiError::AccountFrozen,
        constraint = {
            let a = marginfi_account.load()?;
            let g = group.load()?;
            is_signer_authorized(&a, g.governance_admin, authority.key(), false, false, false)
        } @ MarginfiError::Unauthorized
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroup,
        constraint = is_integration_asset_tag(bank.load()?.config.asset_tag)
            @ MarginfiError::WrongAssetTagForIntegrationInstructions
    )]
    pub bank: AccountLoader<'info, Bank>,
}
