use crate::{
    check,
    prelude::MarginfiError,
    state::marginfi_group::{Bank, BankConfig, BankConfigOpt, GroupConfig, MarginfiGroup},
    MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};

pub fn initialize(ctx: Context<InitializeMarginfiGroup>) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_init()?;

    let InitializeMarginfiGroup { admin, .. } = ctx.accounts;

    marginfi_group.set_initial_configuration(admin.key());

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeMarginfiGroup<'info> {
    #[account(zero)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Configure margin group
pub fn configure(ctx: Context<ConfigureMarginfiGroup>, config: GroupConfig) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_mut()?;

    marginfi_group.configure(config)?;

    Ok(())
}

#[derive(Accounts)]
pub struct ConfigureMarginfiGroup<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,
}

/// Add a bank to the lending pool
pub fn lending_pool_add_bank(
    ctx: Context<LendingPoolAddBank>,
    bank_index: u8,
    bank_config: BankConfig,
) -> MarginfiResult {
    let LendingPoolAddBank {
        asset_mint,
        liquidity_vault,
        insurance_vault,
        fee_vault,
        marginfi_group,
        ..
    } = ctx.accounts;

    let mut marginfi_group = marginfi_group.load_mut()?;

    check!(
        marginfi_group.lending_pool.banks[bank_index as usize].is_none(),
        MarginfiError::BankAlreadyExists
    );

    let bank = Bank::new(
        bank_config,
        asset_mint.key(),
        liquidity_vault.key(),
        insurance_vault.key(),
        fee_vault.key(),
    );

    marginfi_group.lending_pool.banks[bank_index as usize] = Some(bank);

    Ok(())
}

//. TODO: Add security checks
#[derive(Accounts)]
pub struct LendingPoolAddBank<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,
    pub asset_mint: Account<'info, Mint>,
    pub liquidity_vault: Account<'info, TokenAccount>,
    pub insurance_vault: Account<'info, TokenAccount>,
    pub fee_vault: Account<'info, TokenAccount>,
}

pub fn lending_pool_configure_bank(
    ctx: Context<LendingPoolConfigureBank>,
    bank_index: u8,
    bank_config: BankConfigOpt,
) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_mut()?;

    let mut bank = marginfi_group
        .lending_pool
        .banks
        .get_mut(bank_index as usize)
        .expect("Bank index out of bounds")
        .ok_or(MarginfiError::BankNotFound)?;

    bank.configure(bank_config)?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolConfigureBank<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,
}
