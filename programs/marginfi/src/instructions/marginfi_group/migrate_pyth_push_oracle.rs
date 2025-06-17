use crate::constants::{
    PYTH_PUSH_MARGINFI_SPONSORED_SHARD_ID, PYTH_PUSH_MIGRATED_FLAG,
    PYTH_PUSH_PYTH_SPONSORED_SHARD_ID,
};
use crate::state::marginfi_group::{Bank, MarginfiGroup};
use crate::state::price::PythPushOraclePriceFeed;
use crate::{MarginfiError, MarginfiResult};
use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::FeedId;

pub fn migrate_pyth_push_oracle(ctx: Context<MigratePythPushOracle>) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;
    if bank.config.is_pyth_push_migrated() {
        msg!("bank already migrated");
        return Ok(());
    }
    require!(
        bank.config.oracle_setup == crate::state::price::OracleSetup::PythPushOracle,
        MarginfiError::InvalidOracleSetup
    );

    let feed_id: FeedId = bank.config.oracle_keys[0].to_bytes();

    let expected_oracle_0 =
        PythPushOraclePriceFeed::find_oracle_address(PYTH_PUSH_PYTH_SPONSORED_SHARD_ID, &feed_id).0;
    let expected_oracle_1 = PythPushOraclePriceFeed::find_oracle_address(
        PYTH_PUSH_MARGINFI_SPONSORED_SHARD_ID,
        &feed_id,
    )
    .0;

    let oracle_key = ctx.accounts.oracle.key();
    require!(
        oracle_key == expected_oracle_0 || oracle_key == expected_oracle_1,
        MarginfiError::WrongOracleAccountKeys
    );

    PythPushOraclePriceFeed::check_ai_and_feed_id(&ctx.accounts.oracle, &feed_id)?;

    bank.config.oracle_keys[0] = oracle_key;
    bank.config
        .update_config_flag(true, PYTH_PUSH_MIGRATED_FLAG);

    Ok(())
}

#[derive(Accounts)]
pub struct MigratePythPushOracle<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(mut, has_one = group)]
    pub bank: AccountLoader<'info, Bank>,
    /// CHECK: validated via program logic
    pub oracle: AccountInfo<'info>,
}
