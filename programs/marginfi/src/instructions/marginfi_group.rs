use crate::{
    bank_signer, check,
    constants::{
        FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, INSURANCE_VAULT_AUTHORITY_SEED,
        INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED,
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
    bank_index: u16,
    bank_config: BankConfig,
) -> MarginfiResult {
    let LendingPoolAddBank {
        asset_mint,
        liquidity_vault,
        insurance_vault,
        fee_vault,
        marginfi_group,
        pyth_oracle,
        ..
    } = ctx.accounts;

    let mut marginfi_group = marginfi_group.load_mut()?;

    check!(
        marginfi_group.lending_pool.banks[bank_index as usize].is_none(),
        MarginfiError::BankAlreadyExists
    );

    load_pyth_price_feed(pyth_oracle)?;

    let bank = Bank::new(
        bank_config,
        asset_mint.key(),
        liquidity_vault.key(),
        insurance_vault.key(),
        fee_vault.key(),
        Clock::get()?.unix_timestamp,
    );

    marginfi_group.lending_pool.banks[bank_index as usize] = Some(bank);

    Ok(())
}

#[derive(Accounts)]
#[instruction(bank_index: u16, bank_config: BankConfig)]
pub struct LendingPoolAddBank<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(
        mut,
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,
    pub asset_mint: Box<Account<'info, Mint>>,
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED,
            asset_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub liquidity_vault_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        token::mint = asset_mint,
        token::authority = liquidity_vault_authority,
        seeds = [
            LIQUIDITY_VAULT_SEED,
            asset_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump,
    )]
    pub liquidity_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED,
            asset_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub insurance_vault_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        token::mint = asset_mint,
        token::authority = insurance_vault_authority,
        seeds = [
            INSURANCE_VAULT_SEED,
            asset_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump,
    )]
    pub insurance_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        seeds = [
            FEE_VAULT_AUTHORITY_SEED,
            asset_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub fee_vault_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        token::mint = asset_mint,
        token::authority = fee_vault_authority,
        seeds = [
            FEE_VAULT_SEED,
            asset_mint.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump,
    )]
    pub fee_vault: Box<Account<'info, TokenAccount>>,
    #[account(address = bank_config.pyth_oracle)]
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub pyth_oracle: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn lending_pool_configure_bank(
    ctx: Context<LendingPoolConfigureBank>,
    bank_index: u16,
    bank_config: BankConfigOpt,
) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_mut()?;

    let mut bank = marginfi_group
        .lending_pool
        .banks
        .get_mut(bank_index as usize)
        .expect("Bank index out of bounds")
        .ok_or(MarginfiError::BankNotFound)?;

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
    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,
    /// Set only if pyth oracle is being changed otherwise can be a random account.
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub pyth_oracle: UncheckedAccount<'info>,
}

pub fn interest(ctx: Context<Interest>, bank_index: u16) -> MarginfiResult {
    let Interest {
        liquidity_vault_authority,
        insurance_vault,
        fee_vault,
        token_program,
        marginfi_group: marginfi_group_loader,
        liquidity_vault,
    } = &ctx.accounts;

    let clock = Clock::get()?;
    let mut marginfi_group = marginfi_group_loader.load_mut()?;
    let bank = marginfi_group
        .lending_pool
        .get_initialized_bank_mut(bank_index)?;

    let (protocol_fee, insurance_fee) = bank.accrue_interest(&clock)?;

    let liq_vault_bump = *ctx.bumps.get("bank_liquidity_vault_authority").unwrap();

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
#[instruction(bank_index: u16)]
pub struct Interest<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED,
            marginfi_group.load()?.lending_pool.banks[bank_index as usize].unwrap().mint_pk.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub liquidity_vault_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_SEED,
            marginfi_group.load()?.lending_pool.banks[bank_index as usize].unwrap().mint_pk.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub liquidity_vault: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [
            INSURANCE_VAULT_SEED,
            marginfi_group.load()?.lending_pool.banks[bank_index as usize].unwrap().mint_pk.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub insurance_vault: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [
            FEE_VAULT_SEED,
            marginfi_group.load()?.lending_pool.banks[bank_index as usize].unwrap().mint_pk.key().as_ref(),
            marginfi_group.key().as_ref(),
        ],
        bump
    )]
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub fee_vault: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}
