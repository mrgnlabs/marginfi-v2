use crate::{
    check,
    ix_utils::{
        get_discrim_hash, load_and_validate_instructions, validate_ix_first, validate_ix_last,
        validate_ixes_exclusive, Hashable,
    },
    prelude::*,
    state::marginfi_account::{MarginfiAccountImpl, RiskEngine, RiskRequirementType},
};
use anchor_lang::{prelude::*, solana_program::sysvar};
use bytemuck::Zeroable;
use marginfi_type_crate::{
    constants::ix_discriminators,
    types::{HealthCache, LiquidationRecord, MarginfiAccount, ACCOUNT_IN_RECEIVERSHIP},
};

/// (Permissionless) Begins a liquidation: snapshots the account and marks it in receivership. The
/// liquidator now has full control over the account until the end of the tx.
/// * Fails if account is healthy
/// * Fails if end liquidation instruction isn't at the end of this tx.
/// * Fails if the start liquidation instruction appears more than once in this tx.
/// * Fails if any mrgn instruction other than start, end, withdraw, or repay (or the equivalent
///   from a third party integration) are used within this tx.
pub fn start_liquidation<'info>(
    ctx: Context<'_, '_, 'info, 'info, StartLiquidation<'info>>,
) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
    let mut liq_record = ctx.accounts.liquidation_record.load_mut()?;

    check!(
        !marginfi_account.get_flag(ACCOUNT_IN_RECEIVERSHIP),
        MarginfiError::UnexpectedLiquidationState
    );

    // Note: the liquidator can use the health cache state after this ix concludes to plan their
    // liquidation strategy.
    let mut health_cache = HealthCache::zeroed();
    let risk_engine = RiskEngine::new(&marginfi_account, &ctx.remaining_accounts)?;
    // Note: This will error if healthy
    let (_pre_health, assets, liabs) = risk_engine
        .check_pre_liquidation_condition_and_get_account_health(
            None,
            &mut Some(&mut health_cache),
        )?;
    let (assets_equity, liabilities_equity) = risk_engine
        .get_account_health_components(RiskRequirementType::Equity, &mut Some(&mut health_cache))?;
    marginfi_account.health_cache = health_cache;

    // Snapshot mini lending account and other values to use in later checks
    let record = LiquidationRecord::new(
        ctx.accounts.liquidation_record.key(),
        ctx.accounts.marginfi_account.key(),
        ctx.accounts.liquidation_receiver.key(),
        assets.into(),
        liabs.into(),
        assets_equity.into(),
        liabilities_equity.into(),
        marginfi_account.lending_account,
    );
    *liq_record = record;

    marginfi_account.set_flag(ACCOUNT_IN_RECEIVERSHIP);

    // Introspection logic
    let sysvar = &ctx.accounts.instruction_sysvar;
    let ixes = load_and_validate_instructions(sysvar, None)?;
    validate_ix_first(
        &ixes,
        &ctx.program_id,
        &ix_discriminators::START_LIQUIDATION,
    )?;
    validate_ix_last(&ixes, &ctx.program_id, &ix_discriminators::END_LIQUIDATION)?;
    validate_ixes_exclusive(
        &ixes,
        &ctx.program_id,
        &[
            // Note: since start must be first, it isn't possible to init within the same tx here,
            // so `&ix_discriminators::INIT_LIQUIDATION_RECORD` is not a valid entry.
            &ix_discriminators::START_LIQUIDATION,
            &ix_discriminators::END_LIQUIDATION,
            &ix_discriminators::LENDING_ACCOUNT_WITHDRAW,
            &ix_discriminators::LENDING_ACCOUNT_REPAY,
            // TODO add withdraw/repay from integrator as they are added to the program. Also
            // remember to add a test to ix_utils to validate you added the correct hash.
        ],
    )?;

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
    #[account(mut)]
    pub liquidation_record: AccountLoader<'info, LiquidationRecord>,

    /// This account will have the authority to withdraw/repay as if they are the user authority
    /// until the end of the tx.
    ///
    /// CHECK: no checks whatsoever, liquidator decides this without restriction
    pub liquidation_receiver: UncheckedAccount<'info>,

    /// CHECK: validated aginst known hard-coded sysvar key
    #[account(
        address = sysvar::instructions::id()
    )]
    pub instruction_sysvar: AccountInfo<'info>,
}

impl Hashable for StartLiquidation<'_> {
    fn get_hash() -> [u8; 8] {
        get_discrim_hash("global", "liquidate_start")
    }
}
