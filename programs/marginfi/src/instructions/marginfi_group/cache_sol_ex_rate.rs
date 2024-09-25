// Permissionless ix to order a bank with ASSET_TAG_STAKED to cache the current exchange rate of
// SOL:LST (sol_appreciation_rate) on the active spl-single-pool

use crate::{
    check,
    constants::{ASSET_TAG_STAKED, SPL_SINGLE_POOL_ID},
    state::marginfi_group::Bank,
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::*;
use fixed::types::I80F48;

#[derive(Accounts)]
pub struct CacheSolExRate<'info> {
    #[account(mut)]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        constraint = lst_mint.key() == bank.load()?.mint @ MarginfiError::StakePoolValidationFailed
    )]
    pub lst_mint: Box<InterfaceAccount<'info, Mint>>,
    /// CHECK: Validated using `stake_pool`
    pub sol_pool: AccountInfo<'info>,

    /// CHECK: We validate this is correct backwards, by deriving the PDA of the `lst_mint` using
    /// this key. Since the mint is already checked against the known value on the Bank, if it
    /// derives the same `lst_mint`, then this must be the correct pool, and we can subsequently use
    /// it to validate the `sol_pool`
    pub stake_pool: AccountInfo<'info>,
}

pub fn cache_sol_ex_rate(ctx: Context<CacheSolExRate>) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;
    let stake_pool_bytes = &ctx.accounts.stake_pool.key().to_bytes();

    // This ix does not apply to non-staked assets, set the default value and exit
    if bank.config.asset_tag != ASSET_TAG_STAKED {
        bank.sol_appreciation_rate = I80F48::ONE.into();
        msg!("Wrong asset type flagged, resetting to default value and aborting");
        return Ok(());
    }

    let program_id = &SPL_SINGLE_POOL_ID;
    // Validate the given stake_pool derives the same lst_mint, proving stake_pool is correct
    let (exp_mint, _) = Pubkey::find_program_address(&[b"mint", stake_pool_bytes], program_id);
    check!(
        exp_mint == ctx.accounts.lst_mint.key(),
        MarginfiError::StakePoolValidationFailed
    );

    // Validate the now-proven stake_pool derives the given sol_pool
    let (exp_pool, _) = Pubkey::find_program_address(&[b"stake", stake_pool_bytes], program_id);
    check!(
        exp_pool == ctx.accounts.sol_pool.key(),
        MarginfiError::StakePoolValidationFailed
    );

    // Note: LST mint and SOL use the same decimals, so decimals do not need to be considered
    let lst_supply: I80F48 = I80F48::from(ctx.accounts.lst_mint.supply);
    // Handle the edge case when the supply is zero
    if lst_supply == I80F48::ZERO {
        bank.sol_appreciation_rate = I80F48::ONE.into();
        return Ok(());
    }
    let sol_pool_balance: I80F48 = I80F48::from(ctx.accounts.sol_pool.lamports());

    let sol_lst_exchange_rate: I80F48 = sol_pool_balance / lst_supply;
    // Sanity check the exchange rate
    if sol_lst_exchange_rate < I80F48::ONE {
        panic!("invalid exchange rate or slashing now exists");
    }
    bank.sol_appreciation_rate = sol_lst_exchange_rate.into();

    Ok(())
}
