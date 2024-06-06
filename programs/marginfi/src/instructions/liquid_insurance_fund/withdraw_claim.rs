use crate::{
    check,
    constants::{INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUID_INSURANCE_USER_SEED},
    events::{LiquidInsuranceFundEventHeader, MarginfiWithdrawClaimLiquidInsuranceFundEvent},
    math_error,
    state::liquid_insurance_fund::{LiquidInsuranceFund, LiquidInsuranceFundAccount},
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct SettleWithdrawClaimInLiquidInsuranceFund<'info> {
    #[account(
        mut,
        address = user_insurance_fund_account.load()?.authority,
    )]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,

    /// The corresponding insurance vault that the liquid insurance fund deposits into.
    /// This is the insurance vault of the underlying bank
    #[account(
        mut,
        seeds = [
            INSURANCE_VAULT_SEED.as_bytes(),
            liquid_insurance_fund.load()?.bank.key().as_ref(),
        ],
        bump = liquid_insurance_fund.load()?.lif_vault_bump,
    )]
    pub bank_insurance_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            liquid_insurance_fund.load()?.bank.key().as_ref(),
        ],
        bump = liquid_insurance_fund.load()?.lif_authority_bump,
    )]
    pub bank_insurance_vault_authority: AccountInfo<'info>,

    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

    #[account(
        mut,
        close = bank_insurance_vault,
        seeds = [
            LIQUID_INSURANCE_USER_SEED.as_bytes(),
            signer.key().as_ref(),
        ],
        bump,
    )]
    pub user_insurance_fund_account: AccountLoader<'info, LiquidInsuranceFundAccount>,

    pub token_program: Program<'info, Token>,
}

/// 1) Retrieve earliest withdraw request for this user, for this liquid insurance fund
/// 2) Check whether enough time has passed
/// 3) Process withdrawal
/// 4) Transfer tokens
pub fn settle_withdraw_claim_in_liquid_insurance_fund(
    ctx: Context<SettleWithdrawClaimInLiquidInsuranceFund>,
) -> MarginfiResult {
    let SettleWithdrawClaimInLiquidInsuranceFund {
        signer_token_account,
        bank_insurance_vault,
        bank_insurance_vault_authority,
        liquid_insurance_fund,
        user_insurance_fund_account,
        token_program,
        signer: _,
    } = ctx.accounts;
    let clock = Clock::get()?;

    // 1) Retrieve earliest withdraw request for this user, for this liquid insurance fund
    let mut user_account = user_insurance_fund_account.load_mut()?;
    let mut liquid_insurance_fund_account = liquid_insurance_fund.load_mut()?;
    let withdrawal = user_account
        .get_earliest_withdrawal(&liquid_insurance_fund.key())
        .ok_or(MarginfiError::InvalidWithdrawal)?;

    // 2) Check whether enough time has passed
    check!(
        withdrawal
            .withdraw_request_timestamp
            .checked_add(liquid_insurance_fund_account.min_withdraw_period)
            .ok_or_else(math_error!())?
            > clock.unix_timestamp,
        // TODO: more informative error
        MarginfiError::InvalidWithdrawal
    );

    // 3) Calculate share value in tokens
    let user_withdraw_amount: u64 = liquid_insurance_fund_account.process_withdrawal(withdrawal)?;

    // Withdraw user funds from the relevant insurance vault
    liquid_insurance_fund_account.withdraw_spl_transfer(
        user_withdraw_amount,
        Transfer {
            from: bank_insurance_vault.to_account_info(),
            to: signer_token_account.to_account_info(),
            authority: bank_insurance_vault_authority.to_account_info(),
        },
        token_program.to_account_info(),
    )?;

    emit!(MarginfiWithdrawClaimLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund_account.bank,
        },
        amount: user_withdraw_amount,
        success: true,
    });

    Ok(())
}
