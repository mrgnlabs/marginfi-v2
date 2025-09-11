// Permissionless ix to propagate a group's staked collateral settings to any bank in that group.
// Also perform pyth pull oracle migration for 0.1.4
use crate::constants::{ASSET_TAG_STAKED, PYTH_PUSH_MIGRATED, PYTH_SPONSORED_SHARD_ID};
use crate::state::marginfi_group::Bank;
use crate::state::price::PythPushOraclePriceFeed;
use crate::state::staked_settings::StakedSettings;
use crate::{MarginfiError, MarginfiGroup};
use anchor_lang::prelude::*;

pub fn propagate_staked_settings(ctx: Context<PropagateStakedSettings>) -> Result<()> {
    let settings = ctx.accounts.staked_settings.load()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let (oracle_before, oracle_after) = (bank.config.oracle_keys[0], settings.oracle);
    let (age_before, age_after) = (bank.config.oracle_max_age, settings.oracle_max_age);

    // Note: Initial execution of this ix completes migration. After that, propagation will fail if
    // this is still a feed_id instead of a feed key. Next time you want to edit staked settings
    // after the release of 0.1.4, remember to change this into a feed.
    bank.config.oracle_keys[0] = settings.oracle;

    // TODO remove in 0.1.5
    // * Only used for 0.1.3 -> 0.1.4 oracle migration
    if !bank.config.is_pyth_push_migrated() {
        let feed_id = bank.config.oracle_keys[0];
        let (pyth_feed, _) = PythPushOraclePriceFeed::find_oracle_address(
            PYTH_SPONSORED_SHARD_ID,
            &feed_id.to_bytes(),
        );
        // Note: this overrides `bank.config.oracle_keys[0] = settings.oracle;`, but only once (to
        // complete migration). After that, this ix will fail unless settings.oracle is updated to a
        // feed instead of a feed id.
        bank.config.oracle_keys[0] = pyth_feed;
        // TODO staked banks will always set this flag in 0.1.5
        bank.config.update_config_flag(true, PYTH_PUSH_MIGRATED);
    }

    bank.config.asset_weight_init = settings.asset_weight_init;
    bank.config.asset_weight_maint = settings.asset_weight_maint;
    bank.config.deposit_limit = settings.deposit_limit;
    bank.config.total_asset_value_init_limit = settings.total_asset_value_init_limit;
    bank.config.oracle_max_age = settings.oracle_max_age;
    bank.config.risk_tier = settings.risk_tier;

    // Only validate the oracle info if it has changed (this includes 0.1.3 -> 0.1.4 oracle migration)
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
        has_one = marginfi_group @ MarginfiError::InvalidMarginfigroupConstraint
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
