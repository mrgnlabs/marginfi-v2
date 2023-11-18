use anchor_lang::prelude::*;
use marginfi::state::marginfi_account::MarginfiAccount;
use marginfi::state::marginfi_group::WrappedI80F48;
use fixed::types::I80F48;
use std::ops::{Div, Mul, Add};

declare_id!("CSjewsFhiPYdz94HLCmntXPiFXUPbwhxUUFo29dVaYwo");

const POINTS_SEED: &[u8] = b"points";

#[program]
pub mod points_program {
    use super::*;

    pub fn initialize_points_account(ctx: Context<InitializePointsAccount>, initial_points: i128) -> Result<()> {
        let points_account = &mut ctx.accounts.points_account;
        points_account.owner_mfi_account = ctx.accounts.owner_mfi_account.key();
        points_account.points = WrappedI80F48::from(I80F48::from_num(initial_points));
        Ok(())
    }
                                                                // pubkey is the bank_pk
    pub fn accrue_points_naive(ctx: Context<AccruePointsNaive>, price_data: Vec<(Pubkey, i128)>) -> Result<()> {
        let lending_account_balances = &ctx.accounts.marginfi_account.load()?.lending_account.balances;
        let points_account = &mut ctx.accounts.points_account;

        let mut current_asset_balance: i128 = 0;
        let mut current_liab_balance: i128 = 0;

        for balance in lending_account_balances.iter() {
            if balance.active {
                if let Some((_, price)) = price_data.iter().find(|(pk, _)| *pk == balance.bank_pk) {
                    current_asset_balance += I80F48::from(balance.asset_shares).mul(I80F48::from_num(*price)).to_bits();
    
                    current_liab_balance += I80F48::from(balance.liability_shares).mul(I80F48::from_num(*price)).to_bits();
                }
            }
        }

        points_account.update_sma(current_asset_balance, current_liab_balance);

        let clock = Clock::get().unwrap();           
        let current_unix_timestamp: u64 = clock.unix_timestamp.try_into().unwrap();

        points_account.accrue_points(current_unix_timestamp);
    
        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct PointsAccount {
    pub owner_mfi_account: Pubkey,
    pub points: WrappedI80F48,
    pub asset_sma: WrappedI80F48,
    pub liab_sma: WrappedI80F48,
    pub sma_count: u64, 
    pub last_recorded_timestamp: u64,
}

impl PointsAccount {
    pub fn update_sma(&mut self, current_asset_balance: i128, current_liab_balance: i128) {
        let current_asset_sma_value = I80F48::from_num(self.asset_sma.value);
        let total_asset_value = current_asset_sma_value
            .mul(I80F48::from_num(self.sma_count as i128))
            .add(I80F48::from_num(current_asset_balance));
        self.asset_sma = WrappedI80F48::from(total_asset_value
            .div(I80F48::from_num(self.sma_count as i128 + 1)));

        let current_liab_sma_value = I80F48::from_num(self.liab_sma.value);
        let total_liab_value = current_liab_sma_value
            .mul(I80F48::from_num(self.sma_count as i128))
            .add(I80F48::from_num(current_liab_balance));                      
        self.liab_sma = WrappedI80F48::from(total_liab_value
            .div(I80F48::from_num(self.sma_count as i128 + 1)));

        self.sma_count += 1;
    }

    pub fn accrue_points(&mut self, current_timestamp: u64) {
        // 1 point per $1 lent per 24h
        let lending_points = I80F48::from(self.asset_sma).div(I80F48::from_num(24 * 60 * 60 / (current_timestamp - self.last_recorded_timestamp)));

        // 4 points per $1 borrowed per 24h
        let borrowing_points = I80F48::from(self.liab_sma).mul(I80F48::from_num(4)).div(I80F48::from_num(24 * 60 * 60 / (current_timestamp - self.last_recorded_timestamp)));

        self.points = WrappedI80F48::from(I80F48::from(self.points) + lending_points + borrowing_points);
    }
}

#[derive(Accounts)]
pub struct AccruePointsNaive<'info> {
    #[account(mut, constraint = points_account.owner_mfi_account == marginfi_account.key())]
    pub points_account: Account<'info, PointsAccount>,

    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,
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

    pub owner_mfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
