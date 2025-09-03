use anchor_lang::prelude::*;
use marginfi_type_crate::{constants::FEE_STATE_SEED, types::FeeState};

use crate::state::panic_state::PanicStateImpl;

pub fn panic_unpause_permissionless(ctx: Context<PanicUnpausePermissionless>) -> Result<()> {
    let mut fee_state = ctx.accounts.fee_state.load_mut()?;
    let current_timestamp = Clock::get()?.unix_timestamp;

    require!(
        fee_state.panic_state.is_paused_flag(),
        crate::errors::MarginfiError::ProtocolNotPaused
    );

    require!(
        fee_state.panic_state.is_expired(current_timestamp),
        crate::errors::MarginfiError::PauseLimitExceeded
    );

    fee_state.panic_state.unpause();

    msg!(
        "Protocol auto-unpaused at timestamp: {} (expired after {} seconds)",
        current_timestamp,
        current_timestamp - fee_state.panic_state.pause_start_timestamp
    );
    msg!(
        "Consecutive pause count reset to: {}",
        fee_state.panic_state.consecutive_pause_count
    );

    Ok(())
}

#[derive(Accounts)]
pub struct PanicUnpausePermissionless<'info> {
    /// Anyone can call this to unpause expired pauses
    pub signer: Signer<'info>,

    #[account(
        mut,
        seeds = [FEE_STATE_SEED.as_bytes()],
        bump
    )]
    pub fee_state: AccountLoader<'info, FeeState>,
}
