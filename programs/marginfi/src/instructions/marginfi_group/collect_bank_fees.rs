use crate::constants::{FEE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_AUTHORITY_SEED};
use crate::events::{GroupEventHeader, LendingPoolBankCollectFeesEvent};
use crate::{
    bank_signer,
    constants::{
        FEE_VAULT_SEED, INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED,
    },
    math_error,
    state::marginfi_group::{Bank, BankVaultType, MarginfiGroup},
    MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer};
use fixed::types::I80F48;
use std::cmp::min;

pub fn lending_pool_collect_bank_fees(ctx: Context<LendingPoolCollectBankFees>) -> MarginfiResult {
    let LendingPoolCollectBankFees {
        liquidity_vault_authority,
        insurance_vault,
        fee_vault,
        token_program,
        liquidity_vault,
        ..
    } = ctx.accounts;

    let mut bank = ctx.accounts.bank.load_mut()?;

    let mut available_liquidity = I80F48::from_num(liquidity_vault.amount);

    let (insurance_fee_transfer_amount, new_outstanding_insurance_fees) = {
        let outstanding = I80F48::from(bank.collected_insurance_fees_outstanding);
        let transfer_amount = min(outstanding, available_liquidity).int();

        (
            transfer_amount.int(),
            outstanding
                .checked_sub(transfer_amount)
                .ok_or_else(math_error!())?,
        )
    };

    bank.collected_insurance_fees_outstanding = new_outstanding_insurance_fees.into();

    available_liquidity = available_liquidity
        .checked_sub(insurance_fee_transfer_amount)
        .ok_or_else(math_error!())?;

    let (group_fee_transfer_amount, new_outstanding_group_fees) = {
        let outstanding = I80F48::from(bank.collected_group_fees_outstanding);
        let transfer_amount = min(outstanding, available_liquidity).int();

        (
            transfer_amount.int(),
            outstanding
                .checked_sub(transfer_amount)
                .ok_or_else(math_error!())?,
        )
    };

    available_liquidity = available_liquidity
        .checked_sub(group_fee_transfer_amount)
        .ok_or_else(math_error!())?;

    assert!(available_liquidity >= I80F48::ZERO);

    bank.collected_group_fees_outstanding = new_outstanding_group_fees.into();

    bank.withdraw_spl_transfer(
        group_fee_transfer_amount
            .checked_to_num()
            .ok_or_else(math_error!())?,
        Transfer {
            from: liquidity_vault.to_account_info(),
            to: fee_vault.to_account_info(),
            authority: liquidity_vault_authority.to_account_info(),
        },
        token_program.to_account_info(),
        bank_signer!(
            BankVaultType::Liquidity,
            ctx.accounts.bank.key(),
            bank.liquidity_vault_authority_bump
        ),
    )?;

    bank.withdraw_spl_transfer(
        insurance_fee_transfer_amount
            .checked_to_num()
            .ok_or_else(math_error!())?,
        Transfer {
            from: liquidity_vault.to_account_info(),
            to: insurance_vault.to_account_info(),
            authority: liquidity_vault_authority.to_account_info(),
        },
        token_program.to_account_info(),
        bank_signer!(
            BankVaultType::Liquidity,
            ctx.accounts.bank.key(),
            bank.liquidity_vault_authority_bump
        ),
    )?;

    emit!(LendingPoolBankCollectFeesEvent {
        header: GroupEventHeader {
            marginfi_group: ctx.accounts.marginfi_group.key(),
            signer: None
        },
        bank: ctx.accounts.bank.key(),
        mint: liquidity_vault.mint,
        insurance_fees_collected: insurance_fee_transfer_amount.to_num::<f64>(),
        insurance_fees_outstanding: new_outstanding_insurance_fees.to_num::<f64>(),
        group_fees_collected: group_fee_transfer_amount.to_num::<f64>(),
        group_fees_outstanding: new_outstanding_group_fees.to_num::<f64>(),
    });

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolCollectBankFees<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.liquidity_vault_authority_bump
    )]
    pub liquidity_vault_authority: AccountInfo<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.liquidity_vault_bump
    )]
    pub liquidity_vault: Account<'info, TokenAccount>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            INSURANCE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_bump
    )]
    pub insurance_vault: AccountInfo<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            FEE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.fee_vault_bump
    )]
    pub fee_vault: AccountInfo<'info>,

    #[account(
        mut,
        address = crate::constants::PROTOCOL_FEE_TREASURY
    )]
    pub protocol_fee_vault: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn lending_pool_withdraw_fees(
    ctx: Context<LendingPoolWithdrawFees>,
    amount: u64,
) -> MarginfiResult {
    let LendingPoolWithdrawFees {
        bank: bank_loader,
        fee_vault,
        fee_vault_authority,
        dst_token_account,
        token_program,
        ..
    } = ctx.accounts;

    let bank = bank_loader.load()?;

    bank.withdraw_spl_transfer(
        amount,
        Transfer {
            from: fee_vault.to_account_info(),
            to: dst_token_account.to_account_info(),
            authority: fee_vault_authority.to_account_info(),
        },
        token_program.to_account_info(),
        bank_signer!(
            BankVaultType::Fee,
            bank_loader.key(),
            bank.fee_vault_authority_bump
        ),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolWithdrawFees<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            FEE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.fee_vault_bump
    )]
    pub fee_vault: AccountInfo<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            FEE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.fee_vault_authority_bump
    )]
    pub fee_vault_authority: AccountInfo<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(mut)]
    pub dst_token_account: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn lending_pool_withdraw_insurance(
    ctx: Context<LendingPoolWithdrawInsurance>,
    amount: u64,
) -> MarginfiResult {
    let LendingPoolWithdrawInsurance {
        bank: bank_loader,
        insurance_vault,
        insurance_vault_authority,
        dst_token_account,
        token_program,
        ..
    } = ctx.accounts;

    let bank = bank_loader.load()?;

    bank.withdraw_spl_transfer(
        amount,
        Transfer {
            from: insurance_vault.to_account_info(),
            to: dst_token_account.to_account_info(),
            authority: insurance_vault_authority.to_account_info(),
        },
        token_program.to_account_info(),
        bank_signer!(
            BankVaultType::Insurance,
            bank_loader.key(),
            bank.insurance_vault_authority_bump
        ),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolWithdrawInsurance<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            INSURANCE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_bump
    )]
    pub insurance_vault: AccountInfo<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_authority_bump
    )]
    pub insurance_vault_authority: AccountInfo<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(mut)]
    pub dst_token_account: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}
