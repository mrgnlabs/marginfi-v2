use crate::{
    constants::{
        FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, INSURANCE_VAULT_AUTHORITY_SEED,
        INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED,
    },
    events::{GroupEventHeader, LendingPoolBankCreateEvent},
    state::marginfi_group::{Bank, BankConfig, BankConfigCompact, MarginfiGroup},
    MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

/// Add a bank to the lending pool
///
/// Admin only
///
/// TODO: Allow for different oracle configurations
pub fn lending_pool_add_bank(
    ctx: Context<LendingPoolAddBank>,
    bank_config: BankConfig,
) -> MarginfiResult {
    let LendingPoolAddBank {
        bank_mint,
        liquidity_vault,
        insurance_vault,
        fee_vault,
        bank: bank_loader,
        ..
    } = ctx.accounts;

    let mut bank = bank_loader.load_init()?;

    let liquidity_vault_bump = *ctx.bumps.get("liquidity_vault").unwrap();
    let liquidity_vault_authority_bump = *ctx.bumps.get("liquidity_vault_authority").unwrap();
    let insurance_vault_bump = *ctx.bumps.get("insurance_vault").unwrap();
    let insurance_vault_authority_bump = *ctx.bumps.get("insurance_vault_authority").unwrap();
    let fee_vault_bump = *ctx.bumps.get("fee_vault").unwrap();
    let fee_vault_authority_bump = *ctx.bumps.get("fee_vault_authority").unwrap();

    *bank = Bank::new(
        ctx.accounts.marginfi_group.key(),
        bank_config,
        bank_mint.key(),
        bank_mint.decimals,
        liquidity_vault.key(),
        insurance_vault.key(),
        fee_vault.key(),
        Clock::get().unwrap().unix_timestamp,
        liquidity_vault_bump,
        liquidity_vault_authority_bump,
        insurance_vault_bump,
        insurance_vault_authority_bump,
        fee_vault_bump,
        fee_vault_authority_bump,
    );

    bank.config.validate()?;
    bank.config.validate_oracle_setup(ctx.remaining_accounts)?;

    emit!(LendingPoolBankCreateEvent {
        header: GroupEventHeader {
            marginfi_group: ctx.accounts.marginfi_group.key(),
            signer: Some(*ctx.accounts.admin.key)
        },
        bank: bank_loader.key(),
        mint: bank_mint.key(),
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(bank_config: BankConfigCompact)]
pub struct LendingPoolAddBank<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub bank_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        space = 8 + std::mem::size_of::<Bank>(),
        payer = fee_payer,
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump
    )]
    pub liquidity_vault_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        token::mint = bank_mint,
        token::authority = liquidity_vault_authority,
        seeds = [
            LIQUIDITY_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub liquidity_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump
    )]
    pub insurance_vault_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        token::mint = bank_mint,
        token::authority = insurance_vault_authority,
        seeds = [
            INSURANCE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub insurance_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            FEE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump
    )]
    pub fee_vault_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        token::mint = bank_mint,
        token::authority = fee_vault_authority,
        seeds = [
            FEE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub fee_vault: Box<Account<'info, TokenAccount>>,

    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// A copy of lending_pool_add_bank but with an additional bank seed provided.
/// This seed is used by the LendingPoolAddBankWithSeed.bank to generate a
/// PDA account to sign for newly added bank transactions securely.
/// The previous lending_pool_add_bank is preserved for backwards-compatibility.
pub fn lending_pool_add_bank_with_seed(
    ctx: Context<LendingPoolAddBankWithSeed>,
    bank_config: BankConfig,
    bank_seed: u64,
) -> MarginfiResult {
    let LendingPoolAddBankWithSeed {
        bank_mint,
        liquidity_vault,
        insurance_vault,
        fee_vault,
        bank: bank_loader,
        ..
    } = ctx.accounts;

    let mut bank = bank_loader.load_init()?;

    let liquidity_vault_bump = *ctx.bumps.get("liquidity_vault").unwrap();
    let liquidity_vault_authority_bump = *ctx.bumps.get("liquidity_vault_authority").unwrap();
    let insurance_vault_bump = *ctx.bumps.get("insurance_vault").unwrap();
    let insurance_vault_authority_bump = *ctx.bumps.get("insurance_vault_authority").unwrap();
    let fee_vault_bump = *ctx.bumps.get("fee_vault").unwrap();
    let fee_vault_authority_bump = *ctx.bumps.get("fee_vault_authority").unwrap();

    *bank = Bank::new(
        ctx.accounts.marginfi_group.key(),
        bank_config,
        bank_mint.key(),
        bank_mint.decimals,
        liquidity_vault.key(),
        insurance_vault.key(),
        fee_vault.key(),
        Clock::get().unwrap().unix_timestamp,
        liquidity_vault_bump,
        liquidity_vault_authority_bump,
        insurance_vault_bump,
        insurance_vault_authority_bump,
        fee_vault_bump,
        fee_vault_authority_bump,
    );

    bank.config.validate()?;
    bank.config.validate_oracle_setup(ctx.remaining_accounts)?;

    emit!(LendingPoolBankCreateEvent {
        header: GroupEventHeader {
            marginfi_group: ctx.accounts.marginfi_group.key(),
            signer: Some(*ctx.accounts.admin.key)
        },
        bank: bank_loader.key(),
        mint: bank_mint.key(),
    });

    Ok(())
}

/// A copy of LendingPoolAddBank but with an additional bank seed provided.
/// This seed is used by the LendingPoolAddBankWithSeed.bank to generate a
/// PDA account to sign for newly added bank transactions securely.
/// The previous LendingPoolAddBank is preserved for backwards-compatibility.
#[derive(Accounts)]
#[instruction(bank_config: BankConfigCompact, bank_seed: u64)]
pub struct LendingPoolAddBankWithSeed<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub bank_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        space = 8 + std::mem::size_of::<Bank>(),
        payer = fee_payer,
        seeds = [
            marginfi_group.key().as_ref(),
            bank_mint.key().as_ref(),
            &bank_seed.to_le_bytes(),
        ],
        bump,
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑a
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump
    )]
    pub liquidity_vault_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        token::mint = bank_mint,
        token::authority = liquidity_vault_authority,
        seeds = [
            LIQUIDITY_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub liquidity_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump
    )]
    pub insurance_vault_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        token::mint = bank_mint,
        token::authority = insurance_vault_authority,
        seeds = [
            INSURANCE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub insurance_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            FEE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump
    )]
    pub fee_vault_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        token::mint = bank_mint,
        token::authority = fee_vault_authority,
        seeds = [
            FEE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub fee_vault: Box<Account<'info, TokenAccount>>,

    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
