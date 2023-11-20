use anchor_lang::prelude::*;
use marginfi::state::{marginfi_group::WrappedI80F48, marginfi_account::MarginfiAccount};
use fixed::types::I80F48;
use std::ops::{Div, Mul, Add};

declare_id!("CSjewsFhiPYdz94HLCmntXPiFXUPbwhxUUFo29dVaYwo");

const MAX_POINTS_ACCOUNTS: usize = 25_000;

#[program]
pub mod points_program {
    use super::*;

    pub fn initialize_global_points(ctx: Context<InitializeGlobalPoints>) -> Result<()> {
        
        Ok(())
    }

    pub fn intialize_points_account(ctx: Context<InitializePointsAccount>, initial_points: i128) -> Result<()> {

        Ok(())
    }

    pub fn accrue_points(
        ctx: Context<AccruePoints>, 
        account_balance_datas: Vec<(Pubkey, AccountBalances)>, 
        price_data: Vec<(Pubkey, i128)>, 
        starting_index: usize,
        slice_length: Option<usize>,
        ) -> Result<()> {
            
        Ok(())
    }
}

#[account(zero_copy)]
pub struct PointsMapping {
    pub points_accounts: [Option<PointsAccount>; MAX_POINTS_ACCOUNTS],
}

#[derive(Default, Copy, Clone)]
pub struct PointsAccount {
    pub owner_mfi_account: Pubkey,
    pub points: WrappedI80F48,
    pub asset_sma: WrappedI80F48,
    pub liab_sma: WrappedI80F48,
    pub sma_count: u64, 
    pub last_recorded_timestamp: u64,
}

// This is so we can pass in [Balance; 16] as an endpoint argument
#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct AccountBalances {
    pub balances: [Balance; 16]
}

// This is a re-implementation of marginfi::state::::marginfi_account::Balance
// Only here because I don't want to change main marginfi code when it's not _really_ necessary
#[derive(AnchorSerialize, AnchorDeserialize, Default, Copy, Clone)]
pub struct Balance {
    pub active: bool,
    pub bank_pk: Pubkey,
    pub asset_shares: WrappedI80F48,
    pub liability_shares: WrappedI80F48,
    pub emissions_outstanding: WrappedI80F48,
    pub last_update: u64,
    pub _padding: [u64; 1],
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
pub struct InitializeGlobalPoints<'info> {
    pub points_mapping: AccountLoader<'info, PointsMapping>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializePointsAccount<'info> {
    pub points_mapping: AccountLoader<'info, PointsMapping>,

    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AccruePoints<'info> {
    pub points_mapping: AccountLoader<'info, PointsMapping>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}