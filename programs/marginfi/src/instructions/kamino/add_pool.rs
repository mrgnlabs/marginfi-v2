// Adds a Kamino type bank to a group with sane defaults. Used to integrate with Kamino
// allowing users to interact with Kamino pools through marginfi
use crate::{
    constants::{
        FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, INSURANCE_VAULT_AUTHORITY_SEED,
        INSURANCE_VAULT_SEED, KAMINO_PROGRAM_ID, LIQUIDITY_VAULT_AUTHORITY_SEED,
        LIQUIDITY_VAULT_SEED,
    },
    events::{GroupEventHeader, LendingPoolBankCreateEvent},
    log_pool_info,
    state::{
        kamino::KaminoConfigCompact,
        marginfi_group::{Bank, MarginfiGroup},
        price::OracleSetup,
    },
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;
use anchor_spl::token_interface::*;
use kamino_mocks::state::MinimalReserve;

/// Add a Kamino bank to the marginfi lending pool
pub fn lending_pool_add_bank_kamino(
    ctx: Context<LendingPoolAddBankKamino>,
    bank_config: KaminoConfigCompact,
    _bank_seed: u64,
) -> MarginfiResult {
    // Note: Kamino banks don't need to debit the flat SOL fee because these will always be
    // first-party pools owned by mrgn and never permissionless pools
    let LendingPoolAddBankKamino {
        bank_mint,
        bank: bank_loader,
        kamino_reserve: reserve_loader,
        kamino_obligation,
        ..
    } = ctx.accounts;

    let mut bank = bank_loader.load_init()?;
    let mut group = ctx.accounts.group.load_mut()?;
    let reserve_key = reserve_loader.key();
    let obligation_key = kamino_obligation.key();

    // Validate that we're using a supported Kamino oracle setup type
    require!(
        matches!(
            bank_config.oracle_setup,
            OracleSetup::KaminoPythPush | OracleSetup::KaminoSwitchboardPull
        ),
        MarginfiError::KaminoInvalidOracleSetup
    );

    let config = bank_config.to_bank_config(reserve_key);

    let liquidity_vault_bump = ctx.bumps.liquidity_vault;
    let liquidity_vault_authority_bump = ctx.bumps.liquidity_vault_authority;
    let insurance_vault_bump = ctx.bumps.insurance_vault;
    let insurance_vault_authority_bump = ctx.bumps.insurance_vault_authority;
    let fee_vault_bump = ctx.bumps.fee_vault;
    let fee_vault_authority_bump = ctx.bumps.fee_vault_authority;

    *bank = Bank::new(
        ctx.accounts.group.key(),
        config, // Use the modified BankConfig directly instead of converting from BankConfigCompact
        bank_mint.key(),
        bank_mint.decimals,
        ctx.accounts.liquidity_vault.key(),
        ctx.accounts.insurance_vault.key(),
        ctx.accounts.fee_vault.key(),
        Clock::get().unwrap().unix_timestamp,
        liquidity_vault_bump,
        liquidity_vault_authority_bump,
        insurance_vault_bump,
        insurance_vault_authority_bump,
        fee_vault_bump,
        fee_vault_authority_bump,
    );

    bank.kamino_reserve = reserve_key;
    bank.kamino_obligation = obligation_key;

    log_pool_info(&bank);

    group.add_bank()?;

    bank.config.validate()?;
    bank.config
        .validate_oracle_setup(ctx.remaining_accounts, None, None, None)?;

    emit!(LendingPoolBankCreateEvent {
        header: GroupEventHeader {
            marginfi_group: ctx.accounts.group.key(),
            signer: Some(group.admin)
        },
        bank: bank_loader.key(),
        mint: bank_mint.key(),
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(bank_config: KaminoConfigCompact, bank_seed: u64)]
pub struct LendingPoolAddBankKamino<'info> {
    #[account(
        mut,
        has_one = admin
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    pub admin: Signer<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Must match the mint used by `kamino_reserve`, Kamino calls this the `reserve_liquidity_mint`
    /// aka `liquidity.mint_pubkey`
    pub bank_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        space = 8 + std::mem::size_of::<Bank>(),
        payer = fee_payer,
        seeds = [
            group.key().as_ref(),
            bank_mint.key().as_ref(),
            &bank_seed.to_le_bytes(),
        ],
        bump,
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        constraint = kamino_reserve.load()?.mint_pubkey == bank_mint.key()
            @ MarginfiError::KaminoReserveMintAddressMismatch,
    )]
    pub kamino_reserve: AccountLoader<'info, MinimalReserve>,

    /// Note: not yet initialized in this instruction, run `init_obligation` after.
    #[account(
        seeds = [
            &[0u8],
            &[0u8],
            liquidity_vault_authority.key().as_ref(),
            kamino_reserve.load()?.lending_market.as_ref(),
            system_program::ID.as_ref(),
            system_program::ID.as_ref()
        ],
        bump,
        seeds::program = KAMINO_PROGRAM_ID
    )]
    pub kamino_obligation: SystemAccount<'info>,

    /// Will be authority of the bank's `kamino_obligation`. Note: When depositing/withdrawing
    /// Kamino assets, the source/destination must also be owned by the obligation authority. This
    /// account owns the `liquidity_vault`, and thus acts as intermediary for deposits/withdraws
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub liquidity_vault_authority: SystemAccount<'info>,

    /// For Kamino banks, the `liquidity_vault` never holds assets, but is instead used as an
    /// intermediary when depositing/withdrawing, e.g., withdrawn funds move from Kamino -> here ->
    /// the user's token account.
    #[account(
        init,
        payer = fee_payer,
        seeds = [
            LIQUIDITY_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
        token::mint = bank_mint,
        token::authority = liquidity_vault_authority,
    )]
    pub liquidity_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Note: Currently does nothing.
    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub insurance_vault_authority: SystemAccount<'info>,

    /// Note: Currently does nothing.
    #[account(
        init,
        payer = fee_payer,
        seeds = [
            INSURANCE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
        token::mint = bank_mint,
        token::authority = insurance_vault_authority,
    )]
    pub insurance_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Note: Currently does nothing.
    #[account(
        seeds = [
            FEE_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub fee_vault_authority: SystemAccount<'info>,

    /// Note: Currently does nothing.
    #[account(
        init,
        payer = fee_payer,
        seeds = [
            FEE_VAULT_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump,
        token::mint = bank_mint,
        token::authority = fee_vault_authority,
    )]
    pub fee_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
