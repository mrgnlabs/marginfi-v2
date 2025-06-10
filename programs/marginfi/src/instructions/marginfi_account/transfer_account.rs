use crate::{
    constants::ACCOUNT_TRANSFER_FEE,
    events::{AccountEventHeader, MarginfiAccountTransferAccountAuthorityEvent},
    prelude::*,
    state::marginfi_account::{LendingAccount, MarginfiAccount, ACCOUNT_DISABLED},
};
use anchor_lang::prelude::*;
use bytemuck::Zeroable;

pub fn transfer_account_authority(ctx: Context<TransferAccountAuthority>) -> MarginfiResult {
    // The global fee wallet claims a (nominal) fee
    anchor_lang::system_program::transfer(ctx.accounts.transfer_fee(), ACCOUNT_TRANSFER_FEE)?;

    let mut old_account = ctx.accounts.old_marginfi_account.load_mut()?;

    let mut new_account = ctx.accounts.new_marginfi_account.load_init()?;
    new_account.initialize(old_account.group, ctx.accounts.new_authority.key());
    new_account.lending_account = old_account.lending_account;
    new_account.emissions_destination_account = old_account.emissions_destination_account;
    new_account.account_flags = old_account.account_flags;
    new_account.migrated_from = ctx.accounts.old_marginfi_account.key();

    old_account.lending_account = LendingAccount::zeroed();
    old_account.set_flag(ACCOUNT_DISABLED);

    emit!(MarginfiAccountTransferAccountAuthorityEvent {
        header: AccountEventHeader {
            signer: Some(ctx.accounts.authority.key()),
            marginfi_account: ctx.accounts.new_marginfi_account.key(),
            marginfi_account_authority: ctx.accounts.new_authority.key(),
            marginfi_group: ctx.accounts.group.key(),
        },
        old_account_authority: ctx.accounts.authority.key(),
        new_account_authority: ctx.accounts.new_authority.key(),
    });

    Ok(())
}

#[derive(Accounts)]
pub struct TransferAccountAuthority<'info> {
    #[account(
        mut, 
        has_one = authority, 
        has_one = group
    )]
    pub old_marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<MarginfiAccount>()
    )]
    pub new_marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: WARN: New authority is completely unchecked
    pub new_authority: AccountInfo<'info>,

    /// CHECK: Validated against group fee state cache
    #[account(mut)]
    pub global_fee_wallet: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> TransferAccountAuthority<'info> {
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
