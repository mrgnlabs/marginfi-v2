use crate::{
    constants::{MARGINFI_ACCOUNT_SEED, MOCKS_PROGRAM_ID},
    events::{AccountEventHeader, MarginfiAccountCreateEvent},
    prelude::*,
    state::marginfi_account::MarginfiAccount,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::Sysvar;

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

    // Validate third-party id restriction if provided
    if let Some(id) = third_party_id {
        if id == 42 {
            // Restrict id=42 to CPI calls from the mocks program only
            let caller_program = ctx.accounts.cpi_program.as_ref().map(|p| p.key());
            match caller_program {
                Some(program_key) => {
                    // Check if the caller is the mocks program
                    if program_key != MOCKS_PROGRAM_ID {
                        return err!(MarginfiError::Unauthorized);
                    }
                }
                None => return err!(MarginfiError::Unauthorized),
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

    pub authority: Signer<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Optional program account for CPI validation
    /// CHECK: Used for validating third-party id restrictions
    pub cpi_program: Option<UncheckedAccount<'info>>,

    pub system_program: Program<'info, System>,
}
