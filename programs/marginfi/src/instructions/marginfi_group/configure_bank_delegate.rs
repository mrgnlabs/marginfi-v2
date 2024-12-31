use crate::{
    state::marginfi_group::{Bank, MarginfiGroup},
    MarginfiResult,
};
use crate::{check, MarginfiError};
use anchor_lang::prelude::*;

pub fn lending_pool_configure_bank_delegate(
    ctx: Context<LendingPoolConfigureBankDelegate>,
    delegate_admin: Pubkey,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;
    bank.delegate_admin = delegate_admin;
    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolConfigureBankDelegate<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,
}

pub fn lending_pool_increase_deposit_limit(
    ctx: Context<LendingPoolDelegateOperation>,
    new_deposit_limit: u64,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;
    
    // Only allow increasing the limit
    check!(new_deposit_limit <= bank.config.deposit_limit, MarginfiError::InvalidDelegateOperation);
    
    bank.config.deposit_limit = new_deposit_limit;
    Ok(())
}

pub fn lending_pool_increase_emissions_rate(
    ctx: Context<LendingPoolDelegateOperation>,
    new_emissions_rate: u64,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;
    
    // Only allow increasing the rate
    check!(new_emissions_rate <= bank.emissions_rate, MarginfiError::InvalidDelegateOperation);
    
    bank.emissions_rate = new_emissions_rate;
    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolDelegateOperation<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        constraint = bank.load()?.delegate_admin == delegate_admin.key(),
    )]
    pub delegate_admin: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,
}
