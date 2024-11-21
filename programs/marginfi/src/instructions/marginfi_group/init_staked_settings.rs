// Used by the group admin to enable staked collateral banks and configure their default features
use crate::constants::STAKED_SETTINGS_SEED;
use crate::state::marginfi_group::{RiskTier, WrappedI80F48};
use crate::state::staked_settings::StakedSettings;
use crate::MarginfiGroup;
use anchor_lang::prelude::*;

pub fn initialize_staked_settings(
    ctx: Context<InitStakedSettings>,
    settings: StakedSettingsConfig,
) -> Result<()> {
    let mut staked_settings = ctx.accounts.staked_settings.load_init()?;

    *staked_settings = StakedSettings::new(
        ctx.accounts.staked_settings.key(),
        ctx.accounts.marginfi_group.key(),
        settings.oracle,
        settings.asset_weight_init,
        settings.asset_weight_maint,
        settings.deposit_limit,
        settings.total_asset_value_init_limit,
        settings.oracle_max_age,
        settings.risk_tier,
    );

    Ok(())
}

#[derive(Accounts)]
pub struct InitStakedSettings<'info> {
    #[account(
        has_one = admin
    )]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    pub admin: Signer<'info>,

    /// Pays the init fee
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    #[account(
        init,
        seeds = [
            STAKED_SETTINGS_SEED.as_bytes(),
            marginfi_group.key().as_ref()
        ],
        bump,
        payer = fee_payer,
        space = 8 + StakedSettings::LEN,
    )]
    pub staked_settings: AccountLoader<'info, StakedSettings>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorDeserialize, AnchorSerialize, Default)]
pub struct StakedSettingsConfig {
    pub oracle: Pubkey,

    pub asset_weight_init: WrappedI80F48,
    pub asset_weight_maint: WrappedI80F48,

    pub deposit_limit: u64,
    pub total_asset_value_init_limit: u64,

    pub oracle_max_age: u16,
    /// WARN: You almost certainly want "Collateral", using Isolated risk tier makes the asset
    /// worthless as collateral, and is generally useful only when creating a staked collateral pool
    /// for rewards purposes only.
    pub risk_tier: RiskTier,
}
