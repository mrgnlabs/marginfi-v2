use crate::{
    constants::{INSURANCE_VAULT_SEED, LIQUID_INSURANCE_USER_SEED},
    events::{LiquidInsuranceFundEventHeader, MarginfiDepositIntoLiquidInsuranceFundEvent},
    state::liquid_insurance_fund::{LiquidInsuranceFund, LiquidInsuranceFundAccount},
    utils::calculate_post_fee_spl_deposit_amount,
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use fixed::types::I80F48;

#[derive(Accounts)]
pub struct DepositIntoLiquidInsuranceFund<'info> {
    #[account(mut)]
    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

    #[account(mut)]
    pub signer: Signer<'info>,

    /// CHECK: Account to move tokens into
    #[account(mut)]
    pub signer_token_account: AccountInfo<'info>,

    /// The corresponding insurance vault that the liquid insurance fund deposits into.
    /// This is the insurance vault of the underlying bank
    #[account(
        mut,
        seeds = [
            INSURANCE_VAULT_SEED.as_bytes(),
            liquid_insurance_fund.load()?.bank.as_ref(),
        ],
        bump = liquid_insurance_fund.load()?.lif_vault_bump,
    )]
    pub bank_insurance_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [
            LIQUID_INSURANCE_USER_SEED.as_bytes(),
            signer.key().as_ref(),
        ],
        bump
    )]
    pub user_insurance_fund_account: AccountLoader<'info, LiquidInsuranceFundAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

/// 1) Check for existing deposit, or try to find free slot if non-existent
/// 2) Calculate deposit_num_shares(deposit_amount)
/// 3) SPL transfer to deposit
/// 4) Update user shares, total shares
pub fn deposit_into_liquid_insurance_fund<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, DepositIntoLiquidInsuranceFund<'info>>,
    deposit_amount: u64,
) -> MarginfiResult {
    if deposit_amount == 0 {
        return Ok(());
    }

    let DepositIntoLiquidInsuranceFund {
        liquid_insurance_fund: liquid_insurance_fund_loader,
        signer,
        signer_token_account,
        bank_insurance_vault,
        token_program,
        user_insurance_fund_account,
        ..
    } = ctx.accounts;
    let mut liquid_insurance_fund = liquid_insurance_fund_loader.load_mut()?;
    let maybe_bank_mint = crate::utils::maybe_take_bank_mint(
        &mut ctx.remaining_accounts,
        &liquid_insurance_fund.bank_mint,
        token_program.key,
    )?;
    let clock = Clock::get()?;

    let mut user_insurance_fund_account = user_insurance_fund_account.load_mut()?;

    // 1) Check for existing deposit, or try to find free slot if non-existent
    let deposit = user_insurance_fund_account
        .get_or_init_deposit(&liquid_insurance_fund_loader.key())
        .ok_or(MarginfiError::InsuranceFundAccountBalanceSlotsFull)?;

    liquid_insurance_fund.update_share_price_internal(bank_insurance_vault.amount.into())?;

    // 2) Calculate deposit_num_shares(deposit_amount)
    let postfee_deposit_amount = maybe_bank_mint
        .as_ref()
        .map(|mint_ai| {
            calculate_post_fee_spl_deposit_amount(
                mint_ai.to_account_info(),
                deposit_amount,
                clock.epoch,
            )
        })
        .unwrap_or(Ok(deposit_amount))?;
    let deposit_num_shares: I80F48 =
        liquid_insurance_fund.get_shares(I80F48::from_num(postfee_deposit_amount))?;

    //  SPL transfer to deposit
    liquid_insurance_fund.deposit_spl_transfer(
        deposit_amount,
        signer_token_account.to_account_info(),
        bank_insurance_vault.to_account_info(),
        signer.to_account_info(),
        token_program.to_account_info(),
        maybe_bank_mint.as_ref(),
        ctx.remaining_accounts,
    )?;

    // 3) Update user shares, total shares
    deposit.add_shares(deposit_num_shares);
    liquid_insurance_fund.deposit_shares(deposit_num_shares)?;

    emit!(MarginfiDepositIntoLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund.bank,
        },
        amount: postfee_deposit_amount,
        signer_token_address: signer_token_account.key(),
    });

    Ok(())
}
