use crate::constants::{
    MARGINFI_SPONSORED_SHARD_ID, PYTH_PUSH_MIGRATED, PYTH_SPONSORED_SHARD_ID,
};
use crate::state::marginfi_group::{Bank, MarginfiGroup};
use crate::state::price::{OracleSetup, PythPushOraclePriceFeed};
use crate::{MarginfiError, MarginfiResult};
use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::FeedId;

pub fn migrate_pyth_push_oracle(ctx: Context<MigratePythPushOracle>) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    // TODO support staked collateral maybe? Probably doesn't matter.
    // TODO support both feeds maybe?
    //
    if bank.config.is_pyth_push_migrated() {
        msg!("bank already migrated");
        return Ok(());
    }
    require!(
        bank.config.oracle_setup == OracleSetup::PythPushOracle,
        MarginfiError::InvalidOracleSetup
    );

    let feed_id: FeedId = bank.config.oracle_keys[0].to_bytes();

    let (pyth_feed, _) =
        PythPushOraclePriceFeed::find_oracle_address(PYTH_SPONSORED_SHARD_ID, &feed_id);
    let (mrgn_feed, _) =
        PythPushOraclePriceFeed::find_oracle_address(MARGINFI_SPONSORED_SHARD_ID, &feed_id);

    let oracle_key = ctx.accounts.oracle.key();
    require!(
        oracle_key == pyth_feed || oracle_key == mrgn_feed,
        MarginfiError::WrongOracleAccountKeys
    );

    PythPushOraclePriceFeed::check_ai_and_feed_id(&ctx.accounts.oracle, &feed_id)?;

    bank.config.oracle_keys[0] = oracle_key;
    bank.config
        .update_config_flag(true, PYTH_PUSH_MIGRATED);

    Ok(())
}

#[derive(Accounts)]
pub struct MigratePythPushOracle<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut,
        has_one = group
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// Must use the Pyth Sponsored shard ID (0) or mrgn's (3301)
    ///
    /// CHECK: validated via program logic
    pub oracle: AccountInfo<'info>,
}
