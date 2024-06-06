use crate::{
    constants::LIQUID_INSURANCE_USER_SEED,
    events::MarginfiCreateNewLiquidInsuranceFundAccountEvent, prelude::*,
    state::liquid_insurance_fund::LiquidInsuranceFundAccount,
};
use anchor_lang::prelude::*;
use solana_program::sysvar::Sysvar;

pub fn initialize_account(ctx: Context<LiquidInsuranceFundAccountInitialize>) -> MarginfiResult {
    let LiquidInsuranceFundAccountInitialize {
        user_insurance_fund_account,
        signer,
        ..
    } = ctx.accounts;

    let mut user_insurance_fund_account = user_insurance_fund_account.load_init()?;

    user_insurance_fund_account.initialize(signer.key());

    emit!(MarginfiCreateNewLiquidInsuranceFundAccountEvent { user: signer.key() });

    Ok(())
}

#[derive(Accounts)]
pub struct LiquidInsuranceFundAccountInitialize<'info> {
    #[account(
        init,
        payer = signer,
        space = 8 + std::mem::size_of::<LiquidInsuranceFundAccount>(),
        seeds = [
            LIQUID_INSURANCE_USER_SEED.as_bytes(),
            signer.key().as_ref(),
        ],
        bump
    )]
    pub user_insurance_fund_account: AccountLoader<'info, LiquidInsuranceFundAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
