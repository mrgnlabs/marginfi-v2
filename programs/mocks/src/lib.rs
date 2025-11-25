use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::*;

pub mod errors;
pub mod instructions;
pub mod macros;
pub mod number;
pub mod state;
// pub mod utils;

use crate::instructions::*;
use number::Number;
use state::*;
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

    /// Create a marginfi account PDA via CPI
    pub fn create_marginfi_account_pda_via_cpi(
        ctx: Context<CreateMarginfiAccountPdaViaCpi>,
        account_index: u16,
        third_party_id: Option<u16>,
    ) -> Result<()> {
        instructions::pda_account_creation::CreateMarginfiAccountPdaViaCpi::create_marginfi_account_pda_via_cpi(
            ctx,
            account_index,
            third_party_id,
        )
    }

    /// Start a liquidation via CPI
    pub fn start_liquidation_via_cpi<'info>(
        ctx: Context<'_, '_, 'info, 'info, StartLiquidationViaCpi<'info>>,
    ) -> Result<()> {
        instructions::start_liquidate::StartLiquidationViaCpi::start_liquidation_via_cpi(ctx)
    }

    /// Handle bankruptcy via CPI
    pub fn handle_bankruptcy<'info>(
        ctx: Context<'_, '_, 'info, 'info, HandleBankruptcyViaCpi<'info>>,
    ) -> Result<()> {
        instructions::handle_bankruptcy::HandleBankruptcyViaCpi::handle_bankruptcy_via_cpi(ctx)
    }

    // Exponent mocks
    pub fn init_state(ctx: Context<InitState>, exchange_rate: Number) -> Result<()> {
        let st = &mut ctx.accounts.state;
        st.admin = ctx.accounts.admin.key();
        st.bump = ctx.bumps.state;
        st.mint_bump = ctx.bumps.mint_sy;
        st.pool_bump = ctx.bumps.pool_sy;
        st.exchange_rate = exchange_rate;
        st.mint_sy = ctx.accounts.mint_sy.key();
        st.pool_sy = ctx.accounts.pool_sy.key();
        Ok(())
    }

    /// set the exchange rate to any ratio without any limits.
    pub fn set_exchange_rate(ctx: Context<SetRate>, exchange_rate: Number) -> Result<()> {
        ctx.accounts.state.exchange_rate = exchange_rate;
        Ok(())
    }

    // [3] init with 0 balance
    #[instruction(discriminator = [3])]
    pub fn init_personal(ctx: Context<InitPersonal>) -> Result<()> {
        let p = &mut ctx.accounts.personal;
        p.owner = ctx.accounts.owner.key();
        p.sy_balance = 0;
        Ok(())
    }

    // [5] deposit arbitrary sy tokens without any limits, updates the "Personal" balance
    #[instruction(discriminator = [5])]
    pub fn deposit_sy(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.from_sy.to_account_info(),
                    to: ctx.accounts.pool_sy.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount,
        )?;
        ctx.accounts.personal.sy_balance = ctx
            .accounts
            .personal
            .sy_balance
            .checked_add(amount)
            .unwrap();
        let ret = SyState {
            exchange_rate: ctx.accounts.state.exchange_rate,
            emission_indexes: vec![],
        };
        anchor_lang::solana_program::program::set_return_data(&ret.try_to_vec()?);
        Ok(())
    }

    // [6] withdraw arbitrary sy tokens without any limits, updates the "Personal" balance, fails if
    // not enough in the pool.
    #[instruction(discriminator = [6])]
    pub fn withdraw_sy(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_sy.to_account_info(),
                    to: ctx.accounts.to_sy.to_account_info(),
                    authority: ctx.accounts.state.to_account_info(),
                },
                &[state_signer_seeds!(ctx.accounts.state)],
            ),
            amount,
        )?;
        ctx.accounts.personal.sy_balance = ctx
            .accounts
            .personal
            .sy_balance
            .checked_sub(amount)
            .unwrap();
        let ret = SyState {
            exchange_rate: ctx.accounts.state.exchange_rate,
            emission_indexes: vec![],
        };
        anchor_lang::solana_program::program::set_return_data(&ret.try_to_vec()?);
        Ok(())
    }

    // [1] mint arbitrary sy tokens without any limits
    #[instruction(discriminator = [1])]
    pub fn mint_sy(ctx: Context<MintSy>, amount_base: u64) -> Result<()> {
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint_sy.to_account_info(),
                    to: ctx.accounts.dst_sy.to_account_info(),
                    authority: ctx.accounts.state.to_account_info(),
                },
                &[state_signer_seeds!(ctx.accounts.state)],
            ),
            amount_base,
        )?;
        let ret = MintSyReturnData {
            sy_out_amount: amount_base,
            exchange_rate: ctx.accounts.state.exchange_rate,
        };
        anchor_lang::solana_program::program::set_return_data(&ret.try_to_vec()?);
        Ok(())
    }

    // [2] burn arbitrary sy tokens without any limits
    #[instruction(discriminator = [2])]
    pub fn redeem_sy(ctx: Context<RedeemSy>, amount_sy: u64) -> Result<()> {
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.mint_sy.to_account_info(),
                    from: ctx.accounts.src_sy.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            amount_sy,
        )?;
        let ret = RedeemSyReturnData {
            base_out_amount: amount_sy,
            exchange_rate: ctx.accounts.state.exchange_rate,
        };
        anchor_lang::solana_program::program::set_return_data(&ret.try_to_vec()?);
        Ok(())
    }

    // [8] TBD
    #[instruction(discriminator = [8])]
    pub fn claim_emission(_ctx: Context<ClaimEmission>, _amount: u64) -> Result<()> {
        Ok(())
    }
}
