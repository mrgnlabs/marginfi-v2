// Used by the group admin to edit the default features of staked collateral banks. Remember to
// propagate afterwards.
use crate::state::marginfi_group::{RiskTier, WrappedI80F48};
use crate::state::staked_settings::StakedSettings;
use crate::{set_if_some, MarginfiGroup};
use anchor_lang::prelude::*;

pub fn edit_staked_settings(
    ctx: Context<EditStakedSettings>,
    settings: StakedSettingsEditConfig,
) -> Result<()> {
    // let group = ctx.accounts.marginfi_group.load()?;
    let mut staked_settings = ctx.accounts.staked_settings.load_mut()?;
    // require_keys_eq!(group.admin, ctx.accounts.admin.key());

    set_if_some!(staked_settings.oracle, settings.oracle);
    set_if_some!(
        staked_settings.asset_weight_init,
        settings.asset_weight_init
    );
    set_if_some!(
        staked_settings.asset_weight_maint,
        settings.asset_weight_maint
    );
    set_if_some!(staked_settings.deposit_limit, settings.deposit_limit);
    set_if_some!(
        staked_settings.total_asset_value_init_limit,
        settings.total_asset_value_init_limit
    );
    set_if_some!(staked_settings.oracle_max_age, settings.oracle_max_age);
    set_if_some!(staked_settings.risk_tier, settings.risk_tier);

    Ok(())
}

#[derive(Accounts)]
pub struct EditStakedSettings<'info> {
    #[account(
        has_one = admin
    )]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = marginfi_group
    )]
    pub staked_settings: AccountLoader<'info, StakedSettings>,
}

#[derive(AnchorDeserialize, AnchorSerialize, Default)]
pub struct StakedSettingsEditConfig {
    pub oracle: Option<Pubkey>,

    pub asset_weight_init: Option<WrappedI80F48>,
    pub asset_weight_maint: Option<WrappedI80F48>,

    pub deposit_limit: Option<u64>,
    pub total_asset_value_init_limit: Option<u64>,

    pub oracle_max_age: Option<u16>,
    /// WARN: You almost certainly want "Collateral", using Isolated risk tier makes the asset
    /// worthless as collateral, making all outstanding accounts eligible to be liquidated, and is
    /// generally useful only when creating a staked collateral pool for rewards purposes only.
    pub risk_tier: Option<RiskTier>,
}
