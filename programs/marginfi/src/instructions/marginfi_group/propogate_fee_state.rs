use anchor_lang::prelude::*;

use crate::{constants::FEE_STATE_SEED, state::fee_state::FeeState, MarginfiGroup};

#[derive(Accounts)]
pub struct PropogateFee<'info> {
    // Note: there is just one FeeState per program, so no further check is required.
    #[account(
        seeds = [FEE_STATE_SEED.as_bytes()],
        bump,
    )]
    pub fee_state: AccountLoader<'info, FeeState>,

    /// Any group, this ix is permisionless and can propogate the fee to any group
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,
}

pub fn propogate_fee(ctx: Context<PropogateFee>) -> Result<()> {
    let mut group = ctx.accounts.marginfi_group.load_mut()?;
    let fee_state = ctx.accounts.fee_state.load()?;

    group.fee_state_cache.global_fee_wallet = fee_state.global_fee_wallet;
    group.fee_state_cache.program_fee_fixed = fee_state.program_fee_fixed;
    group.fee_state_cache.program_fee_rate = fee_state.program_fee_rate;

    Ok(())
}
