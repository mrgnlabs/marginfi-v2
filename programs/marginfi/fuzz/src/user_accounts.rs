use std::collections::{HashMap, HashSet};

use anchor_lang::{
    prelude::{AccountInfo, AccountLoader, Pubkey},
    Key,
};
use fixed::types::I80F48;

use marginfi::state::marginfi_account::MarginfiAccount;

use crate::{arbitrary_helpers::BankIdx, bank_accounts::BankAccounts};

pub struct UserAccount<'info> {
    pub margin_account: AccountInfo<'info>,
    pub token_accounts: Vec<AccountInfo<'info>>,
}

impl<'info> UserAccount<'info> {
    pub fn new(
        margin_account: AccountInfo<'info>,
        token_accounts: Vec<AccountInfo<'info>>,
    ) -> Self {
        Self {
            margin_account,
            token_accounts,
        }
    }

    pub fn get_liquidation_banks(&self, banks: &[BankAccounts]) -> Option<(BankIdx, BankIdx)> {
        let marginfi_account_al =
            AccountLoader::<MarginfiAccount>::try_from(&self.margin_account).ok()?;
        let marginfi_account = marginfi_account_al.load().ok()?;

        // let bank_map = get_bank_map(banks);

        let mut asset_balances = marginfi_account
            .lending_account
            .balances
            .iter()
            .filter(|blc| !blc.is_empty(marginfi::state::marginfi_account::BalanceSide::Assets))
            .collect::<Vec<_>>();
        let mut liab_balances = marginfi_account
            .lending_account
            .balances
            .iter()
            .filter(|blc| {
                !blc.is_empty(marginfi::state::marginfi_account::BalanceSide::Liabilities)
            })
            .collect::<Vec<_>>();

        asset_balances
            .sort_by(|a, b| I80F48::from(a.asset_shares).cmp(&I80F48::from(b.asset_shares)));
        liab_balances.sort_by(|a, b| {
            I80F48::from(a.liability_shares).cmp(&I80F48::from(b.liability_shares))
        });

        let best_asset_bank = asset_balances.first()?.bank_pk;
        let best_liab_bank = liab_balances.first()?.bank_pk;

        let best_asset_pos = banks
            .iter()
            .position(|bank| bank.bank.key.eq(&best_asset_bank))?;
        let best_liab_pos = banks
            .iter()
            .position(|bank| bank.bank.key.eq(&best_liab_bank))?;

        Some((BankIdx(best_asset_pos as u8), BankIdx(best_liab_pos as u8)))
    }

    pub fn get_remaining_accounts(
        &self,
        bank_map: &HashMap<Pubkey, &BankAccounts<'info>>,
        include_banks: Vec<Pubkey>,
        exclude_banks: Vec<Pubkey>,
    ) -> Vec<AccountInfo<'info>> {
        let marginfi_account_al =
            AccountLoader::<MarginfiAccount>::try_from(&self.margin_account).unwrap();
        let marginfi_account = marginfi_account_al.load().unwrap();

        let mut already_included_banks = HashSet::new();

        let mut ais = marginfi_account
            .lending_account
            .balances
            .iter()
            .filter(|a| a.active && !exclude_banks.contains(&a.bank_pk))
            .flat_map(|balance| {
                let _bank_accounts = bank_map.get(&balance.bank_pk).unwrap();

                let bank_accounts = bank_map.get(&balance.bank_pk).unwrap();

                already_included_banks.insert(bank_accounts.bank.key());

                vec![bank_accounts.bank.clone(), bank_accounts.oracle.clone()]
            })
            .collect::<Vec<_>>();

        let missing_banks = include_banks
            .iter()
            .filter(|key| !already_included_banks.contains(key))
            .collect::<Vec<_>>();

        let mut missing_bank_ais = missing_banks
            .iter()
            .flat_map(|key| {
                let bank_accounts = bank_map.get(key).unwrap();
                vec![bank_accounts.bank.clone(), bank_accounts.oracle.clone()]
            })
            .collect::<Vec<AccountInfo>>();

        ais.append(&mut missing_bank_ais);

        ais
    }
}
