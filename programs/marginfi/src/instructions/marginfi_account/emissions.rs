use anchor_lang::{prelude::*, Accounts, ToAccountInfo};
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    check,
    constants::{EMISSIONS_AUTH_SEED, EMISSIONS_TOKEN_ACCOUNT_SEED},
    debug,
    prelude::{MarginfiError, MarginfiResult},
    state::{
        marginfi_account::{BankAccountWrapper, MarginfiAccount, DISABLED_FLAG},
        marginfi_group::{Bank, MarginfiGroup},
    },
};

pub fn lending_account_withdraw_emissions<'info>(
    ctx: Context<'_, '_, 'info, 'info, LendingAccountWithdrawEmissions<'info>>,
) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;

    check!(
        !marginfi_account.get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

    let mut bank = ctx.accounts.bank.load_mut()?;

    let mut balance = BankAccountWrapper::find(
        ctx.accounts.bank.to_account_info().key,
        &mut bank,
        &mut marginfi_account.lending_account,
    )?;

    // Settle emissions
    let emissions_settle_amount = balance.settle_emissions_and_get_transfer_amount()?;

    if emissions_settle_amount > 0 {
        debug!("Transferring {} emissions to user", emissions_settle_amount);

        let signer_seeds: &[&[&[u8]]] = &[&[
            EMISSIONS_AUTH_SEED.as_bytes(),
            &ctx.accounts.bank.key().to_bytes(),
            &ctx.accounts.emissions_mint.key().to_bytes(),
            &[ctx.bumps.emissions_auth],
        ]];

        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.emissions_vault.to_account_info(),
                    to: ctx.accounts.destination_account.to_account_info(),
                    authority: ctx.accounts.emissions_auth.to_account_info(),
                    mint: ctx.accounts.emissions_mint.to_account_info(),
                },
                signer_seeds,
            ),
            emissions_settle_amount,
            ctx.accounts.emissions_mint.decimals,
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountWithdrawEmissions<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group,
        has_one = authority
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = emissions_mint
    )]
    pub bank: AccountLoader<'info, Bank>,

    pub emissions_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: PDA seeds validated
    #[account(
        seeds = [
            EMISSIONS_AUTH_SEED.as_bytes(),
            bank.key().as_ref(),
            emissions_mint.key().as_ref(),
        ],
        bump
    )]
    pub emissions_auth: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            EMISSIONS_TOKEN_ACCOUNT_SEED.as_bytes(),
            bank.key().as_ref(),
            emissions_mint.key().as_ref(),
        ],
        bump,
    )]
    pub emissions_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub destination_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
}

/// Permissionlessly settle unclaimed emissions to a users account.
pub fn lending_account_settle_emissions(
    ctx: Context<LendingAccountSettleEmissions>,
) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let mut balance = BankAccountWrapper::find(
        ctx.accounts.bank.to_account_info().key,
        &mut bank,
        &mut marginfi_account.lending_account,
    )?;

    balance.claim_emissions(Clock::get()?.unix_timestamp.try_into().unwrap())?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountSettleEmissions<'info> {
    #[account(
        mut,
        constraint = marginfi_account.load()?.group == bank.load()?.group,
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub bank: AccountLoader<'info, Bank>,
}

/// emissions rewards will be withdrawn to the emissions_destination_account
pub fn marginfi_account_update_emissions_destination_account<'info>(
    ctx: Context<'_, '_, 'info, 'info, MarginfiAccountUpdateEmissionsDestinationAccount<'info>>,
) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;

    check!(
        !marginfi_account.get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

    let emissions_destination_account = &marginfi_account.emissions_destination_account;

    // Ensure that the emissions_destination_account was not previously set by the user
    check!(
        emissions_destination_account.eq(&Pubkey::default()),
        MarginfiError::EmissionsDestinationAccountAlreadySet
    );

    let _ = marginfi_account
        .update_emissions_destination_account(ctx.accounts.destination_account.key());

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiAccountUpdateEmissionsDestinationAccount<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = marginfi_account.load()?.group == marginfi_group.key(),
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        address = marginfi_account.load()?.authority,
    )]
    pub signer: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        address = bank.load()?.emissions_mint
    )]
    pub emissions_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub destination_account: Box<InterfaceAccount<'info, TokenAccount>>,
}

/// Permissionlessly withdraw emissions to user emissions_destination_account
pub fn lending_account_withdraw_emissions_permissionless<'info>(
    ctx: Context<'_, '_, 'info, 'info, LendingAccountWithdrawEmissionsPermissionless<'info>>,
) -> MarginfiResult {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;

    check!(
        !marginfi_account.get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

    let emissions_destination_account = &marginfi_account.emissions_destination_account;

    // Ensure that the emissions_destination_account was previously set by the user
    check!(
        !emissions_destination_account.eq(&Pubkey::default()),
        MarginfiError::InvalidEmissionsDestinationAccount
    );

    // Ensure that the destination_account matches the stored one in MarginfiAccount
    check!(
        ctx.accounts
            .destination_account
            .key()
            .eq(&emissions_destination_account),
        MarginfiError::InvalidEmissionsDestinationAccount
    );

    let mut bank = ctx.accounts.bank.load_mut()?;

    let mut bank_account = BankAccountWrapper::find(
        ctx.accounts.bank.to_account_info().key,
        &mut bank,
        &mut marginfi_account.lending_account,
    )?;

    // Settle emissions
    let emissions_settle_amount = bank_account.settle_emissions_and_get_transfer_amount()?;

    if emissions_settle_amount > 0 {
        debug!("Transferring {} emissions to user", emissions_settle_amount);

        let signer_seeds: &[&[&[u8]]] = &[&[
            EMISSIONS_AUTH_SEED.as_bytes(),
            &ctx.accounts.bank.key().to_bytes(),
            &ctx.accounts.emissions_mint.key().to_bytes(),
            &[ctx.bumps.emissions_auth],
        ]];

        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.emissions_vault.to_account_info(),
                    to: ctx.accounts.destination_account.to_account_info(),
                    authority: ctx.accounts.emissions_auth.to_account_info(),
                    mint: ctx.accounts.emissions_mint.to_account_info(),
                },
                signer_seeds,
            ),
            emissions_settle_amount,
            ctx.accounts.emissions_mint.decimals,
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountWithdrawEmissionsPermissionless<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = marginfi_account.load()?.group == marginfi_group.key(),
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        address = bank.load()?.emissions_mint
    )]
    pub emissions_mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [
            EMISSIONS_AUTH_SEED.as_bytes(),
            bank.key().as_ref(),
            emissions_mint.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: Asserted by PDA
    pub emissions_auth: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            EMISSIONS_TOKEN_ACCOUNT_SEED.as_bytes(),
            bank.key().as_ref(),
            emissions_mint.key().as_ref(),
        ],
        bump,
    )]
    pub emissions_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub destination_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
}
