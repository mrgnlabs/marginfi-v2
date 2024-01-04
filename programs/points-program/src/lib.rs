use anchor_lang::prelude::*;
use marginfi::state::{marginfi_group::WrappedI80F48, marginfi_account::MarginfiAccount};
use fixed::types::I80F48;
use std::ops::{Div, Mul, Add};
use anchor_lang::solana_program::log::sol_log_compute_units;

declare_id!("CSjewsFhiPYdz94HLCmntXPiFXUPbwhxUUFo29dVaYwo");

const MAX_POINTS_ACCOUNTS: usize = 25_000;

#[program]
pub mod points_program {
    use super::*;

    // the PointsMapping needs to be initialized outside the program because it's too large for CPIs
    // this endpoint is for zeroing everything out before starting to record points
    pub fn initialize_global_points(ctx: Context<InitializeGlobalPoints>) -> Result<()> {
        let mut points_mapping = ctx.accounts.points_mapping.load_init()?;

        *points_mapping = PointsMapping::default();

        Ok(())
    }

    pub fn initialize_points_account(ctx: Context<InitializePointsAccount>, initial_points: i128) -> Result<()> {
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
        // Note that the balances must be passed in as UI amounts, NOT native
        account_balance_datas: Vec<(Pubkey, AccountBalances)>, 
        price_data: Vec<(Pubkey, f64)>, 
        starting_index: usize,
        ) -> Result<()> {
            let mut points_mapping = ctx.accounts.points_mapping.load_mut()?;
            let clock = Clock::get()?;
        
            for (i, (_, account_balances)) in account_balance_datas.iter().enumerate() {
                if let Some(points_account) = points_mapping.points_accounts[starting_index + i].as_mut() {
                    let (current_asset_balance, current_liab_balance) = account_balances.get_account_balances(&price_data);
            
                    points_account.update_sma(current_asset_balance, current_liab_balance);

                    let unix_ts: u64 = clock.unix_timestamp.try_into().unwrap_or_default(); 
                    points_account.accrue_points(unix_ts);
                }
            }
        
            sol_log_compute_units();
        
            Ok(())
    }
}

#[account(zero_copy)]
pub struct PointsMapping {
    pub points_accounts: [Option<PointsAccount>; MAX_POINTS_ACCOUNTS],
    pub first_free_index: usize,
}

impl Default for PointsMapping {
    fn default() -> Self {
        PointsMapping {
            points_accounts: [None; MAX_POINTS_ACCOUNTS],
            first_free_index: 0,
        }
    }
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
#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct AccountBalances {
    pub balances: [Balance; 16]
}

impl AccountBalances {
    pub fn get_account_balances(&self, price_data: &Vec<(Pubkey, f64)>) -> (WrappedI80F48, WrappedI80F48) {
        let mut current_asset_balance = I80F48::from_num(0);
        let mut current_liab_balance = I80F48::from_num(0);
    
        for balance in self.balances {
            if balance.active {
                if let Some((_, price)) = price_data.iter().find(|(pk, _)| *pk == balance.bank_pk) {
                    let price_i80f48 = I80F48::from_num(*price);
                    current_asset_balance += I80F48::from_bits(balance.asset_shares.value) * price_i80f48;
                    current_liab_balance += I80F48::from_bits(balance.liability_shares.value) * price_i80f48;
                }
            }
        }
        
        (WrappedI80F48::from(current_asset_balance), WrappedI80F48::from(current_liab_balance))
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

impl From<marginfi::state::marginfi_account::Balance> for Balance {
    fn from(item: marginfi::state::marginfi_account::Balance) -> Self {
        Balance {
            active: item.active,
            bank_pk: item.bank_pk,
            asset_shares: item.asset_shares,
            liability_shares: item.liability_shares,
            emissions_outstanding: item.emissions_outstanding,
            last_update: item.last_update,
            _padding: item._padding,
        }
    }
}

impl PointsAccount {
    pub fn update_sma(&mut self, current_asset_balance: WrappedI80F48, current_liab_balance: WrappedI80F48) {
        let current_asset_sma_value = I80F48::from_bits(self.asset_sma.value);
        let current_asset_balance_i80f48 = I80F48::from_bits(current_asset_balance.value);

        let total_asset_value = current_asset_sma_value
            .mul(I80F48::from_num(self.sma_count as i128))
            .add(current_asset_balance_i80f48);
        self.asset_sma = WrappedI80F48::from(total_asset_value
            .div(I80F48::from_num(self.sma_count as i128 + 1)));

        let current_liab_sma_value = I80F48::from_bits(self.liab_sma.value);
        let current_liab_balance_i80f48 = I80F48::from_bits(current_liab_balance.value);

        let total_liab_value = current_liab_sma_value
            .mul(I80F48::from_num(self.sma_count as i128))
            .add(current_liab_balance_i80f48);                      
        self.liab_sma = WrappedI80F48::from(total_liab_value
            .div(I80F48::from_num(self.sma_count as i128 + 1)));

        self.sma_count += 1;    
    }


    // This function is for testing purposes in a Clock-less environment
    // We'll assume accrual every 30 seconds.
    pub fn accrue_points(&mut self, current_timestamp: u64) {
        let lending_rate_per_second = I80F48::from_num(1.0 / 86400.0); // 1 point per $1 lent per day
        let borrowing_rate_per_second = I80F48::from_num(4.0 / 86400.0); // 4 points per $1 borrowed per day

        let time_elapsed = 30; 

        let lending_points = I80F48::from_bits(self.asset_sma.value) * lending_rate_per_second * I80F48::from_num(time_elapsed);
        let borrowing_points = I80F48::from_bits(self.liab_sma.value) * borrowing_rate_per_second * I80F48::from_num(time_elapsed);

        self.points = WrappedI80F48::from(I80F48::from_bits(self.points.value) + lending_points + borrowing_points);
    }

    // This is disabled for now because it doesn't seem as if solana_program_test has a working Clock
    // pub fn accrue_points(&mut self, current_timestamp: u64) {
    //     let time_elapsed = current_timestamp - self.last_recorded_timestamp;
    
    //     if time_elapsed > 0 {
    //         // 1 point per $1 lent per 24h
    //         let lending_rate = I80F48::from_num(1.0 / (24.0 * 60.0 * 60.0)); // Points per dollar per second
    //         let lending_points = I80F48::from_bits(self.asset_sma.value) * lending_rate * I80F48::from_num(time_elapsed);
    
    //         // 4 points per $1 borrowed per 24h
    //         let borrowing_rate = I80F48::from_num(4.0 / (24.0 * 60.0 * 60.0)); // Points per dollar per second
    //         let borrowing_points = I80F48::from_bits(self.liab_sma.value) * borrowing_rate * I80F48::from_num(time_elapsed);
    
    //         self.points = WrappedI80F48::from(I80F48::from_bits(self.points.value) + lending_points + borrowing_points);
    
    //         self.last_recorded_timestamp = current_timestamp;
    //     }
    // }
}

#[derive(Accounts)]
pub struct InitializeGlobalPoints<'info> {
    #[account(zero)]
    pub points_mapping: AccountLoader<'info, PointsMapping>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializePointsAccount<'info> {
    #[account(mut)]
    pub points_mapping: AccountLoader<'info, PointsMapping>,

    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AccruePoints<'info> {
    #[account(mut)]
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