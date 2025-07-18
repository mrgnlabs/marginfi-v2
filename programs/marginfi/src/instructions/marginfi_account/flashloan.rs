use crate::{
    check,
    prelude::*,
    state::marginfi_account::{MarginfiAccountImpl, RiskEngine},
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{get_stack_height, TRANSACTION_LEVEL_STACK_HEIGHT},
    sysvar::{self, instructions},
};
use marginfi_type_crate::types::{MarginfiAccount, ACCOUNT_DISABLED, ACCOUNT_IN_FLASHLOAN};

pub fn lending_account_start_flashloan(
    ctx: Context<LendingAccountStartFlashloan>,
    end_index: u64,
) -> MarginfiResult<()> {
    check_flashloan_can_start(
        &ctx.accounts.marginfi_account,
        &ctx.accounts.ixs_sysvar,
        end_index as usize,
    )?;

    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
    marginfi_account.set_flag(ACCOUNT_IN_FLASHLOAN);

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountStartFlashloan<'info> {
    #[account(
        mut,
        has_one = authority
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,

    /// CHECK: Instructions sysvar
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
/// 6. Start flashloan ix is not in CPI
/// 7. End flashloan ix is not in CPI
pub fn check_flashloan_can_start(
    marginfi_account: &AccountLoader<MarginfiAccount>,
    sysvar_ixs: &AccountInfo,
    end_fl_idx: usize,
) -> MarginfiResult<()> {
    // Note: FLASHLOAN_ENABLED_FLAG is now deprecated, any non-disabled account can initiate a flash loan.
    let current_ix_idx: usize = instructions::load_current_index_checked(sysvar_ixs)?.into();

    check!(current_ix_idx < end_fl_idx, MarginfiError::IllegalFlashloan);

    // Check current ix is not a CPI
    let current_ix = instructions::load_instruction_at_checked(current_ix_idx, sysvar_ixs)?;

    check!(
        get_stack_height() == TRANSACTION_LEVEL_STACK_HEIGHT,
        MarginfiError::IllegalFlashloan,
        "Start flashloan ix should not be in CPI"
    );

    check!(
        current_ix.program_id.eq(&crate::ID),
        MarginfiError::IllegalFlashloan,
        "Start flashloan ix should not be in CPI"
    );

    // Will error if ix doesn't exist
    let unchecked_end_fl_ix = instructions::load_instruction_at_checked(end_fl_idx, sysvar_ixs)?;

    let discrim = &unchecked_end_fl_ix.data[..8];
    // TODO figure out anchor's fancy new discrim syntax to avoid hard coding this.
    const FLASHLOAN_DISCRIM: [u8; 8] = [105, 124, 201, 106, 153, 2, 8, 156];
    if discrim != FLASHLOAN_DISCRIM {
        msg!("discrim: {:?}, expected: {:?}", discrim, FLASHLOAN_DISCRIM);
        return err!(MarginfiError::IllegalFlashloan);
    }

    check!(
        unchecked_end_fl_ix.program_id.eq(&crate::ID),
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
        !marginf_account.get_flag(ACCOUNT_DISABLED),
        MarginfiError::AccountDisabled
    );

    check!(
        !marginf_account.get_flag(ACCOUNT_IN_FLASHLOAN),
        MarginfiError::IllegalFlashloan
    );

    Ok(())
}

pub fn lending_account_end_flashloan<'info>(
    ctx: Context<'_, '_, 'info, 'info, LendingAccountEndFlashloan<'info>>,
) -> MarginfiResult<()> {
    check!(
        get_stack_height() == TRANSACTION_LEVEL_STACK_HEIGHT,
        MarginfiError::IllegalFlashloan,
        "End flashloan ix should not be in CPI"
    );

    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;

    marginfi_account.unset_flag(ACCOUNT_IN_FLASHLOAN);

    let (risk_result, _engine) =
        RiskEngine::check_account_init_health(&marginfi_account, ctx.remaining_accounts, &mut None);
    risk_result?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountEndFlashloan<'info> {
    #[account(
        mut,
        has_one = authority
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,
}
