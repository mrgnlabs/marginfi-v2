use anchor_lang::prelude::*;

use crate::{constants::FEE_STATE_SEED, state::fee_state::FeeState, MarginfiGroup, MarginfiResult};

#[derive(Accounts)]
pub struct ConfigGroupFee<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    /// `global_fee_admin` of the FeeState
    pub global_fee_admin: Signer<'info>,

    // Note: there is just one FeeState per program, so no further check is required.
    #[account(
        seeds = [FEE_STATE_SEED.as_bytes()],
        bump,
        has_one = global_fee_admin
    )]
    pub fee_state: AccountLoader<'info, FeeState>,
}

pub fn config_group_fee(ctx: Context<ConfigGroupFee>, flag: u64) -> MarginfiResult {
    let mut marginfi_group = ctx.accounts.marginfi_group.load_mut()?;

    marginfi_group.set_flags(flag)?;

    msg!("flags set to: {:?}", flag);

    Ok(())
}
