use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    number::Number,
    state::{Personal, State},
};

#[derive(Accounts)]
pub struct InitState<'info> {
    #[account(
        init,
        payer = payer,
        seeds = [b"state", admin.key().as_ref()],
        bump,
        space = 8 + 32 + 1 + 1 + 1 + Number::SIZEOF + 32 + 32,
    )]
    pub state: Account<'info, State>,

    #[account(
        init,
        payer = payer,
        seeds = [b"mint_sy", state.key().as_ref()],
        bump,
        mint::decimals = 9,
        mint::authority = state,
    )]
    pub mint_sy: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        seeds = [b"pool_sy", state.key().as_ref()],
        bump,
        token::mint = mint_sy,
        token::authority = state,
    )]
    pub pool_sy: Account<'info, TokenAccount>,

    pub admin: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SetRate<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct InitPersonal<'info> {
    #[account(init, payer = owner, space = 8 + 32 + 8)]
    pub personal: Account<'info, Personal>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetState<'info> {
    pub state: Account<'info, State>,
}

#[derive(Accounts)]
pub struct GetPosition<'info> {
    pub personal: Account<'info, Personal>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,
    #[account(mut, has_one = owner)]
    pub personal: Account<'info, Personal>,
    #[account(mut)]
    pub from_sy: Account<'info, TokenAccount>, // vault/market escrow
    #[account(mut, address = state.pool_sy)]
    pub pool_sy: Account<'info, TokenAccount>,
    /// CHECK: signer PDA supplied by caller; must own from_sy and pool_sy
    pub authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub owner: Signer<'info>, // personal owner (for has_one check)
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,
    #[account(mut, has_one = owner)]
    pub personal: Account<'info, Personal>,
    #[account(mut, address = state.pool_sy)]
    pub pool_sy: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to_sy: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimEmission<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,
}

#[derive(Accounts)]
pub struct MintSy<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,
    #[account(mut, address = state.mint_sy)]
    pub mint_sy: Account<'info, Mint>,
    #[account(mut)]
    pub dst_sy: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RedeemSy<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,
    #[account(mut, address = state.mint_sy)]
    pub mint_sy: Account<'info, Mint>,
    #[account(mut)]
    pub src_sy: Account<'info, TokenAccount>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}
