use crate::{
    check,
    constants::{LIQUIDATION_DOLLAR_THRESHOLD, LIQUIDATION_MAX_FEE_MINIMUM},
    ix_utils::{get_discrim_hash, Hashable},
    prelude::*,
    state::marginfi_account::{MarginfiAccountImpl, RiskEngine, RiskRequirementType},
};
use anchor_lang::prelude::*;
use bytemuck::Zeroable;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use marginfi_type_crate::{
    constants::FEE_STATE_SEED,
    types::{FeeState, HealthCache, LiquidationRecord, MarginfiAccount, ACCOUNT_IN_RECEIVERSHIP},
};

// TODO more detail
/// (Permissionless) Ends a liquidation
pub fn end_liquidation<'info>(
    ctx: Context<'_, '_, 'info, 'info, EndLiquidation<'info>>,
) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
    let mut liq_record = ctx.accounts.liquidation_record.load_mut()?;
    let fee_state = ctx.accounts.fee_state.load()?;

    check!(
        marginfi_account.get_flag(ACCOUNT_IN_RECEIVERSHIP),
        MarginfiError::EndNotLast
    );

    let pre_assets: I80F48 = liq_record.cache.asset_value_maint.into();
    let pre_liabs: I80F48 = liq_record.cache.liability_value_maint.into();
    let pre_assets_equity: I80F48 = liq_record.cache.asset_value_equity.into();
    let pre_liabs_equity: I80F48 = liq_record.cache.liability_value_equity.into();
    let pre_health: I80F48 = pre_assets - pre_liabs;
    // Accounts worth less than the threshold can be fully liquidated fully, regardless of health
    let ignore_health = pre_assets_equity < LIQUIDATION_DOLLAR_THRESHOLD;

    // Validate health still negative and load risk engine info
    let mut post_hc = HealthCache::zeroed();
    let risk_engine = RiskEngine::new(&marginfi_account, &ctx.remaining_accounts)?;
    // Note: This will error if healthy, we guarantee that liquidation improves health to at most 0,
    // unless the account's net value is below the threshold, then we can clear it regardless (or not).
    let (post_health, _post_assets, _post_liabs) = risk_engine
        .check_pre_liquidation_condition_and_get_account_health(
            None,
            &mut Some(&mut post_hc),
            ignore_health,
        )?;
    let (post_assets_equity, post_liabilities_equity) = risk_engine
        .get_account_health_components(RiskRequirementType::Equity, &mut Some(&mut post_hc))?;
    marginfi_account.health_cache = post_hc;

    // validate health has improved.
    check!(post_health > pre_health, MarginfiError::HealthDidNotImprove);

    // ??? Do we care about this, as long as you improved health maybe you can claim as much as you want?
    // ensure seized asset‐value ≤ 105% of repaid liability‐value
    let seized: I80F48 = pre_assets_equity - post_assets_equity;
    let repaid: I80F48 = pre_liabs_equity - post_liabilities_equity;
    // Liquidator's allowed fee cannot go lower than LIQUIDATION_MAX_FEE_MINIMUM
    let max_fee: I80F48 = I80F48::max(
        fee_state.liquidation_max_fee.into(),
        I80F48!(1) + LIQUIDATION_MAX_FEE_MINIMUM,
    );

    if !ignore_health {
        check!(
            seized <= repaid * max_fee,
            MarginfiError::LiquidationPremiumTooHigh
        );
    }

    // ??? Is this better/more flexible than debiting the insurance fee in the repaid asset?
    let liquidation_flat_sol_fee = fee_state.liquidation_flat_sol_fee;
    if liquidation_flat_sol_fee > 0 {
        anchor_lang::system_program::transfer(
            ctx.accounts.transfer_flat_fee(),
            liquidation_flat_sol_fee as u64,
        )?;
    }

    // clear receivership
    marginfi_account.unset_flag(ACCOUNT_IN_RECEIVERSHIP);
    liq_record.liquidation_receiver = Pubkey::default();

    // record the entry in the liquidation record
    {
        let seized_f64 = seized.to_num::<f64>();
        let repaid_f64 = repaid.to_num::<f64>();

        // Rotate left to eject the oldest entry
        liq_record.entries.rotate_left(1);
        let entry = &mut liq_record.entries[3];

        entry.asset_amount_seized = seized_f64.to_le_bytes();
        entry.liab_amount_repaid = repaid_f64.to_le_bytes();
        entry.timestamp = Clock::get()?.unix_timestamp;
    }

    // TODO emit event
    Ok(())
}

#[derive(Accounts)]
pub struct EndLiquidation<'info> {
    /// Account under liquidation
    #[account(
        mut,
        has_one = liquidation_record
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    /// The associated liquidation record PDA for the given `marginfi_account`
    #[account(
        mut,
        has_one = liquidation_receiver
    )]
    pub liquidation_record: AccountLoader<'info, LiquidationRecord>,

    // Note: mutable signer because it must pay the transfer fee
    #[account(mut)]
    pub liquidation_receiver: Signer<'info>,

    // Note: there is just one FeeState per program, so no further check is required.
    #[account(
        seeds = [FEE_STATE_SEED.as_bytes()],
        bump,
        has_one = global_fee_wallet
    )]
    pub fee_state: AccountLoader<'info, FeeState>,

    /// CHECK: The fee admin's native SOL wallet, validated against fee state
    #[account(mut)]
    pub global_fee_wallet: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> EndLiquidation<'info> {
    fn transfer_flat_fee(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, anchor_lang::system_program::Transfer<'info>> {
        CpiContext::new(
            self.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                // ??? Do we want to support the fee being paid by a different account? This may
                // help CPI consumers, but adds another account.
                from: self.liquidation_receiver.to_account_info(),
                to: self.global_fee_wallet.to_account_info(),
            },
        )
    }
}

impl Hashable for EndLiquidation<'_> {
    fn get_hash() -> [u8; 8] {
        get_discrim_hash("global", "liquidate_end")
    }
}
