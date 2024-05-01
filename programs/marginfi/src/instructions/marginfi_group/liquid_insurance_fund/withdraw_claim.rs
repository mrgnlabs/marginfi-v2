use crate::{
    check,
    constants::{
        INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUID_INSURANCE_WITHDRAW_SEED,
    },
    events::{LiquidInsuranceFundEventHeader, MarginfiWithdrawClaimLiquidInsuranceFundEvent},
    state::{
        liquid_insurance_fund::LiquidInsuranceFund,
        marginfi_account::{MarginfiAccount, DISABLED_FLAG},
        marginfi_group::Bank,
    },
    MarginfiError, MarginfiGroup, MarginfiResult, WithdrawParams,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{
    burn, close_account, Burn, CloseAccount, Mint, Token, TokenAccount, Transfer,
};
use fixed::types::I80F48;

#[derive(Accounts)]
pub struct SettleWithdrawClaimInLiquidInsuranceFund<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = marginfi_account.load()?.group == marginfi_group.key(),
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub mint_token: Account<'info, Mint>,

    #[account(
        mut,
        constraint = signer.key() == withdraw_params_account.load()?.signer.key(),
    )]
    pub signer: Signer<'info>,

    #[account(
        mut,
        constraint = signer_token_account.key() == withdraw_params_account.load()?.signer_token_account.key(),
    )]
    pub signer_token_account: Account<'info, TokenAccount>,

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
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_authority_bump
    )]
    pub bank_insurance_vault_authority: AccountInfo<'info>,

    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

    #[account(
        seeds = [
            LIQUID_INSURANCE_WITHDRAW_SEED.as_bytes(),
            bank.key().as_ref(),
            ],
            bump,
    )]
    pub withdraw_params_account: AccountLoader<'info, WithdrawParams>,

    pub token_program: Program<'info, Token>,
}

pub fn settle_withdraw_claim_in_liquid_insurance_fund(
    ctx: Context<SettleWithdrawClaimInLiquidInsuranceFund>,
) -> MarginfiResult {
    let SettleWithdrawClaimInLiquidInsuranceFund {
        marginfi_group,
        marginfi_account,
        mint_token,
        signer,
        signer_token_account,
        bank,
        bank_insurance_vault,
        bank_insurance_vault_authority,
        liquid_insurance_fund,
        withdraw_params_account,
        token_program,
    } = ctx.accounts;

    check!(
        !ctx.accounts
            .marginfi_account
            .load()?
            .get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

    let min_withdraw_period = ctx
        .accounts
        .liquid_insurance_fund
        .load()?
        .min_withdraw_period;
    let current_time = Clock::get()?.unix_timestamp;

    // Ensure enough time has passed since the withdraw request was made
    check!(
        current_time.checked_add(min_withdraw_period).unwrap()
            > ctx.accounts.withdraw_params_account.load()?.timestamp,
        MarginfiError::InvalidWithdrawal
    );

    let mut liquid_insurance_fund = ctx.accounts.liquid_insurance_fund.load_mut()?;
    let total_bank_insurance_vault_amount = ctx.accounts.bank_insurance_vault.amount;

    // User shares
    let user_withdraw_shares = ctx.accounts.withdraw_params_account.load()?.shares;
    // Convert to units of bank insurance vault
    let user_withdraw_tokens =
        liquid_insurance_fund.get_value(I80F48::from_num(user_withdraw_shares))?;
    let user_withdraw_tokens = user_withdraw_tokens
        .checked_to_num::<u64>()
        .ok_or(MarginfiError::MathError)?;

    // Internal accounting update
    liquid_insurance_fund.withdraw_shares(
        I80F48::from_num(user_withdraw_shares),
        I80F48::from_num(total_bank_insurance_vault_amount),
    )?;

    // Withdraw user funds from the relevant insurance vault
    liquid_insurance_fund.withdraw_spl_transfer(
        user_withdraw_tokens,
        Transfer {
            from: ctx.accounts.bank_insurance_vault.to_account_info(),
            to: ctx.accounts.signer_token_account.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        },
        token_program.to_account_info(),
    )?;

    // Burn the amount of tokenized shares withdraw by the user
    burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                authority: ctx.accounts.signer.to_account_info(),
                from: ctx.accounts.signer_token_account.to_account_info(),
                mint: ctx.accounts.mint_token.to_account_info(),
            },
        ),
        user_withdraw_shares,
    )?;

    // close the withdraw params account
    let mut withdraw_params_account = ctx.accounts.withdraw_params_account.load_mut();

    close_account(CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.withdraw_params_account.to_account_info(),
            destination: ctx.accounts.signer.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        },
    ))?;

    emit!(MarginfiWithdrawClaimLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund.bank,
            bank_insurance_vault: liquid_insurance_fund.bank_insurance_vault,
            token_mint: liquid_insurance_fund.mint
        },
        amount: user_withdraw_shares,
        success: true,
    });

    Ok(())
}
