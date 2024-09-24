// Adds a ASSET_TAG_STAKED type bank to a group with sane defaults. Used by validators to add their
// freshly-minted LST to a group so users can borrow SOL against it

// TODO should we support this for riskTier::Isolated too?

// TODO pick a hardcoded oracle

// TODO pick a hardcoded interest regmine

// TODO pick a hardcoded asset weight (~85%?) and `total_asset_value_init_limit`

// TODO pick a hardcoded max oracle age (~30s?)

// TODO pick a hardcoded initial deposit limit ()

// TODO should the group admin need to opt in to this functionality (configure the group)? We could
// also configure the key that assumes default admin here instead of using the group's admin
use crate::{
    constants::{
        ASSET_TAG_STAKED, FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, INSURANCE_VAULT_AUTHORITY_SEED,
        INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED,
    },
    events::{GroupEventHeader, LendingPoolBankCreateEvent},
    state::{
        marginfi_group::{
            Bank, BankConfigCompact, BankOperationalState, InterestRateConfig, MarginfiGroup,
            RiskTier,
        },
        price::OracleSetup,
    },
    MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::*;
use fixed_macro::types::I80F48;

pub fn lending_pool_add_bank_permissionless(
    ctx: Context<LendingPoolAddBankPermissionless>,
    _bank_seed: u64,
) -> MarginfiResult {
    let LendingPoolAddBankPermissionless {
        bank_mint,
        liquidity_vault,
        insurance_vault,
        fee_vault,
        bank: bank_loader,
        ..
    } = ctx.accounts;

    let mut bank = bank_loader.load_init()?;
    let group = ctx.accounts.marginfi_group.load()?;

    let liquidity_vault_bump = ctx.bumps.liquidity_vault;
    let liquidity_vault_authority_bump = ctx.bumps.liquidity_vault_authority;
    let insurance_vault_bump = ctx.bumps.insurance_vault;
    let insurance_vault_authority_bump = ctx.bumps.insurance_vault_authority;
    let fee_vault_bump = ctx.bumps.fee_vault;
    let fee_vault_authority_bump = ctx.bumps.fee_vault_authority;

    let default_ir_config = InterestRateConfig {
        optimal_utilization_rate: I80F48!(0.4).into(),
        plateau_interest_rate: I80F48!(0.4).into(),
        protocol_fixed_fee_apr: I80F48!(0.01).into(),
        max_interest_rate: I80F48!(3).into(),
        insurance_ir_fee: I80F48!(0.1).into(),
        ..Default::default()
    };

    let default_config: BankConfigCompact = BankConfigCompact {
        asset_weight_init: I80F48!(0.5).into(),
        asset_weight_maint: I80F48!(0.75).into(),
        liability_weight_init: I80F48!(1.5).into(),
        liability_weight_maint: I80F48!(1.25).into(),
        deposit_limit: 42,
        interest_rate_config: default_ir_config.into(),
        operational_state: BankOperationalState::Operational,
        oracle_setup: OracleSetup::PythLegacy,
        oracle_key: Pubkey::new_unique(),
        borrow_limit: 0,
        risk_tier: RiskTier::Collateral,
        asset_tag: ASSET_TAG_STAKED,
        _pad0: [0; 6],
        total_asset_value_init_limit: 42,
        oracle_max_age: 10,
    };

    *bank = Bank::new(
        ctx.accounts.marginfi_group.key(),
        default_config.into(),
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
            signer: Some(group.admin)
        },
        bank: bank_loader.key(),
        mint: bank_mint.key(),
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(bank_seed: u64)]
pub struct LendingPoolAddBankPermissionless<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub bank_mint: Box<InterfaceAccount<'info, Mint>>,

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
    pub liquidity_vault: Box<InterfaceAccount<'info, TokenAccount>>,

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
    pub insurance_vault: Box<InterfaceAccount<'info, TokenAccount>>,

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
    pub fee_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub rent: Sysvar<'info, Rent>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
