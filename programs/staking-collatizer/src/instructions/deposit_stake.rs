use anchor_lang::prelude::*;
use solana_program::{
    program::invoke_signed,
    stake::state::{Authorized, Lockup, StakeAuthorize},
    system_instruction,
};

use crate::{constants::{STAKEHOLDER_SEED, STAKEHOLDER_STAKE_ACC_SEED}, state::StakeHolder};

#[derive(Accounts)]
pub struct DepositStake<'info> {

    #[account(mut)]
    pub admin: Signer<'info>,

    /// The `user_stake_account`'s authority must also sign. This supports use cases where the admin
    /// is moving stake from another wallet they control.
    pub stake_authority: Signer<'info>,

    // TODO add user accounts (stakeUser, etc)

    // TODO check admin, stake acc, etc
    #[account(
        mut
    )]
    pub stakeholder: AccountLoader<'info, StakeHolder>,

    // /// CHECK: User's stake account (active and delegated to a validator), used by cpi
    // #[account(mut)]
    // pub user_stake_account: UncheckedAccount<'info>,

    // /// CHECK: Stakeholder's stake account, validated against seeds
    // #[account(
    //     mut,
    //     seeds = [
    //         STAKEHOLDER_STAKE_ACC_SEED.as_bytes(), 
    //         stakeholder.key().as_ref()
    //     ],
    //     bump,
    // )]
    // pub stake_account: UncheckedAccount<'info>,

    /// CHECK: Native stake program, checked against known hardcoded key
    #[account(
        constraint = stake_program.key() == solana_program::stake::program::ID
    )]
    pub stake_program: UncheckedAccount<'info>,
    /// Sysvar required by the Solana staking program
    pub clock: Sysvar<'info, Clock>,
    pub system_program: Program<'info, System>,
}

pub fn deposit_stake(ctx: Context<DepositStake>) -> Result<()> {
    msg!("Start deposit");
    // let mut stakeholder = ctx.accounts.stakeholder.load_mut()?;

    // let authorize_ix = solana_program::stake::instruction::authorize(
    //     &ctx.accounts.user_stake_account.key(),   // User's stake account
    //     &ctx.accounts.stake_authority.key(), // Current authorized staker
    //     &ctx.accounts.stakeholder.key(), // Program stakeholder becomes the new staker
    //     StakeAuthorize::Staker,
    //     None
    // );

    // invoke_signed(
    //     &authorize_ix,
    //     &[
    //         ctx.accounts.user_stake_account.to_account_info(),
    //         ctx.accounts.clock.to_account_info(),
    //         ctx.accounts.stake_authority.to_account_info(),
    //         ctx.accounts.stake_program.to_account_info(),
    //     ],
    //     &[],
    // )?;

    msg!("Done transfer authority");

    Ok(())
}
