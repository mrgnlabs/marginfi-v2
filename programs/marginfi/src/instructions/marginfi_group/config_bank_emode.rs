use crate::state::emode::{EmodeEntry, MAX_EMODE_ENTRIES};
use crate::{
    state::marginfi_group::{Bank, MarginfiGroup},
    MarginfiResult,
};
use anchor_lang::prelude::*;

pub fn lending_pool_configure_bank_emode(
    ctx: Context<LendingPoolConfigureBankEmode>,
    emode_tag: u16,
    flags: u64,
    entries: [EmodeEntry; MAX_EMODE_ENTRIES],
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    bank.emode.emode_tag = emode_tag;
    bank.emode.flags = flags;
    bank.emode.entries = entries;
    bank.emode.timestamp = Clock::get()?.unix_timestamp;
    bank.emode.validate_entries()?;

    msg!(
        "emode tag set to {:?} status set to: {:?}",
        emode_tag,
        entries
    );

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolConfigureBankEmode<'info> {
    #[account(
        has_one = emode_admin
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    pub emode_admin: Signer<'info>,

    #[account(
        mut,
        has_one = group,
    )]
    pub bank: AccountLoader<'info, Bank>,
}
