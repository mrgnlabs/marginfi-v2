use crate::{
    prelude::MarginfiResult,
    state::{
        marginfi_account::{BankAccountWrapper, MarginfiAccount, RiskEngine, RiskRequirementType},
        marginfi_group::MarginfiGroup,
    },
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};

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

///
/// Deposit into a bank
/// 1. Add collateral to the margin accounts lending account
/// 2. Transfer collateral from signer to bank liquidity vault
pub fn lending_pool_deposit(ctx: Context<LendingPoolDeposit>, amount: u64) -> MarginfiResult {
    let LendingPoolDeposit {
        marginfi_group,
        marginfi_account,
        signer,
        asset_mint,
        signer_token_account,
        bank_liquidity_vault,
        token_program,
    } = ctx.accounts;

    let mut marginfi_group = marginfi_group.load_mut()?;
    let mut marginfi_account = marginfi_account.load_mut()?;

    let mut bank_account = BankAccountWrapper::find_by_mint_or_create(
        asset_mint.key(),
        &mut marginfi_group.lending_pool,
        &mut marginfi_account.lending_account,
    )?;

    bank_account.deposit(amount)?;
    bank_account.deposit_transfer(
        amount,
        Transfer {
            from: signer_token_account.to_account_info(),
            to: bank_liquidity_vault.to_account_info(),
            authority: signer.to_account_info(),
        },
        token_program.to_account_info(),
    )?;

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

pub fn lending_pool_withdraw(ctx: Context<LendingPoolWithdraw>, amount: u64) -> MarginfiResult {
    let LendingPoolWithdraw {
        marginfi_group,
        marginfi_account,
        signer,
        asset_mint,
        signer_token_account,
        bank_liquidity_vault,
        token_program,
    } = ctx.accounts;

    let mut marginfi_group = marginfi_group.load_mut()?;
    let mut marginfi_account = marginfi_account.load_mut()?;

    let lending_pool = &mut marginfi_group.lending_pool;
    let lending_account = &mut marginfi_account.lending_account;

    let mut bank_account = BankAccountWrapper::find_by_mint_or_create(
        asset_mint.key(),
        lending_pool,
        lending_account,
    )?;

    bank_account.withdraw(amount)?;
    bank_account.withdraw_transfer(
        amount,
        Transfer {
            from: bank_liquidity_vault.to_account_info(),
            to: signer_token_account.to_account_info(),
            authority: signer.to_account_info(),
        },
        token_program.to_account_info(),
    )?;

    // // Check account health, if below threshold fail transaction
    // // Assuming `ctx.remaining_accounts` holds only oracle accounts
    let risk_engine = RiskEngine::new(&marginfi_group, &marginfi_account, ctx.remaining_accounts)?;
    risk_engine.check_account_health(RiskRequirementType::Initial)?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolWithdraw<'info> {
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
