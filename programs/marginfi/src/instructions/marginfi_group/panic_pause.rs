use crate::constants::FEE_STATE_SEED;
use crate::state::fee_state::FeeState;
use anchor_lang::prelude::*;

pub fn panic_pause(ctx: Context<PanicPause>) -> Result<()> {
    let mut fee_state = ctx.accounts.fee_state.load_mut()?;
    let current_timestamp = Clock::get()?.unix_timestamp;

    // Update panic state if the current pause has expired
    fee_state.panic_state.update_if_expired(current_timestamp);

    fee_state.panic_state.pause(current_timestamp)?;

    msg!("Protocol paused at timestamp: {}", current_timestamp);
    msg!(
        "Pause will auto-expire at: {}",
        current_timestamp + crate::state::fee_state::PanicState::PAUSE_DURATION_SECONDS
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
    #[account(mut)]
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
