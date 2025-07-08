// Global fee admin calls this to edit the fee rate or the fee wallet.
use crate::constants::FEE_STATE_SEED;
use crate::utils::wrapped_i80f48_to_f64;
use anchor_lang::prelude::*;
use marginfi_type_crate::types::{FeeState, WrappedI80F48};

pub fn edit_fee_state(
    ctx: Context<EditFeeState>,
    admin: Pubkey,
    fee_wallet: Pubkey,
    bank_init_flat_sol_fee: u32,
    program_fee_fixed: WrappedI80F48,
    program_fee_rate: WrappedI80F48,
) -> Result<()> {
    let mut fee_state = ctx.accounts.fee_state.load_mut()?;
    fee_state.global_fee_admin = admin;
    fee_state.global_fee_wallet = fee_wallet;
    fee_state.bank_init_flat_sol_fee = bank_init_flat_sol_fee;
    fee_state.program_fee_fixed = program_fee_fixed;
    fee_state.program_fee_rate = program_fee_rate;

    let fixed_f64: f64 = wrapped_i80f48_to_f64(fee_state.program_fee_fixed);
    let rate_f64: f64 = wrapped_i80f48_to_f64(fee_state.program_fee_rate);
    msg!("admin set to: {:?} fee wallet: {:?}", admin, fee_wallet);
    msg!(
        "flat sol: {:?} fixed: {:?} rate: {:?}",
        fee_state.bank_init_flat_sol_fee,
        fixed_f64,
        rate_f64
    );

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
