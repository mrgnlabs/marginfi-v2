use crate::{
    check,
    prelude::*,
    state::marginfi_account::{MarginfiAccountImpl, RiskEngine},
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenInterface;
use bytemuck::Zeroable;
use marginfi_type_crate::types::{
    HealthCache, LiquidationRecord, MarginfiAccount, ACCOUNT_IN_RECEIVERSHIP,
};

/// (Permissionless) Begins a liquidation: snapshots the account and marks it in receivership. The
/// liquidator now has full control over the account until the end of the tx.
/// * Fails if account is healthy
/// * Fails if end liquidation instruction isn't at the end of this tx.
/// * Fails if the start liquidation instruction appears more than once in this tx.
pub fn start_liquidation<'info>(
    ctx: Context<'_, '_, 'info, 'info, StartLiquidation<'info>>,
) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
    let mut liq_record = ctx.accounts.liquidation_record.load_mut()?;

    check!(
        !marginfi_account.get_flag(ACCOUNT_IN_RECEIVERSHIP),
        MarginfiError::UnexpectedLiquidationState
    );

    let mut health_cache = HealthCache::zeroed();
    let risk_engine = RiskEngine::new(&marginfi_account, &ctx.remaining_accounts)?;
    // This will error if healthy
    let (_pre_health, assets, liabs) = risk_engine
        .check_pre_liquidation_condition_and_get_account_health(
            None,
            &mut Some(&mut health_cache),
        )?;
    marginfi_account.health_cache = health_cache;

    // Snapshot mini lending account and other values to use in later checks
    let record = LiquidationRecord::new(
        ctx.accounts.liquidation_record.key(),
        ctx.accounts.marginfi_account.key(),
        ctx.accounts.liquidation_receiver.key(),
        assets.into(),
        liabs.into(),
        marginfi_account.lending_account,
    );
    *liq_record = record;

    marginfi_account.set_flag(ACCOUNT_IN_RECEIVERSHIP);

    // TODO: TX introspection logic

    Ok(())
}

#[derive(Accounts)]
pub struct StartLiquidation<'info> {
    /// Account under liquidation
    #[account(
        mut,
        has_one = liquidation_record
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    /// The associated liquidation record PDA for the given `marginfi_account`
    #[account(
        mut,
        has_one = marginfi_account
    )]
    pub liquidation_record: AccountLoader<'info, LiquidationRecord>,

    /// This account will have the authority to withdraw/repay as if they are the user authority
    /// until the end of the tx.
    ///
    /// CHECK: no checks whatsoever, liquidator decides this without restriction
    pub liquidation_receiver: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    // TODO sysvar check for intropsection
}
