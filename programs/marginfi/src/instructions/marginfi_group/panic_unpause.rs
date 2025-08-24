use crate::constants::FEE_STATE_SEED;
use crate::state::fee_state::FeeState;
use anchor_lang::prelude::*;

pub fn panic_unpause(ctx: Context<PanicUnpause>) -> Result<()> {
    
    let mut fee_state = ctx.accounts.fee_state.load_mut()?;
    let current_timestamp = Clock::get()?.unix_timestamp;

    require!(fee_state.panic_state.is_paused(), crate::errors::MarginfiError::ProtocolNotPaused);

    fee_state.panic_state.update_if_expired(current_timestamp);

    if fee_state.panic_state.is_paused() {
        fee_state.panic_state.unpause();
        msg!("Protocol manually unpaused by admin at timestamp: {}", current_timestamp);
    } else {
        msg!("Protocol was already auto-unpaused due to expiration at timestamp: {}", current_timestamp);
    }

    msg!("Consecutive pause count reset to: {}", fee_state.panic_state.consecutive_pause_count);

    Ok(())
}

#[derive(Accounts)]
pub struct PanicUnpause<'info> {
    /// Admin of the global FeeState (can manually unpause)
    #[account(mut)]
    pub global_fee_admin: Signer<'info>,

    #[account(
        mut,
        seeds = [FEE_STATE_SEED.as_bytes()],
        bump,
        has_one = global_fee_admin
    )]
    pub fee_state: AccountLoader<'info, FeeState>,
}