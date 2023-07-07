use anchor_lang::{prelude::*, Accounts, ToAccountInfo};
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

use crate::{
    check,
    constants::{EMISSIONS_AUTH_SEED, EMISSIONS_TOKEN_ACCOUNT_SEED},
    prelude::{MarginfiError, MarginfiResult},
    state::{
        marginfi_account::{BankAccountWrapper, MarginfiAccount, DISABLED_FLAG},
        marginfi_group::{Bank, MarginfiGroup},
    },
};

pub fn lending_account_withdraw_emissions(
    ctx: Context<LendingAccountWithdrawEmissions>,
) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;

    check!(
        !marginfi_account.get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

    let mut bank = ctx.accounts.bank.load_mut()?;

    let mut balance = BankAccountWrapper::find(
        ctx.accounts.bank.to_account_info().key,
        &mut bank,
        &mut marginfi_account.lending_account,
    )?;

    // Settle emissions
    let emissions_settle_amount = balance.settle_emissions_and_get_transfer_amount()?;

    if emissions_settle_amount > 0 {
        let signer_seeds: &[&[&[u8]]] = &[&[
            EMISSIONS_AUTH_SEED.as_bytes(),
            &ctx.accounts.bank.key().to_bytes(),
            &ctx.accounts.emissions_mint.key().to_bytes(),
            &[*ctx.bumps.get("emissions_auth").unwrap()],
        ]];

        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.emissions_vault.to_account_info(),
                    to: ctx.accounts.destination_account.to_account_info(),
                    authority: ctx.accounts.emissions_auth.to_account_info(),
                },
                signer_seeds,
            ),
            emissions_settle_amount,
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountWithdrawEmissions<'info> {
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
            EMISSIONS_AUTH_SEED.as_bytes(),
            bank.key().as_ref(),
            emissions_mint.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: Asserted by PDA
    pub emissions_auth: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            EMISSIONS_TOKEN_ACCOUNT_SEED.as_bytes(),
            bank.key().as_ref(),
            emissions_mint.key().as_ref(),
        ],
        bump,
    )]
    pub emissions_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub destination_account: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

/// Permissionlessly settle unclaimed emissions to a users account.
pub fn lending_account_settle_emissions(
    ctx: Context<LendingAccountSettleEmissions>,
) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let mut balance = BankAccountWrapper::find(
        ctx.accounts.bank.to_account_info().key,
        &mut bank,
        &mut marginfi_account.lending_account,
    )?;

    balance.claim_emissions(Clock::get()?.unix_timestamp.try_into().unwrap())?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountSettleEmissions<'info> {
    #[account(
        mut,
        constraint = marginfi_account.load()?.group == bank.load()?.group,
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub bank: AccountLoader<'info, Bank>,
}
