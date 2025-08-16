use crate::{
    check, check_eq,
    constants::{ACCOUNT_TRANSFER_FEE, MARGINFI_ACCOUNT_SEED, MOCKS_PROGRAM_ID},
    events::{AccountEventHeader, MarginfiAccountTransferToNewAccount},
    prelude::*,
    state::marginfi_account::{
        LendingAccount, MarginfiAccount, ACCOUNT_DISABLED, ACCOUNT_IN_FLASHLOAN,
    },
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::{instructions as ix_sysvar, Sysvar};
use bytemuck::Zeroable;

pub fn transfer_to_new_account(ctx: Context<TransferToNewAccount>) -> MarginfiResult {
    // Validate the global fee wallet and claim a nominal fee
    let group = ctx.accounts.group.load()?;
    check_eq!(
        ctx.accounts.global_fee_wallet.key(),
        group.fee_state_cache.global_fee_wallet,
        MarginfiError::InvalidFeeAta
    );
    anchor_lang::system_program::transfer(ctx.accounts.transfer_fee(), ACCOUNT_TRANSFER_FEE)?;

    let mut old_account = ctx.accounts.old_marginfi_account.load_mut()?;

    check!(
        !old_account.get_flag(ACCOUNT_IN_FLASHLOAN),
        MarginfiError::AccountInFlashloan
    );

    // Prevent multiple migrations from the same account
    check_eq!(
        old_account.migrated_to,
        Pubkey::default(),
        MarginfiError::AccountAlreadyMigrated
    );

    let mut new_account = ctx.accounts.new_marginfi_account.load_init()?;
    new_account.initialize(old_account.group, ctx.accounts.new_authority.key());
    new_account.lending_account = old_account.lending_account;
    new_account.emissions_destination_account = old_account.emissions_destination_account;
    new_account.account_flags = old_account.account_flags;
    new_account.migrated_from = ctx.accounts.old_marginfi_account.key();

    old_account.migrated_to = ctx.accounts.new_marginfi_account.key();

    old_account.lending_account = LendingAccount::zeroed();
    old_account.set_flag(ACCOUNT_DISABLED);

    emit!(MarginfiAccountTransferToNewAccount {
        header: AccountEventHeader {
            signer: Some(ctx.accounts.authority.key()),
            marginfi_account: ctx.accounts.new_marginfi_account.key(),
            marginfi_account_authority: ctx.accounts.new_authority.key(),
            marginfi_group: ctx.accounts.group.key(),
        },
        old_account: ctx.accounts.old_marginfi_account.key(),
        old_account_authority: ctx.accounts.authority.key(),
        new_account_authority: ctx.accounts.new_authority.key(),
    });

    Ok(())
}

#[derive(Accounts)]
pub struct TransferToNewAccount<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group,
        has_one = authority
    )]
    pub old_marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<MarginfiAccount>()
    )]
    pub new_marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: WARN: New authority is completely unchecked
    pub new_authority: UncheckedAccount<'info>,

    /// CHECK: Validated against group fee state cache
    #[account(mut)]
    pub global_fee_wallet: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> TransferToNewAccount<'info> {
    fn transfer_fee(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, anchor_lang::system_program::Transfer<'info>> {
        CpiContext::new(
            self.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: self.authority.to_account_info(),
                to: self.global_fee_wallet.to_account_info(),
            },
        )
    }
}


