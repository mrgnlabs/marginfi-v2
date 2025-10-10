use crate::state::panic_state::PanicStateImpl;
use anchor_lang::prelude::*;
use marginfi_type_crate::constants::FEE_STATE_SEED;
use marginfi_type_crate::types::{FeeState, PanicState};

pub fn panic_pause(ctx: Context<PanicPause>) -> Result<()> {
    let mut fee_state = ctx.accounts.fee_state.load_mut()?;
    let current_timestamp = Clock::get()?.unix_timestamp;

    // Update panic state if the current pause has expired
    fee_state.panic_state.unpause_if_expired(current_timestamp);

    fee_state.panic_state.pause(current_timestamp)?;

    msg!("Protocol paused at timestamp: {}", current_timestamp);
    msg!(
        "Pause will auto-expire at: {}",
        current_timestamp + PanicState::PAUSE_DURATION_SECONDS
    );
    msg!(
        "Daily pause count: {}",
        fee_state.panic_state.daily_pause_count
    );
    msg!(
        "Consecutive pause count: {}",
        fee_state.panic_state.consecutive_pause_count
    );

    Ok(())
}

#[derive(Accounts)]
pub struct PanicPause<'info> {
    /// Admin of the global FeeState (can trigger panic pause)
    pub global_fee_admin: Signer<'info>,

    /// Global fee state account containing the panic state
    #[account(
        mut,
        seeds = [FEE_STATE_SEED.as_bytes()],
        bump,
        has_one = global_fee_admin
    )]
    pub fee_state: AccountLoader<'info, FeeState>,
}
