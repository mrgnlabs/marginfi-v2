use crate::{
    check,
    constants::{
        INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUID_INSURANCE_WITHDRAW_SEED,
    },
    events::{LiquidInsuranceFundEventHeader, MarginfiWithdrawRequestLiquidInsuranceFundEvent},
    math_error,
    state::{
        liquid_insurance_fund::{
            LiquidInsuranceFund, LiquidInsuranceFundAccount, LiquidInsuranceFundAccountData,
        },
        marginfi_group::Bank,
    },
    MarginfiError, MarginfiGroup, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fixed::types::I80F48;

#[instruction(
    signer_bump: u8,
)]
#[derive(Accounts)]
pub struct WithdrawRequestLiquidInsuranceFund<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

    #[account(
        mut,
        address = marginfi_group.load()?.admin,
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

    pub token_program: Program<'info, Token>,

    // PDA for the withdraw account
    #[account(
        init,
        seeds = [
            liquid_insurance_fund.key().as_ref(),
            signer.key().as_ref(),
            &[signer_bump],
        ],
        bump,
        payer = signer,
        space = 8 + std::mem::size_of::<LiquidInsuranceFundAccount>())]
    pub withdraw_params_account: AccountLoader<'info, LiquidInsuranceFundAccount>,

    pub system_program: Program<'info, System>,
}

pub fn create_withdraw_request_from_liquid_token_fund(
    ctx: Context<WithdrawRequestLiquidInsuranceFund>,
    shares: u64,
) -> MarginfiResult {
    let WithdrawRequestLiquidInsuranceFund {
        marginfi_group: marginfi_group_loader,
        liquid_insurance_fund,
        signer,
        signer_token_account,
        bank,
        bank_insurance_vault,
        bank_insurance_vault_authority,
        token_program,
        withdraw_params_account,
        system_program,
        ..
    } = ctx.accounts;

    check!(shares.ge(&0), MarginfiError::InvalidWithdrawal);

    // TODO: Additional withdraw validation?

    let liquid_insurance_fund = ctx.accounts.liquid_insurance_fund.load_mut()?;

    // The current value of the shares being withdrawn
    let withdraw_user_amount = liquid_insurance_fund.get_value(I80F48::from_num(shares))?;
    let withdraw_user_amount = withdraw_user_amount
        .checked_to_num::<u64>()
        .ok_or(MarginfiError::MathError)?;

    // Create withdraw claim with now + 2 weeks as the earliest possible withdraw time
    let current_timestamp = Clock::get()?.unix_timestamp;

    // create withdraw account

    let w = LiquidInsuranceFundAccountData {
        signer: ctx.accounts.signer.key(),
        signer_token_account: ctx.accounts.signer_token_account.key(),
        timestamp: current_timestamp,
        amount: withdraw_user_amount,
    };

    ctx.accounts.withdraw_params_account.load_mut()?.data = w;

    emit!(MarginfiWithdrawRequestLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund.bank,
        },
        amount: withdraw_user_amount,
    });

    Ok(())
}
