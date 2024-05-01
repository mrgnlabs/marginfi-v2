use crate::{
    check,
    constants::{
        INSURANCE_VAULT_SEED, LIQUID_INSURANCE_MINT_AUTHORITY_SEED, LIQUID_INSURANCE_MINT_SEED,
    },
    events::{
        LiquidInsuranceFundEventHeader, MarginfiCreateNewLiquidInsuranceFundEvent,
        MarginfiDepositIntoLiquidInsuranceFundEvent,
    },
    state::{
        liquid_insurance_fund::LiquidInsuranceFund,
        marginfi_account::{MarginfiAccount, DISABLED_FLAG},
        marginfi_group::Bank,
    },
    MarginfiError, MarginfiGroup, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, Mint, MintTo, Token, TokenAccount, Transfer};
use fixed::types::I80F48;

#[derive(Accounts)]
pub struct DepositIntoLiquidInsuranceFund<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = marginfi_account.load()?.group == marginfi_group.key(),
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

    #[account(
        mut,
        address = marginfi_account.load()?.authority,
    )]
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
            LIQUID_INSURANCE_MINT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump
    )]
    pub mint_authority: AccountInfo<'info>,

    #[account(
        seeds = [
            LIQUID_INSURANCE_MINT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

pub fn deposit_into_liquid_insurance_fund(
    ctx: Context<DepositIntoLiquidInsuranceFund>,
    amount: u64,
) -> MarginfiResult {
    let DepositIntoLiquidInsuranceFund {
        marginfi_group: marginfi_group_loader,
        marginfi_account: marginfi_account_loader,
        liquid_insurance_fund,
        signer,
        signer_token_account,
        bank,
        bank_insurance_vault,
        mint_authority,
        mint,
        token_program,
        ..
    } = ctx.accounts;

    check!(
        !ctx.accounts
            .marginfi_account
            .load()?
            .get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

    let mut liquid_insurance_fund = ctx.accounts.liquid_insurance_fund.load_mut()?;

    // Get amount inside the bank's insurance vault
    let total_bank_insurance_vault_amount = ctx.accounts.bank_insurance_vault.amount;

    // update shares of the liquid insurance fund
    liquid_insurance_fund.deposit_shares(
        I80F48::from_num(amount),
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

    // Send the user a tokenized representation of their deposit as shares
    let user_deposited_share_amount = liquid_insurance_fund.get_shares(I80F48::from_num(amount))?;
    let user_deposited_share_amount = user_deposited_share_amount
        .checked_to_num::<u64>()
        .ok_or(MarginfiError::MathError)?;

    // mint tokens from mint and send to the senders token account
    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                authority: ctx.accounts.mint_authority.to_account_info(),
                to: ctx.accounts.signer_token_account.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
            &[&[&[*ctx.bumps.get(LIQUID_INSURANCE_MINT_SEED).unwrap()]]],
        ),
        user_deposited_share_amount,
    )?;

    emit!(MarginfiDepositIntoLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund.bank,
            bank_insurance_vault: liquid_insurance_fund.bank_insurance_vault,
            token_mint: liquid_insurance_fund.mint
        },
        amount: user_deposited_share_amount,
        signer_token_address: ctx.accounts.signer_token_account.key(),
    });

    Ok(())
}
