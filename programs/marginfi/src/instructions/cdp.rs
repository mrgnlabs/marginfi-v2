use std::cell::RefMut;

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, Transfer};
use fixed::types::I80F48;

use crate::{
    constants::{CDP_MINT_AUTH_SEED, LIQUIDITY_VAULT_SEED},
    prelude::MarginfiResult,
    state::{
        cdp::{Cdp, CdpBank, CdpCollateralBank, CdpCollateralBankStatus},
        marginfi_group::{Bank, MarginfiGroup},
    },
};

pub fn create_cdp_bank(ctx: Context<CreateCdpBank>) -> MarginfiResult {
    assert_eq!(ctx.accounts.mint.supply, 0, "Mint has existing supply");

    let mut cdp_bank = ctx.accounts.cdp_bank.load_init()?;

    *cdp_bank = CdpBank {
        group: ctx.accounts.group.key(),
        mint: ctx.accounts.mint.key(),
        mint_authority: ctx.accounts.mint_authority.key(),
        mint_authority_bump: *ctx.bumps.get("mint_authority").unwrap(),
        total_liability_shares: I80F48::ZERO.into(),
        liability_share_value: I80F48::ZERO.into(),
        liability_interest_rate: I80F48::ZERO.into(),
        liability_limit: 0,
    };

    Ok(())
}

/// TODO: Make `cdp_bank` unique per (group, mint) -- this should already be the case becasue `cdp_bank` owns the `mint_authority`,
/// but we should make it explicit
/// TODO: What to do with freeze authority, should be probably controlled by the CDP admin.
#[derive(Accounts)]
pub struct CreateCdpBank<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,
    #[account(mut, address = group.load()?.admin)]
    pub admin: Signer<'info>,
    #[account(mint::authority = mint_authority)]
    pub mint: Box<Account<'info, Mint>>,
    #[account(
        seeds = [
            CDP_MINT_AUTH_SEED.as_bytes(),
            mint.key().as_ref()
        ],
        bump
    )]
    pub mint_authority: AccountInfo<'info>,
    #[account(
        init,
        space = 8 + std::mem::size_of::<CdpBank>(),
        payer = admin,
    )]
    pub cdp_bank: AccountLoader<'info, CdpBank>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

pub fn create_cdp_collateral_bank(ctx: Context<CreateCdpCollateralBank>) -> MarginfiResult {
    let mut cdp_collateral_bank = ctx.accounts.cdp_collateral_bank.load_init()?;

    *cdp_collateral_bank = CdpCollateralBank {
        lending_bank: ctx.accounts.bank.key(),
        cdp_bank: ctx.accounts.cdp_bank.key(),
        status: CdpCollateralBankStatus::Disabled,
    };

    Ok(())
}

/// TODO: Make `cdp_collateral_bank` unique per (bank, cpd_bank)
#[derive(Accounts)]
pub struct CreateCdpCollateralBank<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,
    #[account(mut, address = group.load()?.admin)]
    pub admin: Signer<'info>,
    #[account(constraint = bank.load()?.group == group.key())]
    pub bank: AccountLoader<'info, Bank>,
    #[account(constraint = cdp_bank.load()?.group == group.key())]
    pub cdp_bank: AccountLoader<'info, CdpBank>,
    #[account(
        init,
        space = 8 + std::mem::size_of::<CdpCollateralBank>(),
        payer = admin,

    )]
    pub cdp_collateral_bank: AccountLoader<'info, CdpCollateralBank>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

pub fn create_cdp(ctx: Context<CreateCdp>) -> MarginfiResult {
    let cdp = ctx.accounts.cdp.load_init()?;

    *cdp = Cdp {
        authority: ctx.accounts.authority.key(),
        cdp_collateral_bank: ctx.accounts.cdp_collateral_bank.key(),
        collateral_shares: I80F48::ZERO.into(),
        liability_shares: I80F48::ZERO.into(),
        flags: 0,
    };

    Ok(())
}

#[derive(Accounts)]
pub struct CreateCdp<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: AccountInfo<'info>,
    pub cdp_collateral_bank: AccountLoader<'info, CdpCollateralBank>,
    #[account(
        init,
        payer = fee_payer,
        space = 8 + std::mem::size_of::<Cdp>(),
    )]
    pub cdp: AccountLoader<'info, Cdp>,
    pub system_program: Program<'info, System>,
}

pub fn cdp_deposit(ctx: Context<CdpDeposit>, amount: u64) -> MarginfiResult {
    let mut bank: RefMut<Bank> = ctx.accounts.bank.load_mut()?;

    bank.accrue_interest(Clock::get()?.unix_timestamp)?;

    let mut cdp: RefMut<Cdp> = ctx.accounts.cdp.load_mut()?;

    let deposit_shares = bank.get_asset_shares(I80F48::from_num(amount))?;

    bank.change_asset_shares(deposit_shares);
    cdp.change_collateral_shares(deposit_shares);

    bank.deposit_spl_transfer(
        amount,
        Transfer {
            from: ctx.accounts.cdp_authority_ta,
            to: ctx.accounts.bank_vault,
            authority: ctx.accounts.cdp_authority.to_account_info(),
        },
        ctx.accounts.token_program.to_account_info(),
    );

    Ok(())
}

#[derive(Accounts)]
pub struct CdpDeposit<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,
    pub cdp_bank: AccountLoader<'info, CdpBank>,
    pub cdp_collateral_bank: AccountLoader<'info, CdpCollateralBank>,
    pub bank: AccountLoader<'info, Bank>,
    pub bank_vault: AccountInfo<'info>,
    pub cdp: AccountLoader<'info, Cdp>,
    pub cdp_authority: Signer<'info>,
    pub cdp_authority_ta: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
