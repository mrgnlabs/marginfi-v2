use crate::{
    constants::LIQUID_INSURANCE_USER_SEED,
    events::{LiquidInsuranceFundEventHeader, MarginfiWithdrawRequestLiquidInsuranceFundEvent},
    state::liquid_insurance_fund::{LiquidInsuranceFund, LiquidInsuranceFundAccount},
    MarginfiResult,
};
use anchor_lang::prelude::*;
use fixed::types::I80F48;

#[derive(Accounts)]
#[instruction(
    signer_bump: u8,
)]
pub struct WithdrawRequestLiquidInsuranceFund<'info> {
    #[account(mut)]
    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        mut,
        seeds = [
            LIQUID_INSURANCE_USER_SEED.as_bytes(),
            signer.key().as_ref(),
        ],
        bump
    )]
    pub user_insurance_fund_account: AccountLoader<'info, LiquidInsuranceFundAccount>,
}

pub fn create_withdraw_request_from_liquid_token_fund(
    ctx: Context<WithdrawRequestLiquidInsuranceFund>,
    shares: Option<I80F48>,
) -> MarginfiResult {
    // Note: I80F48::ZERO is not positive -> returns early
    if shares.map(|s| !s.is_positive()) == Some(true) {
        return Ok(());
    }

    let WithdrawRequestLiquidInsuranceFund {
        liquid_insurance_fund,
        user_insurance_fund_account,
        ..
    } = ctx.accounts;
    let clock = Clock::get()?;

    let mut user_insurance_fund_account = user_insurance_fund_account.load_mut()?;
    let liquid_insurance_fund_account = liquid_insurance_fund.load()?;

    let shares = user_insurance_fund_account.create_withdrawal(
        &liquid_insurance_fund.key(),
        shares,
        clock.unix_timestamp,
    )?;

    emit!(MarginfiWithdrawRequestLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund_account.bank,
        },
        shares: shares.to_num::<f64>(),
    });

    Ok(())
}
