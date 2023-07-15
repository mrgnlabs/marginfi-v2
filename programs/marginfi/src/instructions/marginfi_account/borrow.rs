use crate::{
    bank_signer, check,
    constants::{LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED},
    events::{AccountEventHeader, LendingAccountBorrowEvent},
    prelude::{MarginfiError, MarginfiGroup, MarginfiResult},
    state::{
        marginfi_account::{BankAccountWrapper, MarginfiAccount, RiskEngine, DISABLED_FLAG},
        marginfi_group::{Bank, BankVaultType},
    },
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer};
use fixed::types::I80F48;
use solana_program::{clock::Clock, sysvar::Sysvar};

/// 1. Accrue interest
/// 2. Create the user's bank account for the asset borrowed if it does not exist yet
/// 3. Record liability increase in the bank account
/// 4. Transfer funds from the bank's liquidity vault to the signer's token account
/// 5. Verify that the user account is in a healthy state
///
/// Will error if there is an existing asset <=> withdrawing is not allowed.
pub fn lending_account_borrow(ctx: Context<LendingAccountBorrow>, amount: u64) -> MarginfiResult {
    let LendingAccountBorrow {
        marginfi_account: marginfi_account_loader,
        destination_token_account,
        bank_liquidity_vault,
        token_program,
        bank_liquidity_vault_authority,
        bank: bank_loader,
        ..
    } = ctx.accounts;

    let mut marginfi_account = marginfi_account_loader.load_mut()?;

    check!(
        !marginfi_account.get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

    bank_loader.load_mut()?.accrue_interest(
        Clock::get()?.unix_timestamp,
        #[cfg(not(feature = "client"))]
        bank_loader.key(),
    )?;

    {
        let mut bank = bank_loader.load_mut()?;
        let liquidity_vault_authority_bump = bank.liquidity_vault_authority_bump;

        let mut bank_account = BankAccountWrapper::find_or_create(
            &bank_loader.key(),
            &mut bank,
            &mut marginfi_account.lending_account,
        )?;

        bank_account.borrow(I80F48::from_num(amount))?;
        bank_account.withdraw_spl_transfer(
            amount,
            Transfer {
                from: bank_liquidity_vault.to_account_info(),
                to: destination_token_account.to_account_info(),
                authority: bank_liquidity_vault_authority.to_account_info(),
            },
            token_program.to_account_info(),
            bank_signer!(
                BankVaultType::Liquidity,
                bank_loader.key(),
                liquidity_vault_authority_bump
            ),
        )?;

        emit!(LendingAccountBorrowEvent {
            header: AccountEventHeader {
                signer: Some(ctx.accounts.signer.key()),
                marginfi_account: marginfi_account_loader.key(),
                marginfi_account_authority: marginfi_account.authority,
                marginfi_group: marginfi_account.group,
            },
            bank: bank_loader.key(),
            mint: bank.mint,
            amount,
        });
    }

    // Check account health, if below threshold fail transaction
    // Assuming `ctx.remaining_accounts` holds only oracle accounts
    RiskEngine::check_account_init_health(&marginfi_account, ctx.remaining_accounts)?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountBorrow<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = marginfi_account.load() ?.group == marginfi_group.key(),
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        address = marginfi_account.load() ?.authority,
    )]
    pub signer: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load() ?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub destination_token_account: Account<'info, TokenAccount>,

    /// CHECK: Seed constraint check
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load() ?.liquidity_vault_authority_bump,
    )]
    pub bank_liquidity_vault_authority: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load() ?.liquidity_vault_bump,
    )]
    pub bank_liquidity_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}
