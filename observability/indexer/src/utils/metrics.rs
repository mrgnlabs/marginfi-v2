use crate::utils::snapshot::{BankAccounts, OracleData, Snapshot};
use fixed::types::I80F48;
use itertools::Itertools;
use marginfi::constants::ZERO_AMOUNT_THRESHOLD;
use marginfi::state::marginfi_account::{calc_asset_value, MarginfiAccount, RiskEngine, RiskRequirementType};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

#[derive(Debug)]
pub struct MarginfiGroupMetrics {
    pub pubkey: Pubkey,
    pub total_marginfi_accounts: u64,
    pub total_banks: u64,
    pub total_mints: u64,
    pub total_assets_usd: f64,
    pub total_liabilities_usd: f64,
}

impl MarginfiGroupMetrics {
    pub fn new(snapshot: &Snapshot) -> Self {
        let (total_assets_usd, total_liabilities_usd) =
            snapshot
                .banks
                .iter()
                .fold((0.0, 0.0), |mut sums, (_, bank_accounts)| {
                    let total_asset_share = bank_accounts.bank.total_asset_shares;
                    let total_liability_share = bank_accounts.bank.total_liability_shares;
                    let price = snapshot
                        .price_feeds
                        .get(&bank_accounts.bank.config.get_pyth_oracle_key())
                        .unwrap()
                        .get_price();

                    let asset_value_usd = calc_asset_value(
                        bank_accounts
                            .bank
                            .get_asset_amount(total_asset_share.into())
                            .unwrap(),
                        price,
                        bank_accounts.bank.mint_decimals,
                        None,
                    )
                        .unwrap()
                        .to_num::<f64>();
                    let liability_value_usd = calc_asset_value(
                        bank_accounts
                            .bank
                            .get_liability_amount(total_liability_share.into())
                            .unwrap(),
                        price,
                        bank_accounts.bank.mint_decimals,
                        None,
                    )
                        .unwrap()
                        .to_num::<f64>();

                    sums.0 += asset_value_usd;
                    sums.1 += liability_value_usd;

                    sums
                });

        Self {
            pubkey: snapshot.marginfi_group.0,
            total_marginfi_accounts: snapshot.marginfi_accounts.len() as u64,
            total_banks: snapshot.banks.len() as u64,
            total_mints: snapshot
                .banks
                .iter()
                .unique_by(|(_, bank_accounts)| bank_accounts.bank.mint)
                .collect_vec()
                .len() as u64,
            total_assets_usd,
            total_liabilities_usd,
        }
    }
}

#[derive(Debug)]
pub struct LendingPoolBankMetrics {
    pub pubkey: Pubkey,
    pub mint: Pubkey,
    pub deposit_limit_token: f64,
    pub borrow_limit_token: f64,
    pub deposit_limit_usd: f64,
    pub borrow_limit_usd: f64,
    pub total_lenders: u64,
    pub total_borrowers: u64,
    pub total_assets_token: f64,
    pub total_liabilities_token: f64,
    pub total_assets_usd: f64,
    pub total_liabilities_usd: f64,
    pub liquidity_vault_balance: f64,
    pub insurance_vault_balance: f64,
    pub fee_vault_balance: f64,
}

