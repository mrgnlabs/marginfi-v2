use crate::constants::ASSET_TAG_STAKED;
// Permissionless ix to propagate a group's staked collateral settings to any bank in that group
use crate::state::marginfi_group::Bank;
use crate::state::staked_settings::StakedSettings;
use crate::MarginfiGroup;
use anchor_lang::prelude::*;

pub fn propagate_staked_settings(ctx: Context<PropagateStakedSettings>) -> Result<()> {
    let settings = ctx.accounts.staked_settings.load()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let (oracle_before, oracle_after) = (bank.config.oracle_keys[0], settings.oracle);
    let (age_before, age_after) = (bank.config.oracle_max_age, settings.oracle_max_age);

    bank.config.oracle_keys[0] = settings.oracle;
    bank.config.asset_weight_init = settings.asset_weight_init;
    bank.config.asset_weight_maint = settings.asset_weight_maint;
    bank.config.deposit_limit = settings.deposit_limit;
    bank.config.total_asset_value_init_limit = settings.total_asset_value_init_limit;
    bank.config.oracle_max_age = settings.oracle_max_age;
    bank.config.risk_tier = settings.risk_tier;

    // Only validate the oracle info if it has changed
    if oracle_before != oracle_after {
        bank.config
            .validate_oracle_setup(ctx.remaining_accounts, None, None, None)?;
    }
    if age_before != age_after {
        bank.config.validate_oracle_age()?;
    }

    bank.config.validate()?;

    Ok(())
}

#[derive(Accounts)]
pub struct PropagateStakedSettings<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        has_one = marginfi_group
    )]
    pub staked_settings: AccountLoader<'info, StakedSettings>,

    #[account(
        mut,
        constraint = {
            let bank = bank.load()?;
            bank.group == marginfi_group.key() &&
                bank.config.asset_tag == ASSET_TAG_STAKED
        }
    )]
    pub bank: AccountLoader<'info, Bank>,
}
