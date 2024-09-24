// Permissionless ix to order a bank with ASSET_TAG_STAKED to cache the current exchange rate of
// SOL:LST (sol_appreciation_rate) on the active spl-single-pool

use crate::{constants::ASSET_TAG_STAKED, state::marginfi_group::Bank, MarginfiResult};
use anchor_lang::prelude::*;
use fixed::types::I80F48;

#[derive(Accounts)]
pub struct CacheSolExRate<'info> {
    #[account(mut)]
    pub bank: AccountLoader<'info, Bank>,
}

pub fn cache_sol_ex_rate(ctx: Context<CacheSolExRate>) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    // This ix does not apply to non-staked assets, set the default value and exit
    if bank.config.asset_tag != ASSET_TAG_STAKED {
        bank.sol_appreciation_rate = I80F48::ONE.into();
        return Ok(());
    }

    Ok(())
}
