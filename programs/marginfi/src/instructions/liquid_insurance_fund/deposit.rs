use crate::{
    check,
    constants::{INSURANCE_VAULT_SEED, LIQUID_INSURANCE_SEED, LIQUID_INSURANCE_USER_SEED},
    events::{LiquidInsuranceFundEventHeader, MarginfiDepositIntoLiquidInsuranceFundEvent},
    state::{
        liquid_insurance_fund::{LiquidInsuranceFund, LiquidInsuranceFundAccount},
        marginfi_group::Bank,
    },
    MarginfiError, MarginfiGroup, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer};
use fixed::types::I80F48;

#[derive(Accounts)]
pub struct DepositIntoLiquidInsuranceFund<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: Account to move tokens into
    #[account(mut)]
    pub signer_token_account: AccountInfo<'info>,

    #[account(
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// The corresponding insurance vault that the liquid insurance fund deposits into.
    /// This is the insurance vault of the underlying bank
    #[account(
        mut,
        seeds = [
            INSURANCE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_bump
    )]
    pub bank_insurance_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [
            LIQUID_INSURANCE_USER_SEED.as_bytes(),
            signer.key().as_ref(),
        ],
        bump
    )]
    pub user_insurance_fund_account: AccountLoader<'info, LiquidInsuranceFundAccount>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

pub fn deposit_into_liquid_insurance_fund(
    ctx: Context<DepositIntoLiquidInsuranceFund>,
    amount: u64,
) -> MarginfiResult {
    let DepositIntoLiquidInsuranceFund {
        marginfi_group: marginfi_group_loader,
        liquid_insurance_fund,
        signer,
        signer_token_account,
        bank,
        bank_insurance_vault,
        token_program,
        ..
    } = ctx.accounts;

    let mut liquid_insurance_fund = ctx.accounts.liquid_insurance_fund.load_mut()?;

    // Get amount inside the bank's insurance vault
    let total_bank_insurance_vault_amount = ctx.accounts.bank_insurance_vault.amount;

    // Derive shares from user deposit
    let user_shares = liquid_insurance_fund.get_shares(I80F48::from_num(amount))?;

    // update shares of the liquid insurance fund
    liquid_insurance_fund.deposit_shares(
        user_shares,
        I80F48::from_num(total_bank_insurance_vault_amount),
    )?;

    // Deposit user funds into the relevant insurance vault
    liquid_insurance_fund.deposit_spl_transfer(
        amount,
        Transfer {
            from: ctx.accounts.signer_token_account.to_account_info(),
            to: ctx.accounts.bank_insurance_vault.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        },
        token_program.to_account_info(),
    )?;

    let user_deposited_share_amount = user_shares
        .checked_to_num::<u64>()
        .ok_or(MarginfiError::MathError)?;

    let mut user_insurance_fund_account = ctx.accounts.user_insurance_fund_account.load_mut()?;
    user_insurance_fund_account.add_balance(
        user_deposited_share_amount,
        &ctx.accounts.bank_insurance_vault.key(),
    );

    emit!(MarginfiDepositIntoLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund.bank,
        },
        amount: user_deposited_share_amount,
        signer_token_address: ctx.accounts.signer_token_account.key(),
    });

    Ok(())
}
