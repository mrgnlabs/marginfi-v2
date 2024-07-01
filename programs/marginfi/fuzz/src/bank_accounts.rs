use crate::log;

use anchor_lang::prelude::{AccountInfo, ProgramError, Pubkey};
use anchor_lang::Key;

use pyth_sdk_solana::state::SolanaPriceAccount;

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
    pub token_program: AccountInfo<'info>,
    pub mint_decimals: u8,
}

impl<'bump> BankAccounts<'bump> {
    pub fn refresh_oracle(&self, timestamp: i64) -> Result<(), ProgramError> {
        let mut data = self.oracle.try_borrow_mut_data()?;
        let data = bytemuck::from_bytes_mut::<SolanaPriceAccount>(&mut data);

        data.timestamp = timestamp;

        Ok(())
    }

    pub fn update_oracle(&self, updated_price: i64) -> Result<(), ProgramError> {
        let mut data = self.oracle.try_borrow_mut_data()?;
        let data = bytemuck::from_bytes_mut::<SolanaPriceAccount>(&mut data);

        data.agg.price = max(updated_price, 0);
        data.ema_price.val = max(updated_price, 0);
        data.ema_price.numer = max(updated_price, 0);

        Ok(())
    }

    pub fn log_oracle_price(&self) -> Result<(), ProgramError> {
        log!(
            "Oracle price: {}",
            bytemuck::from_bytes::<SolanaPriceAccount>(&self.oracle.try_borrow_data()?)
                .ema_price
                .val
        );

        Ok(())
    }
}

pub fn get_bank_map<'a, 'bump>(
    banks: &'a [BankAccounts<'bump>],
) -> HashMap<Pubkey, &'a BankAccounts<'bump>> {
    HashMap::from_iter(banks.iter().map(|bank| (bank.bank.key(), bank)))
}
