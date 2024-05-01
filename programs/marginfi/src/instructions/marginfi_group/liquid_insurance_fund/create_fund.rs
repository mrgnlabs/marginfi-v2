use crate::{
    constants::{
        INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUID_INSURANCE_MINT_AUTHORITY_SEED,
        LIQUID_INSURANCE_MINT_METADATA_SEED, LIQUID_INSURANCE_MINT_SEED,
    },
    events::{LiquidInsuranceFundEventHeader, MarginfiCreateNewLiquidInsuranceFundEvent},
    state::{liquid_insurance_fund::LiquidInsuranceFund, marginfi_group::Bank},
    MarginfiGroup, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use mpl_token_metadata::state::DataV2;

use anchor_spl::metadata::{create_metadata_accounts_v3, CreateMetadataAccountsV3, Metadata};

#[derive(Accounts)]
#[instruction(
    params: InitMintParams,
    min_withdraw_period: u64,
)]
pub struct CreateNewLiquidInsuranceFund<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        address = marginfi_group.load()?.admin,
    )]
    pub marginfi_authority: Signer<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init,
        space = 8 + std::mem::size_of::<LiquidInsuranceFund>(),
        payer = signer,
        seeds = [
            marginfi_group.key().as_ref(),
            bank.load()?.mint.key().as_ref(),
        ],
        bump,
    )]
    pub liquid_insurance_fund: AccountLoader<'info, LiquidInsuranceFund>,

    #[account(
        constraint = bank.load()?.mint == bank_insurance_fund_mint.key(),
    )]
    pub bank_insurance_fund_mint: Account<'info, Mint>,

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

    /// The corresponding mint that the liquid insurance fund is offering.
    #[account(
        init,
        payer = signer,
        mint::authority = mint,
        mint::freeze_authority = mint,
        mint::decimals = bank.load()?.mint_decimals,
        seeds = [
            LIQUID_INSURANCE_MINT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        seeds = [
            LIQUID_INSURANCE_MINT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump
    )]
    pub mint_authority: AccountInfo<'info>,

    /// CHECK: New Metadata Account being created
    #[account(
        init,
        payer = signer,
        space = std::mem::size_of::<Metadata>(),
        seeds = [
            LIQUID_INSURANCE_MINT_METADATA_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub metadata: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct InitMintParams {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub decimals: u8,
}

pub fn create_new_liquid_insurance_fund(
    ctx: Context<CreateNewLiquidInsuranceFund>,
    mint_metadata: InitMintParams,
    min_withdraw_period: i64,
    initial_number_of_shares: Option<u64>,
) -> MarginfiResult {
    let CreateNewLiquidInsuranceFund {
        bank,
        bank_insurance_vault_authority,
        mint,
        mint_authority,
        ..
    } = ctx.accounts;

    let liquid_insurance_mint_bump = *ctx.bumps.get(LIQUID_INSURANCE_MINT_SEED).unwrap();
    let liquid_insurance_mint_authority_bump =
        *ctx.bumps.get(LIQUID_INSURANCE_MINT_AUTHORITY_SEED).unwrap();

    let current_timestamp = Clock::get()?.unix_timestamp;

    let liquid_insurance_fund = LiquidInsuranceFund::new(
        ctx.accounts.marginfi_group.key(),
        ctx.accounts.marginfi_authority.key(),
        ctx.accounts.bank.key(),
        ctx.accounts.bank_insurance_vault.key(),
        ctx.accounts.bank_insurance_vault_authority.key(),
        ctx.accounts.mint.key(),
        ctx.accounts.mint_authority.key(),
        ctx.accounts.metadata.key(),
        liquid_insurance_mint_bump,
        liquid_insurance_mint_authority_bump,
        min_withdraw_period,
        current_timestamp,
        ctx.accounts.bank_insurance_vault.amount,
        initial_number_of_shares,
    );

    // Next: Create the mint metadata account for the mint associated with the liquid insurance fund
    let token_data: DataV2 = DataV2 {
        name: mint_metadata.name,
        symbol: mint_metadata.symbol,
        uri: mint_metadata.uri,
        seller_fee_basis_points: 0,
        creators: None,
        collection: None,
        uses: None,
    };

    let metadata_ctx = CpiContext::new(
        ctx.accounts.token_metadata_program.to_account_info(),
        CreateMetadataAccountsV3 {
            payer: ctx.accounts.signer.to_account_info(),
            update_authority: ctx.accounts.mint.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            metadata: ctx.accounts.metadata.to_account_info(),
            mint_authority: ctx.accounts.mint_authority.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        },
    );

    create_metadata_accounts_v3(metadata_ctx, token_data, false, true, None)?;

    emit!(MarginfiCreateNewLiquidInsuranceFundEvent {
        header: LiquidInsuranceFundEventHeader {
            bank: liquid_insurance_fund.bank,
            bank_insurance_vault: liquid_insurance_fund.bank_insurance_vault,
            token_mint: liquid_insurance_fund.mint
        },
        mint_metadata_account: liquid_insurance_fund.mint_metadata,
    });

    Ok(())
}
