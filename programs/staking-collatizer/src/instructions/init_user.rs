use anchor_lang::prelude::*;

use crate::{constants::STAKE_USER_SEED, state::StakeUser};

#[derive(Accounts)]
pub struct InitUser<'info> {
    /// Pays the account initialization fee
    #[account(mut)]
    pub payer: Signer<'info>,

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
    msg!(
        "Nothing was done. Signed by: {:?}",
        ctx.accounts.payer.key()
    );
    Ok(())
}
