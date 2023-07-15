use anchor_lang::{prelude::*, Discriminator};
use solana_program::sysvar::{self, instructions};

use crate::{
    check,
    prelude::*,
    state::marginfi_account::{MarginfiAccount, RiskEngine, DISABLED_FLAG, IN_FLASHLOAN_FLAG},
};

pub fn lending_account_start_flashloan(
    ctx: Context<LendingAccountStartFlashloan>,
    end_index: u64,
) -> MarginfiResult<()> {
    check_flashloan_can_start(
        &ctx.accounts.marginfi_account,
        &ctx.accounts.ixs_sysvar,
        end_index,
    )?;

    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
    marginfi_account.set_flag(IN_FLASHLOAN_FLAG);

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountStartFlashloan<'info> {
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
    #[account(address = marginfi_account.load()?.authority)]
    pub signer: Signer<'info>,
    #[account(address = sysvar::instructions::ID)]
    pub ixs_sysvar: AccountInfo<'info>,
}

const END_FL_IX_MARGINFI_ACCOUNT_AI_IDX: usize = 0;

/// Checklist
/// 1. `end_flashloan` ix index is after `start_flashloan` ix index
/// 2. Ixs has an `end_flashloan` ix present
/// 3. `end_flashloan` ix is for the marginfi program
/// 3. `end_flashloan` ix is for the same marginfi account
/// 4. Account is not disabled
/// 5. Account is not already in a flashloan
pub fn check_flashloan_can_start(
    marginfi_account: &AccountLoader<MarginfiAccount>,
    sysvar_ixs: &AccountInfo,
    end_fl_idx: u64,
) -> MarginfiResult<()> {
    let current_ix_idx: u64 = instructions::load_current_index_checked(sysvar_ixs)?.into();

    check!(current_ix_idx < end_fl_idx, MarginfiError::IllegalFlashloan);

    // Will error if ix doesn't exist
    let unchecked_end_fl_ix =
        instructions::load_instruction_at_checked(end_fl_idx as usize, sysvar_ixs)?;

    check!(
        unchecked_end_fl_ix.data[..8]
            .eq(&crate::instruction::LendingAccountEndFlashloan::DISCRIMINATOR),
        MarginfiError::IllegalFlashloan
    );

    check!(
        unchecked_end_fl_ix.program_id.eq(&crate::id()),
        MarginfiError::IllegalFlashloan
    );

    let end_fl_ix = unchecked_end_fl_ix;
    let end_fl_marginfi_account = end_fl_ix
        .accounts
        .get(END_FL_IX_MARGINFI_ACCOUNT_AI_IDX)
        .ok_or(MarginfiError::IllegalFlashloan)?;

    check!(
        end_fl_marginfi_account.pubkey.eq(&marginfi_account.key()),
        MarginfiError::IllegalFlashloan
    );

    let marginf_account = marginfi_account.load()?;

    check!(
        !marginf_account.get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

    check!(
        !marginf_account.get_flag(IN_FLASHLOAN_FLAG),
        MarginfiError::IllegalFlashloan
    );

    Ok(())
}

pub fn lending_account_end_flashloan(
    ctx: Context<LendingAccountEndFlashloan>,
) -> MarginfiResult<()> {
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;

    marginfi_account.unset_flag(IN_FLASHLOAN_FLAG);

    RiskEngine::check_account_init_health(&marginfi_account, ctx.remaining_accounts)?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountEndFlashloan<'info> {
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
    #[account(address = marginfi_account.load()?.authority)]
    pub authority: Signer<'info>,
}
