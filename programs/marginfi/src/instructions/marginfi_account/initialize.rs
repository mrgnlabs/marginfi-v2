use crate::state::marginfi_account::MarginfiAccountImpl;
use crate::{
    constants::MOCKS_PROGRAM_ID,
    events::{AccountEventHeader, MarginfiAccountCreateEvent},
    prelude::*,
};
use anchor_lang::prelude::*;
use marginfi_type_crate::{
    constants::MARGINFI_ACCOUNT_SEED,
    types::{MarginfiAccount, MarginfiGroup},
};

use anchor_lang::solana_program::sysvar::instructions as ix_sysvar;
use anchor_lang::solana_program::sysvar::instructions::load_instruction_at_checked;

fn is_cpi_from_mocks_program(sysvar_info: &AccountInfo) -> MarginfiResult<bool> {
    let current_ix_index = ix_sysvar::load_current_index_checked(sysvar_info)?;

    // Get the current (top-level) instruction
    let current_ixn = load_instruction_at_checked(current_ix_index as usize, sysvar_info)?;

    // The current instruction must match the marginfi program. If it doesn't, it's a CPI.
    if current_ixn.program_id != crate::ID {
        return Ok(current_ixn.program_id == MOCKS_PROGRAM_ID);
    }

    // Direct call (not CPI)
    Ok(false)
}

pub fn initialize_account(ctx: Context<MarginfiAccountInitialize>) -> MarginfiResult {
    let MarginfiAccountInitialize {
        authority,
        marginfi_group,
        marginfi_account: marginfi_account_loader,
        ..
    } = ctx.accounts;

    let mut marginfi_account = marginfi_account_loader.load_init()?;

    marginfi_account.initialize(marginfi_group.key(), authority.key());

    emit!(MarginfiAccountCreateEvent {
        header: AccountEventHeader {
            signer: Some(authority.key()),
            marginfi_account: marginfi_account_loader.key(),
            marginfi_account_authority: marginfi_account.authority,
            marginfi_group: marginfi_account.group,
        }
    });

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiAccountInitialize<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + std::mem::size_of::<MarginfiAccount>()
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Initialize a marginfi account using a PDA (Program Derived Address)
///
/// This function creates a marginfi account at a deterministic address based on:
/// - marginfi_group: The group this account belongs to
/// - authority: The account authority (owner)  
/// - account_index: A u32 value to allow multiple accounts per authority
/// - third_party_id: Optional u32 for third-party tagging (id=42 restricted to mocks program CPI at 5XaaR94jBubdbrRrNW7DtRvZeWvLhSHkEGU3jHTEXV3C)
///
/// PDA seeds: [b"marginfi_account", group, authority, account_index.to_le_bytes(), third_party_id.unwrap_or(0).to_le_bytes()]
pub fn initialize_account_pda(
    ctx: Context<MarginfiAccountInitializePda>,
    _account_index: u32,
    third_party_id: Option<u32>,
) -> MarginfiResult {
    let MarginfiAccountInitializePda {
        authority,
        marginfi_group,
        marginfi_account: marginfi_account_loader,
        ..
    } = ctx.accounts;

    if let Some(id) = third_party_id {
        if id == 42 {
            // Restrict id=42 to CPI calls from the mocks program only
            if !is_cpi_from_mocks_program(&ctx.accounts.instructions_sysvar)? {
                return err!(MarginfiError::Unauthorized);
            }
        }
    }

    let mut marginfi_account = marginfi_account_loader.load_init()?;

    marginfi_account.initialize(marginfi_group.key(), authority.key());

    emit!(MarginfiAccountCreateEvent {
        header: AccountEventHeader {
            signer: Some(authority.key()),
            marginfi_account: marginfi_account_loader.key(),
            marginfi_account_authority: marginfi_account.authority,
            marginfi_group: marginfi_account.group,
        }
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(account_index: u32, third_party_id: Option<u32>)]
pub struct MarginfiAccountInitializePda<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + std::mem::size_of::<MarginfiAccount>(),
        seeds = [
            MARGINFI_ACCOUNT_SEED.as_bytes(),
            marginfi_group.key().as_ref(),
            authority.key().as_ref(),
            &account_index.to_le_bytes(),
            &third_party_id.unwrap_or(0).to_le_bytes(),
        ],
        bump
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    /// CHECK: Authority is only used for PDA seed derivation, no signing required
    pub authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Instructions sysvar for CPI validation
    /// CHECK: Standard sysvar account
    #[account(address = anchor_lang::solana_program::sysvar::instructions::id())]
    pub instructions_sysvar: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}
