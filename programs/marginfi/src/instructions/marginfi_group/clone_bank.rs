use crate::{
    check,
    events::{GroupEventHeader, LendingPoolBankCreateEvent},
    log_pool_info,
    state::{bank::BankImpl, bank_config::BankConfigImpl, marginfi_group::MarginfiGroupImpl},
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::*;
use bytemuck::from_bytes;
use marginfi_type_crate::{
    constants::{
        FEE_STATE_SEED, FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, INSURANCE_VAULT_AUTHORITY_SEED,
        INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED,
    },
    types::{Bank, FeeState, MarginfiGroup},
};

const MAINNET_PROGRAM_ID: Pubkey = pubkey!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");

pub fn lending_pool_clone_bank(
    ctx: Context<LendingPoolCloneBank>,
    bank_seed: u64,
) -> MarginfiResult {
    if crate::ID == MAINNET_PROGRAM_ID {
        panic!("clone bank cannot run on mainnet deployment");
    }

    let _ = bank_seed;

    let fee_state = ctx.accounts.fee_state.load()?;
    let bank_init_flat_sol_fee = fee_state.bank_init_flat_sol_fee;
    drop(fee_state);

    if bank_init_flat_sol_fee > 0 {
        anchor_lang::system_program::transfer(
            ctx.accounts.transfer_flat_fee(),
            bank_init_flat_sol_fee as u64,
        )?;
    }

    let (
        source_mint,
        source_mint_decimals,
        source_config,
        source_flags,
        source_emissions_rate,
        source_emissions_remaining,
        source_emissions_mint,
        source_emode,
        source_fees_destination_account,
    ) = {
        let source_bank_info = ctx.accounts.source_bank.as_ref();

        check!(
            *source_bank_info.owner == MAINNET_PROGRAM_ID,
            MarginfiError::InvalidBankAccount
        );

        let data = source_bank_info
            .try_borrow_data()
            .map_err(|_| error!(MarginfiError::InvalidBankAccount))?;

        check!(
            data.len() >= 8 + std::mem::size_of::<Bank>(),
            MarginfiError::InvalidBankAccount
        );
        check!(
            data[..8] == Bank::DISCRIMINATOR,
            MarginfiError::InvalidBankAccount
        );

        let bank_data = &data[8..8 + std::mem::size_of::<Bank>()];
        let source_bank = from_bytes::<Bank>(bank_data);

        (
            source_bank.mint,
            source_bank.mint_decimals,
            source_bank.config,
            source_bank.flags,
            source_bank.emissions_rate,
            source_bank.emissions_remaining,
            source_bank.emissions_mint,
            source_bank.emode,
            source_bank.fees_destination_account,
        )
    };

    check!(
        ctx.accounts.bank_mint.key() == source_mint,
        MarginfiError::InvalidBankAccount
    );
    check!(
        ctx.accounts.bank_mint.decimals == source_mint_decimals,
        MarginfiError::InvalidBankAccount
    );

    let liquidity_vault_bump = ctx.bumps.liquidity_vault;
    let liquidity_vault_authority_bump = ctx.bumps.liquidity_vault_authority;
    let insurance_vault_bump = ctx.bumps.insurance_vault;
    let insurance_vault_authority_bump = ctx.bumps.insurance_vault_authority;
    let fee_vault_bump = ctx.bumps.fee_vault;
    let fee_vault_authority_bump = ctx.bumps.fee_vault_authority;

    let mut bank = ctx.accounts.bank.load_init()?;
    *bank = Bank::new(
        ctx.accounts.marginfi_group.key(),
        source_config,
        source_mint,
        source_mint_decimals,
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

    bank.flags = source_flags;
    bank.emissions_rate = source_emissions_rate;
    bank.emissions_remaining = source_emissions_remaining;
    bank.emissions_mint = source_emissions_mint;
    bank.emode = source_emode;
    bank.fees_destination_account = source_fees_destination_account;

    log_pool_info(&bank);

    let mut group = ctx.accounts.marginfi_group.load_mut()?;
    group.add_bank()?;

    bank.config.validate()?;
    bank.config.validate_oracle_age()?;

    emit!(LendingPoolBankCreateEvent {
        header: GroupEventHeader {
            marginfi_group: ctx.accounts.marginfi_group.key(),
            signer: Some(*ctx.accounts.admin.key)
        },
        bank: ctx.accounts.bank.key(),
        mint: ctx.accounts.bank_mint.key(),
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(bank_seed: u64)]
pub struct LendingPoolCloneBank<'info> {
    #[account(
        mut,
        has_one = admin
    )]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    #[account(
        seeds = [FEE_STATE_SEED.as_bytes()],
        bump,
        has_one = global_fee_wallet
    )]
    pub fee_state: AccountLoader<'info, FeeState>,

    /// CHECK: The fee admin's native SOL wallet, validated against fee state
    #[account(mut)]
    pub global_fee_wallet: AccountInfo<'info>,

    pub bank_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: Source bank to clone from another program
    pub source_bank: UncheckedAccount<'info>,

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

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> LendingPoolCloneBank<'info> {
    fn transfer_flat_fee(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, anchor_lang::system_program::Transfer<'info>> {
        CpiContext::new(
            self.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: self.fee_payer.to_account_info(),
                to: self.global_fee_wallet.to_account_info(),
            },
        )
    }
}
