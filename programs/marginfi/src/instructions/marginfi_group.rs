use crate::{
    bank_signer, check,
    constants::{
        FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, INSURANCE_VAULT_AUTHORITY_SEED,
        INSURANCE_VAULT_SEED, LENDING_POOL_BANK_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED,
        LIQUIDITY_VAULT_SEED,
    },
    prelude::MarginfiError,
    state::marginfi_group::{
        load_pyth_price_feed, Bank, BankConfig, BankConfigOpt, BankVaultType, GroupConfig,
        MarginfiGroup,
    },
    MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};

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
    bank_config: BankConfig,
) -> MarginfiResult {
    let LendingPoolAddBank {
        bank_mint,
        liquidity_vault,
        insurance_vault,
        fee_vault,
        pyth_oracle,
        ..
    } = ctx.accounts;

    let bank = &mut *ctx.accounts.bank;

    load_pyth_price_feed(pyth_oracle)?;

    *bank = Bank::new(
        bank_config,
        bank_mint.key(),
        liquidity_vault.key(),
        insurance_vault.key(),
        fee_vault.key(),
        Clock::get()?.unix_timestamp,
    );

    Ok(())
}

#[derive(Accounts)]
#[instruction(bank_config: BankConfig)]
pub struct LendingPoolAddBank<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    pub bank_mint: Box<Account<'info, Mint>>,

    /// PDA / seeds check ensures that provided account is legit, and use of the
    /// marginfi group + underlying mint guarantees unicity of bank per mint within a group
    #[account(
        init,
        space = 8 + std::mem::size_of::<Bank>(),
        payer = admin,
        seeds = [
            LENDING_POOL_BANK_SEED.as_bytes(),
            marginfi_group.key().as_ref(),
            bank_mint.key().as_ref(),
        ],
        bump
    )]
    pub bank: Account<'info, Bank>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    pub liquidity_vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = admin,
        token::mint = bank_mint,
        token::authority = liquidity_vault_authority,
        seeds = [
            LIQUIDITY_VAULT_SEED.as_bytes(),
            bank_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump,
    )]
    pub liquidity_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    pub insurance_vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = admin,
        token::mint = bank_mint,
        token::authority = insurance_vault_authority,
        seeds = [
            INSURANCE_VAULT_SEED.as_bytes(),
            bank_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump,
    )]
    pub insurance_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            FEE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    pub fee_vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = admin,
        token::mint = bank_mint,
        token::authority = fee_vault_authority,
        seeds = [
            FEE_VAULT_SEED.as_bytes(),
            bank_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump,
    )]
    pub fee_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(address = bank_config.pyth_oracle)]
    pub pyth_oracle: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

pub fn lending_pool_configure_bank(
    ctx: Context<LendingPoolConfigureBank>,
    bank_config: BankConfigOpt,
) -> MarginfiResult {
    let bank = &mut *ctx.accounts.bank;

    if let Some(pyth_oracle) = bank_config.pyth_oracle {
        check!(
            pyth_oracle == ctx.accounts.pyth_oracle.key(),
            MarginfiError::InvalidPythAccount
        );

        load_pyth_price_feed(&ctx.accounts.pyth_oracle)?;
    }

    bank.configure(bank_config)?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolConfigureBank<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    pub bank_mint: Box<Account<'info, Mint>>,

    #[account(
        seeds = [
            LENDING_POOL_BANK_SEED.as_bytes(),
            marginfi_group.key().as_ref(),
            bank_mint.key().as_ref(),
        ],
        bump
    )]
    pub bank: Box<Account<'info, Bank>>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    /// Set only if pyth oracle is being changed otherwise can be a random account.
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub pyth_oracle: UncheckedAccount<'info>,
}

pub fn lending_pool_bank_accrue_interest(
    ctx: Context<LendingPoolBankAccrueInterest>,
) -> MarginfiResult {
    let LendingPoolBankAccrueInterest {
        liquidity_vault_authority,
        insurance_vault,
        fee_vault,
        token_program,
        marginfi_group: marginfi_group_loader,
        liquidity_vault,
        ..
    } = ctx.accounts;

    let clock = Clock::get()?;
    let bank = &mut *ctx.accounts.bank;

    let (protocol_fee, insurance_fee) = bank.accrue_interest(&clock)?;

    let liq_vault_bump = *ctx.bumps.get("liquidity_vault_authority").unwrap();

    msg!("Protocol fee: {}", protocol_fee);

    bank.withdraw_spl_transfer(
        protocol_fee,
        Transfer {
            from: liquidity_vault.to_account_info(),
            to: fee_vault.to_account_info(),
            authority: liquidity_vault_authority.to_account_info(),
        },
        token_program.to_account_info(),
        bank_signer!(
            BankVaultType::Liquidity,
            bank.mint_pk.key(),
            marginfi_group_loader.key(),
            liq_vault_bump
        ),
    )?;

    msg!("Insurance fee: {}", insurance_fee);

    bank.withdraw_spl_transfer(
        insurance_fee,
        Transfer {
            from: liquidity_vault.to_account_info(),
            to: insurance_vault.to_account_info(),
            authority: liquidity_vault_authority.to_account_info(),
        },
        token_program.to_account_info(),
        bank_signer!(
            BankVaultType::Liquidity,
            bank.mint_pk.key(),
            marginfi_group_loader.key(),
            liq_vault_bump
        ),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolBankAccrueInterest<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    pub bank_mint: Box<Account<'info, Mint>>,

    /// PDA / seeds check ensures that provided account is legit, and use of the
    /// marginfi group + underlying mint guarantees unicity of bank per mint within a group
    #[account(
        seeds = [
            LENDING_POOL_BANK_SEED.as_bytes(),
            marginfi_group.key().as_ref(),
            bank_mint.key().as_ref(),
        ],
        bump
    )]
    pub bank: Box<Account<'info, Bank>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.mint_pk.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    pub liquidity_vault_authority: UncheckedAccount<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_SEED.as_bytes(),
            bank.mint_pk.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    pub liquidity_vault: UncheckedAccount<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            INSURANCE_VAULT_SEED.as_bytes(),
            bank.mint_pk.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    pub insurance_vault: UncheckedAccount<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            FEE_VAULT_SEED.as_bytes(),
            bank.mint_pk.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    pub fee_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
