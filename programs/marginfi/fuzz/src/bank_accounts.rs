use crate::log;

use anchor_lang::prelude::{AccountInfo, ProgramError, Pubkey};
use anchor_lang::Key;

use pyth_sdk_solana::state::PriceAccount;

use std::cmp::max;
use std::collections::HashMap;

pub struct BankAccounts<'info> {
    pub bank: AccountInfo<'info>,
    pub oracle: AccountInfo<'info>,
    pub liquidity_vault: AccountInfo<'info>,
    pub liquidity_vault_authority: AccountInfo<'info>,
    pub insurance_vault: AccountInfo<'info>,
    pub insurance_vault_authority: AccountInfo<'info>,
    pub fee_vault: AccountInfo<'info>,
    pub fee_vault_authority: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub mint_decimals: u8,
}

impl<'bump> BankAccounts<'bump> {
    pub fn refresh_oracle(&self, timestamp: i64) -> Result<(), ProgramError> {
        let mut data = self.oracle.try_borrow_mut_data()?;
        let data = bytemuck::from_bytes_mut::<PriceAccount>(&mut data);

        data.timestamp = timestamp;

        Ok(())
    }

    pub fn update_oracle(&self, price_change: i64) -> Result<(), ProgramError> {
        let mut data = self.oracle.try_borrow_mut_data()?;
        let data = bytemuck::from_bytes_mut::<PriceAccount>(&mut data);

        data.agg.price = max(data.agg.price + price_change, 0);
        data.ema_price.val = max(data.ema_price.val + price_change, 0);
        data.ema_price.numer = max(data.ema_price.numer + price_change, 0);

        Ok(())
    }

    pub fn log_oracle_price(&self) -> Result<(), ProgramError> {
        let data = self.oracle.try_borrow_data()?;
        let data = bytemuck::from_bytes::<PriceAccount>(&data);

        log!("Oracle price: {}", data.ema_price.val);

        Ok(())
    }
}

pub fn get_bank_map<'bump>(
    banks: &'bump [BankAccounts<'bump>],
) -> HashMap<Pubkey, &'bump BankAccounts<'bump>> {
    HashMap::from_iter(banks.iter().map(|bank| (bank.bank.key(), bank.clone())))
}
