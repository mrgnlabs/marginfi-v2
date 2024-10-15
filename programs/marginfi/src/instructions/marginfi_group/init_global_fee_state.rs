// Runs once per program to init the global fee state.
use crate::constants::FEE_STATE_SEED;
use crate::state::fee_state;
use crate::state::marginfi_group::WrappedI80F48;
use anchor_lang::prelude::*;
use fee_state::FeeState;

#[allow(unused_variables)]
pub fn initialize_fee_state(
    ctx: Context<InitFeeState>,
    admin_key: Pubkey,
    fee_wallet: Pubkey,
    bank_init_flat_sol_fee: u32,
    program_fee_fixed: WrappedI80F48,
    program_fee_rate: WrappedI80F48,
) -> Result<()> {
    let mut fee_state = ctx.accounts.fee_state.load_init()?;
    // On mainnet we always use the mrgn program multisig. On other networks, configurable.
    cfg_if::cfg_if! {
        if #[cfg(feature = "mainnet-beta")] {
            fee_state.global_fee_admin = pubkey!("3HGdGLrnK9DsnHi1mCrUMLGfQHcu6xUrXhMY14GYjqvM");
        } else {
            fee_state.global_fee_admin = admin_key;
        }
    }
    fee_state.global_fee_wallet = fee_wallet;
    fee_state.key = ctx.accounts.fee_state.key();
    fee_state.bank_init_flat_sol_fee = bank_init_flat_sol_fee;
    fee_state.bump_seed = ctx.bumps.fee_state;
    fee_state.program_fee_fixed = program_fee_fixed;
    fee_state.program_fee_rate = program_fee_rate;

    Ok(())
}

#[derive(Accounts)]
pub struct InitFeeState<'info> {
    /// Pays the init fee
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        seeds = [
            FEE_STATE_SEED.as_bytes()
        ],
        bump,
        payer = payer,
        space = 8 + FeeState::LEN,
    )]
    pub fee_state: AccountLoader<'info, FeeState>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}
