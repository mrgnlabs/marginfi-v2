// Global fee admin calls this to edit the fee rate or the fee wallet.

use crate::state::fee_state;
use crate::constants::FEE_STATE_SEED;
use anchor_lang::prelude::*;
use fee_state::FeeState;

pub fn edit_fee_state(
    ctx: Context<EditFeeState>,
    fee_wallet: Pubkey,
    bank_init_flat_sol_fee: u32,
) -> Result<()> {
    let mut fee_state = ctx.accounts.fee_state.load_mut()?;
    fee_state.global_fee_wallet = fee_wallet;
    fee_state.bank_init_flat_sol_fee = bank_init_flat_sol_fee;

    Ok(())
}

#[derive(Accounts)]
pub struct EditFeeState<'info> {
    /// Admin of the global FeeState
    #[account(mut)]
    pub global_fee_admin: Signer<'info>,

    // Note: there is just one FeeState per program, so no further check is required.
    #[account(
        mut,
        seeds = [FEE_STATE_SEED.as_bytes()], 
        bump,
        has_one = global_fee_admin
    )]
    pub fee_state: AccountLoader<'info, FeeState>,
}
