use anchor_lang::prelude::*;

use crate::{constants::STAKE_USER_SEED, state::StakeUser};

#[derive(Accounts)]
pub struct InitUser<'info> {
    /// Pays the account initialization fee
    #[account(mut)]
    pub payer: Signer<'info>,

    // TODO owner seperate from payer

    #[account(
        init,
        seeds = [
            STAKE_USER_SEED.as_bytes(),
            payer.key().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + StakeUser::LEN,
    )]
    pub stake_user: AccountLoader<'info, StakeUser>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

pub fn init_user(ctx: Context<InitUser>) -> Result<()> {
    let mut stake_user = ctx.accounts.stake_user.load_init()?;

    stake_user.key = ctx.accounts.stake_user.key();

    Ok(())
}
