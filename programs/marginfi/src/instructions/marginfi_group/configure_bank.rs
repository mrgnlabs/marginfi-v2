use crate::check;
use crate::constants::{EMISSIONS_AUTH_SEED, EMISSIONS_TOKEN_ACCOUNT_SEED};
use crate::events::{GroupEventHeader, LendingPoolBankConfigureEvent};
use crate::prelude::MarginfiError;
use crate::{
    state::marginfi_group::{Bank, BankConfigOpt, MarginfiGroup},
    MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};
use fixed::types::I80F48;

pub fn lending_pool_configure_bank(
    ctx: Context<LendingPoolConfigureBank>,
    bank_config: BankConfigOpt,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    bank.configure(&bank_config)?;

    if bank_config.oracle.is_some() {
        bank.config.validate_oracle_setup(ctx.remaining_accounts)?;
    }

    emit!(LendingPoolBankConfigureEvent {
        header: GroupEventHeader {
            marginfi_group: ctx.accounts.marginfi_group.key(),
            signer: Some(*ctx.accounts.admin.key)
        },
        bank: ctx.accounts.bank.key(),
        mint: bank.mint,
        config: bank_config,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolConfigureBank<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,
}

pub fn lending_pool_setup_emissions(
    ctx: Context<LendingPoolSetupEmissions>,
    emissions_flags: u64,
    emissions_rate: u64,
    total_emissions: u64,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    check!(
        bank.emissions_mint.eq(&Pubkey::default()),
        MarginfiError::EmissionsAlreadySetup
    );

    bank.emissions_mint = ctx.accounts.emissions_mint.key();
    bank.emissions_flags = emissions_flags;
    bank.emissions_rate = emissions_rate;
    bank.emissions_remaining = I80F48::from_num(total_emissions).into();

    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.emissions_funding_account.to_account_info(),
                to: ctx.accounts.emissions_token_account.to_account_info(),
                authority: ctx.accounts.admin.to_account_info(),
            },
        ),
        total_emissions,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolSetupEmissions<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    pub emissions_mint: Account<'info, Mint>,

    #[account(
        seeds = [
            EMISSIONS_AUTH_SEED.as_bytes(),
            bank.key().as_ref(),
            emissions_mint.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: Asserted by PDA constraints
    pub emissions_auth: AccountInfo<'info>,

    #[account(
        init,
        payer = admin,
        token::mint = emissions_mint,
        token::authority = emissions_auth,
        seeds = [
            EMISSIONS_TOKEN_ACCOUNT_SEED.as_bytes(),
            bank.key().as_ref(),
            emissions_mint.key().as_ref(),
        ],
        bump,
    )]
    pub emissions_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: Account provided only for funding rewards
    #[account(mut)]
    pub emissions_funding_account: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
