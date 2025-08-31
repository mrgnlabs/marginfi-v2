// Permissionless ix to migrate a Pyth-pull based bank created in 0.1.3 or earlier to the new oracle
// model in 0.1.4, where a feed is stored directly instead of a feed id.

// TODO remove in 0.1.5
use crate::constants::{MARGINFI_SPONSORED_SHARD_ID, PYTH_PUSH_MIGRATED, PYTH_SPONSORED_SHARD_ID};
use crate::state::marginfi_group::{Bank, MarginfiGroup};
use crate::state::price::{OracleSetup, PythPushOraclePriceFeed};
use crate::{live, MarginfiError, MarginfiResult};
use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::FeedId;

pub fn migrate_pyth_push_oracle(ctx: Context<MigratePythPushOracle>) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    if bank.config.is_pyth_push_migrated() {
        msg!("bank {:?} already migrated", &ctx.accounts.bank.key());
        return Ok(());
    }
    // Staked collateral banks should use propagate instead.
    if bank.config.oracle_setup == OracleSetup::StakedWithPythPush {
        msg!(
            "use propagate to migrate staked banks. Did nothing for bank: {:?}",
            &ctx.accounts.bank.key()
        );
        return Ok(());
    }
    // All other Non-Pyth banks are unaffected, set the flag and exit, the flag does nothing for
    // these banks.
    if bank.config.oracle_setup != OracleSetup::PythPushOracle {
        bank.config.update_config_flag(true, PYTH_PUSH_MIGRATED);
        msg!(
            "bank {:?} does not use Pyth, doing nothing",
            &ctx.accounts.bank.key()
        );
        return Ok(());
    }

    let feed_id_before = bank.config.oracle_keys[0];
    let feed_id_bytes: FeedId = bank.config.oracle_keys[0].to_bytes();
    let (pyth_feed, _) =
        PythPushOraclePriceFeed::find_oracle_address(PYTH_SPONSORED_SHARD_ID, &feed_id_bytes);
    let (mrgn_feed, _) =
        PythPushOraclePriceFeed::find_oracle_address(MARGINFI_SPONSORED_SHARD_ID, &feed_id_bytes);
    let oracle_key = ctx.accounts.oracle.key();

    // On localnet, the feed id is randomly generated, so this check is not applicable.
    if live!() {
        require!(
            oracle_key == pyth_feed || oracle_key == mrgn_feed,
            MarginfiError::WrongOracleAccountKeys
        );
    }

    // Validate that the feed actually exists and is valid, a useful sanity check because most of
    // our banks have a sponsored feed or a mrgn feed but not both. This also repeats the above
    // check, since we are validating the feed bytes again.
    PythPushOraclePriceFeed::check_ai_and_feed_id(&ctx.accounts.oracle, &feed_id_bytes)?;

    bank.config.oracle_keys[0] = oracle_key;
    bank.config.update_config_flag(true, PYTH_PUSH_MIGRATED);

    msg!(
        "migrated bank {:?} to oracle {:?} was {:?}",
        &ctx.accounts.bank.key(),
        &ctx.accounts.oracle.key(),
        &feed_id_before
    );

    Ok(())
}

#[derive(Accounts)]
pub struct MigratePythPushOracle<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut,
        has_one = group @ MarginfiError::InvalidGroupConstraint
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// Must use the Pyth Sponsored shard ID (0) or mrgn's (3301)
    ///
    /// CHECK: validated via program logic
    pub oracle: AccountInfo<'info>,
}
