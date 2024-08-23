use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod macros;
pub mod state;
// pub mod utils;

use crate::instructions::*;
// use crate::state::*;
// use errors::*;

declare_id!("5XaaR94jBubdbrRrNW7DtRvZeWvLhSHkEGU3jHTEXV3C");

#[program]
pub mod mocks {
    use super::*;
    use std::io::Write as IoWrite;

    /// Do nothing
    pub fn do_nothing(ctx: Context<DoNothing>) -> Result<()> {
        instructions::do_nothing::do_nothing(ctx)
    }

    /// Init authority for fake jupiter-like swap pools
    pub fn init_pool_auth(ctx: Context<InitPoolAuth>, nonce: u16) -> Result<()> {
        instructions::init_pool_auth::init_pool_auth(ctx, nonce)
    }

    /// Execute an exchange of a:b like-jupiter. You set the amount a sent and b received.
    pub fn swap_like_jupiter<'info>(
        ctx: Context<'_, '_, '_, 'info, SwapLikeJupiter<'info>>,
        amt_a: u64,
        amt_b: u64,
    ) -> Result<()> {
        instructions::swap_like_jupiter::SwapLikeJupiter::swap_like_jup(ctx, amt_a, amt_b)
    }

    #[derive(Accounts)]
    pub struct Write<'info> {
        #[account(mut)]
        target: Signer<'info>,
    }

    /// Write arbitrary bytes to an arbitrary account. YOLO.
    pub fn write(ctx: Context<Write>, offset: u64, data: Vec<u8>) -> Result<()> {
        let account_data = ctx.accounts.target.to_account_info().data;
        let borrow_data = &mut *account_data.borrow_mut();
        let offset = offset as usize;

        Ok((&mut borrow_data[offset..]).write_all(&data[..])?)
    }
}