/// Transfer an existing marginfi account to a new PDA-based account
/// 
/// This function transfers all balances and state from an old account to a new PDA-based account.
/// The old account is marked as disabled and migrated_to is set to the new account.
/// The new account is created as a PDA with the provided parameters.
///
/// PDA seeds for new account: [b"marginfi_account", group, new_authority, account_index.to_le_bytes(), third_party_id.unwrap_or(0).to_le_bytes()]
pub fn transfer_to_new_account_pda(
    ctx: Context<TransferToNewAccountPda>,
    _account_index: u32,
    third_party_id: Option<u32>,
) -> MarginfiResult {
    // Validate the global fee wallet and claim a nominal fee
    let group = ctx.accounts.group.load()?;
    check_eq!(
        ctx.accounts.global_fee_wallet.key(),
        group.fee_state_cache.global_fee_wallet,
        MarginfiError::InvalidFeeAta
    );
    anchor_lang::system_program::transfer(ctx.accounts.transfer_fee(), ACCOUNT_TRANSFER_FEE)?;

    let mut old_account = ctx.accounts.old_marginfi_account.load_mut()?;

    // Prevent multiple migrations from the same account
    check_eq!(
        old_account.migrated_to,
        Pubkey::default(),
        MarginfiError::AccountAlreadyMigrated
    );

    // Validate third-party id restriction if provided
    if let Some(id) = third_party_id {
        if id == 42 {
            // Restrict id=42 to CPI calls from the mocks program only
            // Use instruction sysvar to get the actual calling program
            let current_ix_index = ix_sysvar::load_current_index_checked(&ctx.accounts.instructions_sysvar)?;
            
            // Look for the previous instruction (caller) if it exists
            if current_ix_index > 0 {
                let caller_ix = ix_sysvar::load_instruction_at_checked(
                    current_ix_index.saturating_sub(1) as usize,
                    &ctx.accounts.instructions_sysvar
                )?;
                
                // Check if the calling program is the mocks program
                if caller_ix.program_id != MOCKS_PROGRAM_ID {
                    return err!(MarginfiError::Unauthorized);
                }
            } else {
                // Direct call (not CPI), which should not be allowed for id=42
                return err!(MarginfiError::Unauthorized);
            }
        }
    }

    let mut new_account = ctx.accounts.new_marginfi_account.load_init()?;
    new_account.initialize(old_account.group, ctx.accounts.new_authority.key());
    new_account.lending_account = old_account.lending_account;
    new_account.emissions_destination_account = old_account.emissions_destination_account;
    new_account.account_flags = old_account.account_flags;
    new_account.migrated_from = ctx.accounts.old_marginfi_account.key();

    old_account.migrated_to = ctx.accounts.new_marginfi_account.key();

    old_account.lending_account = LendingAccount::zeroed();
    old_account.set_flag(ACCOUNT_DISABLED);

    emit!(MarginfiAccountTransferToNewAccount {
        header: AccountEventHeader {
            signer: Some(ctx.accounts.authority.key()),
            marginfi_account: ctx.accounts.new_marginfi_account.key(),
            marginfi_account_authority: ctx.accounts.new_authority.key(),
            marginfi_group: ctx.accounts.group.key(),
        },
        old_account: ctx.accounts.old_marginfi_account.key(),
        old_account_authority: ctx.accounts.authority.key(),
        new_account_authority: ctx.accounts.new_authority.key(),
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(account_index: u32, third_party_id: Option<u32>)]
pub struct TransferToNewAccountPda<'info> {
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group,
        has_one = authority
    )]
    pub old_marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<MarginfiAccount>(),
        seeds = [
            MARGINFI_ACCOUNT_SEED.as_bytes(),
            group.key().as_ref(),
            new_authority.key().as_ref(),
            &account_index.to_le_bytes(),
            &third_party_id.unwrap_or(0).to_le_bytes(),
        ],
        bump
    )]
    pub new_marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: WARN: New authority is completely unchecked
    pub new_authority: UncheckedAccount<'info>,

    /// CHECK: Validated against group fee state cache
    #[account(mut)]
    pub global_fee_wallet: UncheckedAccount<'info>,

    /// Instructions sysvar for CPI validation
    /// CHECK: Standard sysvar account
    #[account(address = anchor_lang::solana_program::sysvar::instructions::id())]
    pub instructions_sysvar: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> TransferToNewAccountPda<'info> {
    fn transfer_fee(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, anchor_lang::system_program::Transfer<'info>> {
        CpiContext::new(
            self.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: self.authority.to_account_info(),
                to: self.global_fee_wallet.to_account_info(),
            },
        )
    }
}
