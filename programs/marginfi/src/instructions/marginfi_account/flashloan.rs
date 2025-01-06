use anchor_lang::{prelude::*, Discriminator};
use solana_program::{
    instruction::{get_stack_height, TRANSACTION_LEVEL_STACK_HEIGHT},
    sysvar::{self, instructions},
};

use crate::{
    check,
    constants::FEE_STATE_SEED,
    prelude::*,
    state::fee_state::FeeState,
    state::marginfi_account::{MarginfiAccount, RiskEngine, DISABLED_FLAG, IN_FLASHLOAN_FLAG},
};

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
    marginfi_account.set_flag(IN_FLASHLOAN_FLAG);

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountStartFlashloan<'info> {
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
    #[account(address = marginfi_account.load()?.authority)]
    pub signer: Signer<'info>,
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
    // Note: FLASHLOAN_ENABLED_FLAG is now deprecated.
    // Any non-disabled account can initiate a flash loan.
    check!(
        !marginfi_account.load()?.get_flag(DISABLED_FLAG),
        MarginfiError::AccountDisabled
    );

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
        current_ix.program_id.eq(&crate::id()),
        MarginfiError::IllegalFlashloan,
        "Start flashloan ix should not be in CPI"
    );

    // Will error if ix doesn't exist
    let unchecked_end_fl_ix = instructions::load_instruction_at_checked(end_fl_idx, sysvar_ixs)?;

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

pub fn lending_account_end_flashloan<'info>(
    ctx: Context<'_, '_, 'info, 'info, LendingAccountEndFlashloan<'info>>,
) -> MarginfiResult<()> {
    check!(
        get_stack_height() == TRANSACTION_LEVEL_STACK_HEIGHT,
        MarginfiError::IllegalFlashloan,
        "End flashloan ix should not be in CPI"
    );

    // Transfer the flat sol flashloan fee to the global fee wallet
    let fee_state = ctx.accounts.fee_state.load()?;
    let flashloan_flat_sol_fee = fee_state.flashloan_flat_sol_fee;
    if flashloan_flat_sol_fee > 0 {
        anchor_lang::system_program::transfer(
            ctx.accounts.transfer_flat_fee(),
            flashloan_flat_sol_fee as u64,
        )?;
    }

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
    pub signer: Signer<'info>,
    // Note: there is just one FeeState per program, so no further check is required.
    #[account(
        seeds = [FEE_STATE_SEED.as_bytes()],
        bump,
        has_one = global_fee_wallet
    )]
    pub fee_state: AccountLoader<'info, FeeState>,
    /// CHECK: The fee admin's native SOL wallet, validated against fee state
    #[account(mut)]
    pub global_fee_wallet: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> LendingAccountEndFlashloan<'info> {
    fn transfer_flat_fee(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, anchor_lang::system_program::Transfer<'info>> {
        CpiContext::new(
            self.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: self.signer.to_account_info(),
                to: self.global_fee_wallet.to_account_info(),
            },
        )
    }
}
