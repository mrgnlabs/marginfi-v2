use crate::{
    constants::{LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED},
    prelude::MarginfiResult,
    state::{
        marginfi_account::{BankAccountWrapper, MarginfiAccount, RiskEngine, RiskRequirementType},
        marginfi_group::MarginfiGroup,
    },
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};

pub fn create(ctx: Context<InitializeMarginfiAccount>) -> MarginfiResult {
    let margin_account = &mut ctx.accounts.marginfi_account.load_init()?;
    let InitializeMarginfiAccount {
        signer,
        marginfi_group,
        ..
    } = ctx.accounts;

    margin_account.initialize(marginfi_group.key(), signer.key());

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeMarginfiAccount<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(zero)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

///
/// Deposit into a bank account
/// 1. Add collateral to the margin accounts lending account
///     - Create a bank account if it doesn't exist for the deposited asset
/// 2. Transfer collateral from signer to bank liquidity vault
pub fn bank_deposit(ctx: Context<LendingPoolDeposit>, amount: u64) -> MarginfiResult {
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
    #[account(
        mut,
        address = marginfi_account.load()?.group
    )]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
    #[account(
        mut,
        address = marginfi_account.load()?.owner,
    )]
    pub signer: Signer<'info>,
    pub asset_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint = asset_mint,
    )]
    pub signer_token_account: Account<'info, TokenAccount>,
    /// TODO: Store bump on-chain
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_SEED,
            asset_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump,
    )]
    pub bank_liquidity_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

/// Withdraw from a bank account, if the user has deposits in the bank account withdraw those,
/// otherwise borrow from the bank if the user has sufficient collateral.
///
/// 1. Remove collateral from the margin accounts lending account
///     - Create a bank account if it doesn't exist for the deposited asset
/// 2. Transfer collateral from bank liquidity vault to signer
/// 3. Verify that the users account is in a healthy state
pub fn bank_withdraw(ctx: Context<LendingPoolWithdraw>, amount: u64) -> MarginfiResult {
    let LendingPoolWithdraw {
        marginfi_group: marginfi_group_loader,
        marginfi_account,
        signer,
        asset_mint,
        signer_token_account,
        bank_liquidity_vault,
        token_program,
        ..
    } = ctx.accounts;

    let mut marginfi_group = marginfi_group_loader.load_mut()?;
    let mut marginfi_account = marginfi_account.load_mut()?;

    let mut bank_account = BankAccountWrapper::find_by_mint_or_create(
        asset_mint.key(),
        &mut marginfi_group.lending_pool,
        &mut marginfi_account.lending_account,
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
        &[&[
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_ref(),
            asset_mint.key().as_ref(),
            marginfi_group_loader.key().as_ref(),
            &[*ctx.bumps.get("bank_liquidity_vault_authority").unwrap()],
        ]],
    )?;

    // // Check account health, if below threshold fail transaction
    // // Assuming `ctx.remaining_accounts` holds only oracle accounts
    let risk_engine = RiskEngine::new(&marginfi_group, &marginfi_account, ctx.remaining_accounts)?;
    risk_engine.check_account_health(RiskRequirementType::Initial)?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolWithdraw<'info> {
    #[account(
        mut,
        address = marginfi_account.load()?.group
    )]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
    #[account(
        mut,
        address = marginfi_account.load()?.owner,
    )]
    pub signer: Signer<'info>,
    pub asset_mint: Account<'info, Mint>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED,
            asset_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub bank_liquidity_vault_authority: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_SEED,
            asset_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump,
    )]
    pub bank_liquidity_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
