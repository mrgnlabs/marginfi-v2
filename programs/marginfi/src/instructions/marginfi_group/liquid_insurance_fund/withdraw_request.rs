use std::borrow::BorrowMut;

use crate::{
    check,
    constants::{
        INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUID_INSURANCE_MINT_AUTHORITY_SEED,
        LIQUID_INSURANCE_MINT_SEED, LIQUID_INSURANCE_WITHDRAW_SEED,
    },
    events::{LiquidInsuranceFundEventHeader, MarginfiWithdrawRequestLiquidInsuranceFundEvent},
    math_error,
    state::{
        liquid_insurance_fund::LiquidInsuranceFund,
        marginfi_account::{MarginfiAccount, DISABLED_FLAG},
        marginfi_group::Bank,
    },
    MarginfiError, MarginfiGroup, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fixed::types::I80F48;

#[derive(Accounts)]
pub struct WithdrawRequestLiquidInsuranceFund<'info> {
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

    /// CHECK: Account to move tokens out from
    #[account(mut)]
    pub signer_token_account: Box<Account<'info, TokenAccount>>,

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
    pub bank_insurance_vault: AccountInfo<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_authority_bump
    )]
    pub bank_insurance_vault_authority: AccountInfo<'info>,

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

    // PDA for the withdraw account
    #[account(
        init,
        seeds = [
            LIQUID_INSURANCE_WITHDRAW_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
        payer = signer,
        space = 8 + std::mem::size_of::<WithdrawParamsAccount>())]
    pub withdraw_params_account: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_withdraw_request_from_liquid_token_fund(
    ctx: Context<WithdrawRequestLiquidInsuranceFund>,
    amount: u64,
) -> MarginfiResult {
    let WithdrawRequestLiquidInsuranceFund {
        marginfi_group: marginfi_group_loader,
        marginfi_account: marginfi_account_loader,
        liquid_insurance_fund,
        signer,
        signer_token_account,
        bank,
        bank_insurance_vault,
        bank_insurance_vault_authority,
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

    check!(amount.ge(&0), MarginfiError::InvalidWithdrawal);

    // TODO: Additional withdraw validation?

    let mut liquid_insurance_fund = ctx.accounts.liquid_insurance_fund.load_mut()?;

    // The current value of the shares being withdrawn
    let withdraw_user_shares = liquid_insurance_fund.get_shares(I80F48::from_num(amount))?;
    let withdraw_user_shares = withdraw_user_shares
        .checked_to_num::<u64>()
        .ok_or(MarginfiError::MathError)?;

    // Create withdraw claim with now + 2 weeks as the earliest possible withdraw time
    let current_timestamp = Clock::get()?.unix_timestamp;
    let earliest_valid_timestamp = current_timestamp
        .checked_add(
            ctx.accounts
                .liquid_insurance_fund
                .load()?
                .min_withdraw_period,
        )
        .ok_or_else(math_error!())?;

    // create withdraw account
    let mut withdraw_pda = ctx.accounts.withdraw_params_account.borrow_mut();
    let mut withdraw_data = WithdrawParams {
        signer: ctx.accounts.signer.key(),
        signer_token_account: ctx.accounts.signer_token_account.key(),
        timestamp: earliest_valid_timestamp,
        shares: withdraw_user_shares,
    };
    // TODO: turn withdraw_pda.data into withdraw_data

    emit!(MarginfiWithdrawRequestLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund.bank,
            bank_insurance_vault: liquid_insurance_fund.bank_insurance_vault,
            token_mint: liquid_insurance_fund.mint
        },
        amount: withdraw_user_shares,
    });

    Ok(())
}

#[account]
pub struct WithdrawParamsAccount {
    data: WithdrawParams,
}

#[account(zero_copy)]
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct WithdrawParams {
    pub signer: Pubkey,
    pub signer_token_account: Pubkey,
    pub timestamp: i64,
    pub shares: u64,
}