impl LendingPoolBankMetrics {
    pub fn new(bank_pk: &Pubkey, bank_accounts: &BankAccounts, snapshot: &Snapshot) -> Self {
        let total_asset_share = bank_accounts.bank.total_asset_shares;
        let total_liability_share = bank_accounts.bank.total_liability_shares;
        let price = snapshot
            .price_feeds
            .get(&bank_accounts.bank.config.get_pyth_oracle_key())
            .unwrap()
            .get_price();


        let deposit_limit_usd =
            calc_asset_value(bank_accounts.bank.config.deposit_limit.into(), price, bank_accounts.bank.mint_decimals, None)
                .unwrap()
                .to_num::<f64>();
        let borrow_limit_usd =
            calc_asset_value(bank_accounts.bank.config.borrow_limit.into(), price, bank_accounts.bank.mint_decimals, None)
                .unwrap()
                .to_num::<f64>();

        let asset_amount = bank_accounts
            .bank
            .get_asset_amount(total_asset_share.into())
            .unwrap();
        let asset_value_usd =
            calc_asset_value(asset_amount, price, bank_accounts.bank.mint_decimals, None)
                .unwrap()
                .to_num::<f64>();
        let liability_amount = bank_accounts
            .bank
            .get_liability_amount(total_liability_share.into())
            .unwrap();
        let liability_value_usd = calc_asset_value(
            liability_amount,
            price,
            bank_accounts.bank.mint_decimals,
            None,
        )
            .unwrap()
            .to_num::<f64>();

        Self {
            pubkey: *bank_pk,
            mint: bank_accounts.bank.mint,
            deposit_limit_token: bank_accounts.bank.config.deposit_limit as f64,
            borrow_limit_token: bank_accounts.bank.config.borrow_limit as f64,
            deposit_limit_usd,
            borrow_limit_usd,
            total_lenders: snapshot
                .marginfi_accounts
                .iter()
                .filter(|(_, account)| {
                    account.lending_account.balances.iter().any(|a| {
                        a.active
                            && I80F48::from(a.asset_shares).gt(&ZERO_AMOUNT_THRESHOLD)
                            && a.bank_pk.eq(bank_pk)
                    })
                })
                .count() as u64,
            total_borrowers: snapshot
                .marginfi_accounts
                .iter()
                .filter(|(_, account)| {
                    account.lending_account.balances.iter().any(|a| {
                        a.active
                            && I80F48::from(a.liability_shares).gt(&ZERO_AMOUNT_THRESHOLD)
                            && a.bank_pk.eq(bank_pk)
                    })
                })
                .count() as u64,
            total_assets_token: asset_amount.to_num::<f64>()
                / (10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            total_liabilities_token: liability_amount.to_num::<f64>()
                / (10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            total_assets_usd: asset_value_usd,
            total_liabilities_usd: liability_value_usd,
            liquidity_vault_balance: (bank_accounts.liquidity_vault_token_account.amount as f64)
                / (10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            insurance_vault_balance: (bank_accounts.insurance_vault_token_account.amount as f64)
                / (10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            fee_vault_balance: (bank_accounts.fee_vault_token_account.amount as f64)
                / (10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
        }
    }
}

#[derive(Debug)]
pub struct MarginfiAccountMetrics {
    pub pubkey: Pubkey,
    pub total_assets_usd: f64,
    pub total_liabilities_usd: f64,
    pub health: f64,
}

impl MarginfiAccountMetrics {
    pub fn new(marginfi_account_pk: &Pubkey, marginfi_account: &MarginfiAccount, snapshot: &Snapshot) -> Self {
        let health = RiskEngine::new(
            marginfi_account,
            &HashMap::from_iter(
                snapshot
                    .banks
                    .iter()
                    .map(|(bank_pk, bank_accounts)| (*bank_pk, bank_accounts.clone().bank)),
            ),
            &HashMap::from_iter(snapshot.price_feeds.iter().map(|(oracle_pk, oracle_data)| {
                match oracle_data {
                    OracleData::Pyth(price_feed) => (*oracle_pk, *price_feed),
                }
            })),
        )
            .unwrap()
            .get_account_health(
                RiskRequirementType::Maintenance,
                snapshot.clock.unix_timestamp,
            )
            .unwrap()
            .to_num::<f64>();

        let (total_assets_usd, total_liabilities_usd) =
            marginfi_account.lending_account.balances
                .iter()
                .filter(|balance| balance.active)
                .fold((0.0, 0.0), |mut sums, balance| {
                    let bank = snapshot.banks.get(&balance.bank_pk).unwrap().bank;

                    let total_asset_share = balance.asset_shares;
                    let total_liability_share = balance.liability_shares;
                    let price = snapshot
                        .price_feeds
                        .get(&bank.config.get_pyth_oracle_key())
                        .unwrap()
                        .get_price();

                    let asset_value_usd = calc_asset_value(
                        bank
                            .get_asset_amount(total_asset_share.into())
                            .unwrap(),
                        price,
                        bank.mint_decimals,
                        None,
                    )
                        .unwrap()
                        .to_num::<f64>();
                    let liability_value_usd = calc_asset_value(
                        bank
                            .get_liability_amount(total_liability_share.into())
                            .unwrap(),
                        price,
                        bank.mint_decimals,
                        None,
                    )
                        .unwrap()
                        .to_num::<f64>();

                    sums.0 += asset_value_usd;
                    sums.1 += liability_value_usd;

                    sums
                });

        Self {
            pubkey: *marginfi_account_pk,
            total_assets_usd,
            total_liabilities_usd,
            health,
        }
    }
}
