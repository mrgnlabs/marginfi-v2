use crate::{
    prelude::MarginfiResult,
    state::{marginfi_account::MarginfiAccount, marginfi_group::MarginfiGroup},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};

pub fn create(ctx: Context<CreateMarginfiAccount>) -> MarginfiResult {
    let margin_account = &mut ctx.accounts.marginfi_account.load_init()?;
    let CreateMarginfiAccount {
        signer,
        marginfi_group,
        ..
    } = ctx.accounts;

    margin_account.initialize(marginfi_group.key(), signer.key());

    Ok(())
}

#[derive(Accounts)]
pub struct CreateMarginfiAccount<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(zero)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn lending_pool_deposit(
    ctx: Context<LendingPoolDeposit>,
    bank_index: u8,
    amount: u64,
) -> MarginfiResult {
    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolDeposit<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub asset_mint: Account<'info, Mint>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub bank_liquidity_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
