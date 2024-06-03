use crate::{
    check,
    constants::{INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUID_INSURANCE_USER_SEED},
    events::{LiquidInsuranceFundEventHeader, MarginfiWithdrawRequestLiquidInsuranceFundEvent},
    state::{
        liquid_insurance_fund::{
            LiquidInsuranceFund, LiquidInsuranceFundAccount, LiquidInsuranceFundAccountData,
        },
        marginfi_group::Bank,
    },
    MarginfiError, MarginfiGroup, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use fixed::types::I80F48;

#[instruction(
    signer_bump: u8,
)]
#[derive(Accounts)]
pub struct WithdrawRequestLiquidInsuranceFund<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

    #[account(mut)]
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

    #[account(
        seeds = [
            LIQUID_INSURANCE_USER_SEED.as_bytes(),
            signer.key().as_ref(),
        ],
        bump
    )]
    pub user_insurance_fund_account: AccountLoader<'info, LiquidInsuranceFundAccount>,

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
        user_insurance_fund_account,
        system_program,
        ..
    } = ctx.accounts;

    check!(shares > 0, MarginfiError::InvalidWithdrawal);

    let mut user_insurance_fund_account = ctx.accounts.user_insurance_fund_account.load_mut()?;
    check!(
        user_insurance_fund_account.authority == ctx.accounts.signer.key(),
        MarginfiError::Unauthorized
    );

    let liquid_insurance_fund = ctx.accounts.liquid_insurance_fund.load_mut()?;

    // check user has these balances and the fund has enough funds draw from
    let existing_user_share_balance =
        user_insurance_fund_account.get_balance(&ctx.accounts.bank_insurance_vault.key())?;

    check!(
        existing_user_share_balance >= shares,
        MarginfiError::InvalidWithdrawal
    );
    check!(
        liquid_insurance_fund.total_shares.into() >= (existing_user_share_balance + shares),
        MarginfiError::InvalidWithdrawal
    );

    // Lock shares
    // Update user balances

    // Create withdraw claim with now + 2 weeks as the earliest possible withdraw time
    let current_timestamp = Clock::get()?.unix_timestamp;

    let locked_shares = user_insurance_fund_account.lock_shares(shares)?;
    user_insurance_fund_account.create_withdrawal(ctx.accounts.bank_insurance_vault.key(), locked_shares, current_timestamp);

    emit!(MarginfiWithdrawRequestLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund.bank,
        },
        amount: withdraw_user_amount,
    });

    Ok(())
}
