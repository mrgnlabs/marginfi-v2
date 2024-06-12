use crate::{
    constants::{INSURANCE_VAULT_AUTHORITY_SEED, LIQUID_INSURANCE_SEED},
    events::{LiquidInsuranceFundEventHeader, MarginfiCreateNewLiquidInsuranceFundEvent},
    state::{liquid_insurance_fund::LiquidInsuranceFund, marginfi_group::Bank},
    MarginfiGroup, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

#[derive(Accounts)]
pub struct CreateLiquidInsuranceFund<'info> {
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
            bank.key().as_ref(),
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
        address = bank.load()?.insurance_vault,
    )]
    pub lif_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_authority_bump
    )]
    pub lif_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

pub fn create_liquid_insurance_fund(
    ctx: Context<CreateLiquidInsuranceFund>,
    min_withdraw_period: i64,
) -> MarginfiResult {
    let CreateLiquidInsuranceFund {
        bank,
        lif_authority,
        liquid_insurance_fund,
        lif_vault,
        ..
    } = ctx.accounts;

    let bank_ = bank.load()?;
    let lif_vault_bump = bank_.insurance_vault_bump;
    let lif_authority_bump = bank_.insurance_vault_authority_bump;

    let mut lif = liquid_insurance_fund.load_init()?;

    lif.initialize(
        bank.key(),
        lif_authority.key(),
        min_withdraw_period,
        lif_vault_bump,
        lif_authority_bump,
        lif_vault.amount,
    );

    emit!(MarginfiCreateNewLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader { bank: lif.bank },
    });

    Ok(())
}
