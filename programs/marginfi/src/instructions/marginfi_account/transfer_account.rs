use crate::{
    check, check_eq,
    constants::ACCOUNT_TRANSFER_FEE,
    events::{AccountEventHeader, MarginfiAccountTransferToNewAccount},
    prelude::*,
    state::marginfi_account::MarginfiAccountImpl,
};
use anchor_lang::prelude::*;
use bytemuck::Zeroable;
use marginfi_type_crate::types::{
    LendingAccount, MarginfiAccount, MarginfiGroup, ACCOUNT_DISABLED, ACCOUNT_IN_FLASHLOAN,
};

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
