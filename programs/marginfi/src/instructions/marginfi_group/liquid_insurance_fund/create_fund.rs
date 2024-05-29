use crate::{
    constants::{INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUID_INSURANCE_SEED},
    events::{LiquidInsuranceFundEventHeader, MarginfiCreateNewLiquidInsuranceFundEvent},
    state::{liquid_insurance_fund::LiquidInsuranceFund, marginfi_group::Bank},
    MarginfiGroup, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
#[instruction(
    min_withdraw_period: u64,
)]
pub struct CreateNewLiquidInsuranceFund<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        address = marginfi_group.load()?.admin,
    )]
    pub signer: Signer<'info>,

    #[account(
        init,
        space = 8 + std::mem::size_of::<LiquidInsuranceFund>(),
        payer = signer,
        seeds = [
            LIQUID_INSURANCE_SEED.as_bytes(),
            bank.load()?.insurance_vault.key().as_ref(),
        ],
        bump,
    )]
    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

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

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_authority_bump
    )]
    pub bank_insurance_vault_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

pub fn create_new_liquid_insurance_fund(
    ctx: Context<CreateNewLiquidInsuranceFund>,
    min_withdraw_period: i64,
) -> MarginfiResult {
    let CreateNewLiquidInsuranceFund {
        bank,
        bank_insurance_vault_authority,
        ..
    } = ctx.accounts;

    let lif_bump = *ctx.bumps.get(LIQUID_INSURANCE_SEED).unwrap();

    let current_timestamp = Clock::get()?.unix_timestamp;

    let liquid_insurance_fund = LiquidInsuranceFund::new(
        ctx.accounts.bank.key(),
        min_withdraw_period,
        current_timestamp,
        lif_bump,
    );

    emit!(MarginfiCreateNewLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund.bank,
        },
    });

    Ok(())
}
