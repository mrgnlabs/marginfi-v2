use anchor_lang::prelude::*;
use solana_program::{
    program::invoke_signed,
    stake::state::{Authorized, Lockup},
    system_instruction,
};

use crate::{constants::{STAKEHOLDER_SEED, STAKEHOLDER_STAKE_ACC_SEED}, state::StakeHolder};

#[derive(Accounts)]
pub struct InitStakeHolder<'info> {
    /// Pays the account initialization fee
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: becomes the admin of the new account, unchecked
    pub admin: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [
            STAKEHOLDER_SEED.as_bytes(),
            vote_account.key().as_ref(),
            admin.key().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + StakeHolder::LEN,
    )]
    pub stakeholder: AccountLoader<'info, StakeHolder>,

    /// CHECK: used by CPI
    pub vote_account: UncheckedAccount<'info>,

    /// CHECK: Stakeholder's stake account to be created, validated against seeds
    #[account(
        mut,
        seeds = [
            STAKEHOLDER_STAKE_ACC_SEED.as_bytes(), 
            stakeholder.key().as_ref()
        ],
        bump,
    )]
    pub stake_account: UncheckedAccount<'info>,

    /// CHECK: Native stake program, checked against known hardcoded key
    #[account(
        constraint = stake_program.key() == solana_program::stake::program::ID
    )]
    pub stake_program: UncheckedAccount<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

pub fn init_stakeholder(ctx: Context<InitStakeHolder>) -> Result<()> {
    let mut stakeholder = ctx.accounts.stakeholder.load_init()?;

    stakeholder.key = ctx.accounts.stakeholder.key();
    stakeholder.admin = ctx.accounts.admin.key();
    stakeholder.vote_account = ctx.accounts.vote_account.key();
    stakeholder.stake_account = ctx.accounts.stake_account.key();

    stakeholder.net_delegation = 0;

    // Create the stake account owned by the Stakeholder PDA
    let space = solana_program::stake::state::StakeStateV2::size_of();
    let rent_exempt_lamports = ctx.accounts.rent.minimum_balance(space);

    let create_stake_account_ix = system_instruction::create_account(
        &ctx.accounts.payer.key(),
        &ctx.accounts.stake_account.key(),
        rent_exempt_lamports,
        space as u64,
        &ctx.accounts.stake_program.key(),
    );

    invoke_signed(
        &create_stake_account_ix,
        &[
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.stake_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[&[
            STAKEHOLDER_STAKE_ACC_SEED.as_bytes(),
            ctx.accounts.stakeholder.key().as_ref(),
            &[ctx.bumps.stake_account],
        ]],
    )?;

    // Initialize the stake account for the Stakeholder PDA
    let authorized = Authorized {
        staker: ctx.accounts.stakeholder.key(),
        withdrawer: ctx.accounts.stakeholder.key(),
    };
    let lockup = Lockup::default();

    let init_stake_account_ix = solana_program::stake::instruction::initialize(
        &ctx.accounts.stake_account.key(),
        &authorized,
        &lockup,
    );

    invoke_signed(
        &init_stake_account_ix,
        &[
            ctx.accounts.stake_account.to_account_info(),
            ctx.accounts.stake_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ],
        &[&[
            STAKEHOLDER_STAKE_ACC_SEED.as_bytes(),
            ctx.accounts.stakeholder.key().as_ref(),
            &[ctx.bumps.stake_account],
        ]],
    )?;

    Ok(())
}
