use anchor_lang::{prelude::*, Accounts, ToAccountInfo};
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};
use fixed::types::I80F48;

use crate::{
    constants::{EMISSIONS_AUTH_SEED, EMISSIONS_TOKEN_ACCOUNT_SEED},
    prelude::MarginfiResult,
    state::{
        marginfi_account::{BankAccountWrapper, MarginfiAccount},
        marginfi_group::{Bank, MarginfiGroup},
    },
};

pub fn withdraw_emissions(ctx: Context<WithdrawEmissions>) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let balance = BankAccountWrapper::find(
        ctx.accounts.bank.to_account_info().key,
        &mut bank,
        &mut marginfi_account.lending_account,
    )?;

    let outstanding_emissions_floored = I80F48::from(balance.balance.emissions_outstanding).floor();

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.emissions_vault.to_account_info(),
                to: ctx.accounts.destination_account.to_account_info(),
                authority: ctx.accounts.emissions_auth.to_account_info(),
            },
            &[],
        ),
        outstanding_emissions_floored.to_num::<u64>(),
    )?;

    balance.balance.emissions_outstanding =
        { I80F48::from(balance.balance.emissions_outstanding) - outstanding_emissions_floored }
            .into();

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawEmissions<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = marginfi_account.load()?.group == marginfi_group.key(),
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        address = marginfi_account.load()?.authority,
    )]
    pub signer: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        address = bank.load()?.emissions_mint
    )]
    pub emissions_mint: Account<'info, Mint>,

    #[account(
        seeds = [
            EMISSIONS_TOKEN_ACCOUNT_SEED.as_bytes(),
            bank.key().as_ref(),
            emissions_mint.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: Asserted by PDA
    pub emissions_auth: AccountInfo<'info>,

    #[account(
        seeds = [
            EMISSIONS_AUTH_SEED.as_bytes(),
            bank.key().as_ref(),
            emissions_mint.key().as_ref(),
        ],
        bump,
    )]
    pub emissions_vault: Box<Account<'info, TokenAccount>>,

    pub destination_account: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}
