use crate::constants::FREEZE_SETTINGS;
use crate::set_if_some;
use crate::state::marginfi_group::InterestRateConfigOpt;
use crate::{
    state::marginfi_group::{Bank, MarginfiGroup},
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;

pub fn lending_pool_configure_bank_interest_only(
    ctx: Context<LendingPoolConfigureBankInterestOnly>,
    interest_rate_config: InterestRateConfigOpt,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    // If settings are frozen, interest rates can't update.
    if bank.get_flag(FREEZE_SETTINGS) {
        msg!("WARN: Bank settings frozen, did nothing.");
    } else {
        bank.config
            .interest_rate_config
            .update(&interest_rate_config);
        msg!("Bank configured!");
    }

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolConfigureBankInterestOnly<'info> {
    #[account(
        mut,
        has_one = delegate_curve_admin @ MarginfiError::InvalidDelegateCurveAdminConstraint,
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    pub delegate_curve_admin: Signer<'info>,

    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroupConstraint,
    )]
    pub bank: AccountLoader<'info, Bank>,
}

pub fn lending_pool_configure_bank_limits_only(
    ctx: Context<LendingPoolConfigureBankLimitsOnly>,
    deposit_limit: Option<u64>,
    borrow_limit: Option<u64>,
    total_asset_value_init_limit: Option<u64>,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    // If settings are frozen, only deposit and borrow limits can update
    if bank.get_flag(FREEZE_SETTINGS) {
        msg!("WARN: Bank settings frozen, only deposit/borrow limits update.");
        // Note: total_asset_value_init_limit is somewhat risky because it can reduce the value of
        // existing deposited assets, which is why it remains frozen for e.g. arena banks.
        set_if_some!(bank.config.deposit_limit, deposit_limit);
        set_if_some!(bank.config.borrow_limit, borrow_limit);
    } else {
        set_if_some!(bank.config.deposit_limit, deposit_limit);
        set_if_some!(bank.config.borrow_limit, borrow_limit);
        set_if_some!(
            bank.config.total_asset_value_init_limit,
            total_asset_value_init_limit
        );
        msg!("Bank configured!");
    }

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolConfigureBankLimitsOnly<'info> {
    #[account(
        mut,
        has_one = delegate_limit_admin @ MarginfiError::InvalidDelegateLimitAdminConstraint,
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    pub delegate_limit_admin: Signer<'info>,

    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroupConstraint,
    )]
    pub bank: AccountLoader<'info, Bank>,
}
