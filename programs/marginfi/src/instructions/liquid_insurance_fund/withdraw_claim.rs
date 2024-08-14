use crate::{
    bank_signer, check,
    constants::{INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUID_INSURANCE_USER_SEED},
    events::{LiquidInsuranceFundEventHeader, MarginfiWithdrawClaimLiquidInsuranceFundEvent},
    math_error,
    state::{
        liquid_insurance_fund::{LiquidInsuranceFund, LiquidInsuranceFundAccount},
        marginfi_group::BankVaultType,
    },
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct SettleWithdrawClaimInLiquidInsuranceFund<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub signer_token_account: InterfaceAccount<'info, TokenAccount>,

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
    pub bank_insurance_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            liquid_insurance_fund.load()?.bank.key().as_ref(),
        ],
        bump = liquid_insurance_fund.load()?.lif_authority_bump,
    )]
    pub bank_insurance_vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

    #[account(
        mut,
        seeds = [
            LIQUID_INSURANCE_USER_SEED.as_bytes(),
            signer.key().as_ref(),
        ],
        bump,
    )]
    pub user_insurance_fund_account: AccountLoader<'info, LiquidInsuranceFundAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

/// 1) Retrieve earliest withdraw request for this user, for this liquid insurance fund
/// 2) Check whether enough time has passed
/// 3) Process withdrawal
/// 4) Transfer tokens
pub fn settle_withdraw_claim_in_liquid_insurance_fund<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, SettleWithdrawClaimInLiquidInsuranceFund<'info>>,
) -> MarginfiResult {
    let SettleWithdrawClaimInLiquidInsuranceFund {
        signer_token_account,
        bank_insurance_vault,
        bank_insurance_vault_authority,
        liquid_insurance_fund: liquid_insurance_fund_loader,
        user_insurance_fund_account,
        token_program,
        signer: _,
    } = ctx.accounts;
    let mut liquid_insurance_fund = liquid_insurance_fund_loader.load_mut()?;
    let maybe_bank_mint = crate::utils::maybe_take_bank_mint(
        &mut ctx.remaining_accounts,
        &liquid_insurance_fund.bank_mint,
        token_program.key,
    )?;
    let clock = Clock::get()?;

    // 1) Retrieve earliest withdraw request for this user, for this liquid insurance fund
    let mut user_account = user_insurance_fund_account.load_mut()?;
    liquid_insurance_fund.update_share_price_internal(bank_insurance_vault.amount.into())?;
    let withdrawal = user_account
        .get_earliest_withdrawal(&liquid_insurance_fund_loader.key())
        .ok_or(MarginfiError::InvalidWithdrawal)?;

    // 2) Check whether enough time has passed
    check!(
        clock.unix_timestamp
            >= withdrawal
                .withdraw_request_timestamp
                .checked_add(liquid_insurance_fund.min_withdraw_period)
                .ok_or_else(math_error!())?,
        // TODO: more informative error
        MarginfiError::InvalidWithdrawal
    );

    // 3) Calculate share value in tokens
    let user_withdraw_amount: u64 = liquid_insurance_fund.process_withdrawal(withdrawal)?;

    // Withdraw user funds from the relevant insurance vault
    liquid_insurance_fund.withdraw_spl_transfer(
        user_withdraw_amount,
        bank_insurance_vault.to_account_info(),
        signer_token_account.to_account_info(),
        bank_insurance_vault_authority.to_account_info(),
        token_program.to_account_info(),
        maybe_bank_mint.as_ref(),
        ctx.remaining_accounts,
        bank_signer!(
            BankVaultType::Insurance,
            liquid_insurance_fund.bank,
            liquid_insurance_fund.lif_authority_bump
        ),
    )?;

    emit!(MarginfiWithdrawClaimLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund.bank,
        },
        amount: user_withdraw_amount,
        success: true,
    });

    Ok(())
}
