use anchor_lang::prelude::*;

declare_id!("65e81uBnLPtUNaFbgzeU4gMwmCbMeeh6GCLDhEVaNNon");

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod macros;
pub mod state;
// pub mod utils;

use crate::instructions::*;
// use crate::state::*;
// use errors::*;

#[program]
pub mod staking_collatizer {
    use super::*;

    pub fn init_user(ctx: Context<InitUser>) -> Result<()> {
        instructions::init_user::init_user(ctx)
    }

    pub fn init_stakeholder(ctx: Context<InitStakeHolder>) -> Result<()> {
        instructions::init_stakeholder::init_stakeholder(ctx)
    }
}
