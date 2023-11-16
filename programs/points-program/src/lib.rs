use anchor_lang::prelude::*;
use switchboard_v2::{AggregatorAccountData, SWITCHBOARD_PROGRAM_ID};

declare_id!("CSjewsFhiPYdz94HLCmntXPiFXUPbwhxUUFo29dVaYwo");

const POINTS_SEED: &[u8] = b"points";

#[error_code]
pub enum OracleError {
    #[msg("Invalid oracle account")] 
    InvalidAccount,
}

#[program]
pub mod points_program {
    use super::*;

    pub fn initialize_points_account(ctx: Context<InitializePointsAccount>, initial_points: u64) -> Result<()> {
        let points_account = &mut ctx.accounts.points_account;
        points_account.owner_mfi_account = ctx.accounts.owner_mfi_account.key();
        points_account.points = initial_points;
        Ok(())
    }
}

#[account]
pub struct PointsAccount {
    pub owner_mfi_account: Pubkey,
    pub points: u64,
}

#[derive(Accounts)]
pub struct InitializePointsAccount<'info> {
    #[account(
        init,
        space = 8 + std::mem::size_of::<PointsAccount>(),
        payer = payer,
        seeds = [
            POINTS_SEED,
            owner_mfi_account.key().as_ref(),
        ],
        bump
    )]
    pub points_account: Account<'info, PointsAccount>,

    /// CHECK:
    pub owner_mfi_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
