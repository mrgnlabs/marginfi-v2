use anchor_lang::prelude::*;
use marginfi::state::{marginfi_group::WrappedI80F48, marginfi_account::MarginfiAccount};
use fixed::types::I80F48;
use std::ops::{Div, Mul, Add};

declare_id!("CSjewsFhiPYdz94HLCmntXPiFXUPbwhxUUFo29dVaYwo");

const MAX_POINTS_ACCOUNTS: usize = 25_000;

#[program]
pub mod points_program {
    use super::*;

    // the PointsMapping needs to be initialized outside the program because it's too large for CPIs
    // this endpoint is for zeroing everything out before starting to record points
    pub fn initialize_global_points(ctx: Context<InitializeGlobalPoints>) -> Result<()> {
        let mut points_mapping = ctx.accounts.points_mapping.load_mut()?;

        // We can't #[derive(Default)] for [T, 25000] so we have to zero it out manually 
        for points_account in points_mapping.points_accounts.iter_mut() {
            *points_account = None;
        }

        points_mapping.first_free_index = 0;

        Ok(())
    }

    pub fn intialize_points_account(ctx: Context<InitializePointsAccount>, initial_points: i128) -> Result<()> {
        let mut points_mapping = ctx.accounts.points_mapping.load_mut()?;
        let first_free_index = points_mapping.first_free_index;

        require!(first_free_index < MAX_POINTS_ACCOUNTS, PointsError::NoFreeIndex);

        let clock = Clock::get().unwrap();
        let unix_ts: u64 = clock.unix_timestamp.try_into().unwrap();

        let new_points_account = PointsAccount {
            owner_mfi_account: ctx.accounts.marginfi_account.key(),
            points: WrappedI80F48 { value: initial_points },
            asset_sma: WrappedI80F48 { value: 0 },
            liab_sma: WrappedI80F48 { value: 0 },
            sma_count: 0,
            last_recorded_timestamp: unix_ts,
        };

        require!(points_mapping.points_accounts[first_free_index].is_none(), PointsError::FailedToInsert);

        points_mapping.points_accounts[first_free_index] = Some(new_points_account);

        points_mapping.first_free_index += 1;

        Ok(())
    }

    pub fn accrue_points(
        ctx: Context<AccruePoints>, 
        // For now, we'll assume that we can pass these in in order of occurrence in points_accounts
        // If we can't do that (which I'm not sure why we wouldn't be able to) we'll have a really hard time doing this efficiently
        account_balance_datas: Vec<(Pubkey, AccountBalances)>, 
        price_data: Vec<(Pubkey, i128)>, 
        starting_index: usize,
        ) -> Result<()> {
            let mut points_accounts = ctx.accounts.points_mapping.load_mut()?.points_accounts;
            let clock = Clock::get()?;

            for (i, (_, account_balances)) in account_balance_datas.iter().enumerate() {
                if let Some(points_account) = points_accounts.get_mut(starting_index + i) {
                    let (current_asset_balance, current_liab_balance) = account_balances.get_account_balances(&price_data);
        
                    points_account.unwrap().update_sma(current_asset_balance, current_liab_balance);
        
                    let unix_ts: u64 = clock.unix_timestamp.try_into().unwrap_or_default(); 
                    points_account.unwrap().accrue_points(unix_ts);
                } else {
                    continue;
                }
            }

        Ok(())
    }
}

#[account(zero_copy)]
pub struct PointsMapping {
    pub points_accounts: [Option<PointsAccount>; MAX_POINTS_ACCOUNTS],
    pub first_free_index: usize,
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

impl AccountBalances {
    pub fn get_account_balances(&self, price_data: &Vec<(Pubkey, i128)>) -> (i128, i128) {
        let mut current_asset_balance: i128 = 0;
        let mut current_liab_balance: i128 = 0;

        for balance in self.balances {
            if balance.active {
                if let Some((_, price)) = price_data.iter().find(|(pk, _)| *pk == balance.bank_pk) {
                    current_asset_balance += I80F48::from(balance.asset_shares).mul(I80F48::from_num(*price)).to_bits();

                    current_liab_balance += I80F48::from(balance.liability_shares).mul(I80F48::from_num(*price)).to_bits();
                }
            }
        }

        (current_asset_balance, current_liab_balance)
    }
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
    #[account(mut)]
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

#[error_code]
pub enum PointsError {
    #[msg("last_free_index is populated")]
    FailedToInsert,
    #[msg("last_free_index == MAX_POINTS_ACCOUNTS")]
    NoFreeIndex,
}