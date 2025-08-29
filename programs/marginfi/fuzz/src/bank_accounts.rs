use crate::log;

use anchor_lang::prelude::{AccountInfo, ProgramError, Pubkey};
use anchor_lang::{AnchorDeserialize, AnchorSerialize, Discriminator, Key};

use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

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

        let mut price_update = PriceUpdateV2::deserialize(&mut &data[8..]).unwrap();

        price_update.price_message.publish_time = timestamp;
        price_update.price_message.prev_publish_time = timestamp;

        let mut new_data = vec![];
        let mut account_data = vec![];

        new_data.extend_from_slice(PriceUpdateV2::DISCRIMINATOR);

        price_update.serialize(&mut account_data).unwrap();

        new_data.extend_from_slice(&account_data);

        data.copy_from_slice(&new_data);

        Ok(())
    }

    pub fn update_oracle(&self, updated_price: i64) -> Result<(), ProgramError> {
        let mut data = self.oracle.try_borrow_mut_data()?;

        let mut price_update = PriceUpdateV2::deserialize(&mut &data[8..]).unwrap();

        price_update.price_message.price = max(updated_price, 0);
        price_update.price_message.ema_price = max(updated_price, 0);

        let mut new_data = vec![];
        let mut account_data = vec![];

        new_data.extend_from_slice(PriceUpdateV2::DISCRIMINATOR);

        price_update.serialize(&mut account_data).unwrap();

        new_data.extend_from_slice(&account_data);

        data.copy_from_slice(&new_data);

        Ok(())
    }

    pub fn log_oracle_price(&self) -> Result<(), ProgramError> {
        let data = self.oracle.try_borrow_data()?;
        let _price_update = PriceUpdateV2::deserialize(&mut &data[8..]).unwrap();
        log!("Oracle price: {}", _price_update.price_message.ema_price);

        Ok(())
    }
}

pub fn get_bank_map<'a, 'bump>(
    banks: &'a [BankAccounts<'bump>],
) -> HashMap<Pubkey, &'a BankAccounts<'bump>> {
    HashMap::from_iter(banks.iter().map(|bank| (bank.bank.key(), bank)))
}
