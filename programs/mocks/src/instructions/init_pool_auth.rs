use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::state::PoolAuth;

#[derive(Accounts)]
#[instruction(
    nonce: u16,
)]
pub struct InitPoolAuth<'info> {
    /// Pays the init fee
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        seeds = [
          &nonce.to_le_bytes(),
          b"pool_auth".as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + PoolAuth::LEN,
    )]
    pub pool_auth: Account<'info, PoolAuth>,

    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        init,
        seeds = [
            mint_a.key().as_ref(),
            pool_auth.key().as_ref(),
            b"pools",
        ],
        bump,
        token::mint = mint_a,
        token::authority = pool_auth,
        payer = payer
    )]
    pub pool_a: Account<'info, TokenAccount>,

    #[account(
        init,
        seeds = [
            mint_b.key().as_ref(),
            pool_auth.key().as_ref(),
            b"pools",
        ],
        bump,
        token::mint = mint_b,
        token::authority = pool_auth,
        payer = payer
    )]
    pub pool_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

pub fn init_pool_auth(ctx: Context<InitPoolAuth>, nonce: u16) -> Result<()> {
    let pool_auth = &mut ctx.accounts.pool_auth;
    pool_auth.nonce = nonce;
    pool_auth.pool_a = ctx.accounts.pool_a.key();
    pool_auth.pool_b = ctx.accounts.pool_b.key();
    pool_auth.bump_seed = ctx.bumps.pool_auth;

    Ok(())
}
