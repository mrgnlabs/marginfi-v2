use crate::utils::big_query::DATE_FORMAT_STR;
use crate::utils::snapshot::{BankAccounts, OracleData, Snapshot};
use anyhow::anyhow;
use chrono::{NaiveDateTime, Utc};
use fixed::types::I80F48;
use itertools::Itertools;
use marginfi::constants::ZERO_AMOUNT_THRESHOLD;
use marginfi::prelude::MarginfiGroup;
use marginfi::state::marginfi_account::{
    calc_asset_value, MarginfiAccount, RiskEngine, RiskRequirementType,
};
use serde::Serialize;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct MarginfiGroupMetricsRow {
    pub id: String,
    pub created_at: String,
    pub timestamp: String,
    pub pubkey: String,
    pub marginfi_accounts_count: u32,
    pub banks_count: u32,
    pub mints_count: u32,
    pub total_assets_in_usd: f64,
    pub total_liabilities_in_usd: f64,
}

#[derive(Debug)]
pub struct MarginfiGroupMetrics {
    pub timestamp: i64,
    pub pubkey: Pubkey,
    pub marginfi_accounts_count: u32,
    pub banks_count: u32,
    pub mints_count: u32,
    pub total_assets_in_usd: f64,
    pub total_liabilities_in_usd: f64,
}

impl MarginfiGroupMetrics {
    pub fn new(
        timestamp: i64,
        marginfi_group_pk: &Pubkey,
        _marginfi_group: &MarginfiGroup,
        snapshot: &Snapshot,
    ) -> anyhow::Result<Self> {
        let group_banks_iter = snapshot
            .banks
            .iter()
            .filter(|(_, bank_accounts)| bank_accounts.bank.group.eq(marginfi_group_pk));
        let group_marginfi_accounts_iter = snapshot
            .marginfi_accounts
            .iter()
            .filter(|(_, marginfi_account)| marginfi_account.group.eq(marginfi_group_pk));

        let (total_assets_usd, total_liabilities_usd) = group_banks_iter.clone().try_fold(
            (0.0, 0.0),
            |mut sums, (bank_pk, bank_accounts)| -> anyhow::Result<(f64, f64)> {
                let total_asset_share = bank_accounts.bank.total_asset_shares;
                let total_liability_share = bank_accounts.bank.total_liability_shares;
                let price_feed_pk = bank_accounts.bank.config.get_pyth_oracle_key();
                let price = snapshot
                    .price_feeds
                    .get(&price_feed_pk)
                    .ok_or_else(|| {
                        anyhow!(
                            "Price feed {} not found for bank {}",
                            price_feed_pk,
                            bank_pk
                        )
                    })?
                    .get_price();

                let asset_value_usd = calc_asset_value(
                    bank_accounts
                        .bank
                        .get_asset_amount(total_asset_share.into())?,
                    price,
                    bank_accounts.bank.mint_decimals,
                    None,
                )?
                    .to_num::<f64>();
                let liability_value_usd = calc_asset_value(
                    bank_accounts
                        .bank
                        .get_liability_amount(total_liability_share.into())?,
                    price,
                    bank_accounts.bank.mint_decimals,
                    None,
                )?
                    .to_num::<f64>();

                sums.0 += asset_value_usd;
                sums.1 += liability_value_usd;

                Ok(sums)
            },
        )?;

        Ok(Self {
            timestamp,
            pubkey: *marginfi_group_pk,
            marginfi_accounts_count: group_marginfi_accounts_iter.count() as u32,
            banks_count: group_banks_iter.clone().count() as u32,
            mints_count: group_banks_iter
                .unique_by(|(_, bank_accounts)| bank_accounts.bank.mint)
                .collect_vec()
                .len() as u32,
            total_assets_in_usd: total_assets_usd,
            total_liabilities_in_usd: total_liabilities_usd,
        })
    }

    pub fn to_row(&self) -> MarginfiGroupMetricsRow {
        MarginfiGroupMetricsRow {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now().format(DATE_FORMAT_STR).to_string(),
            timestamp: NaiveDateTime::from_timestamp_opt(self.timestamp, 0)
                .unwrap()
                .format(DATE_FORMAT_STR)
                .to_string(),
            pubkey: self.pubkey.to_string(),
            marginfi_accounts_count: self.marginfi_accounts_count,
            banks_count: self.banks_count,
            mints_count: self.mints_count,
            total_assets_in_usd: self.total_assets_in_usd,
            total_liabilities_in_usd: self.total_liabilities_in_usd,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LendingPoolBankMetricsRow {
    pub id: String,
    pub created_at: String,
    pub timestamp: String,
    pub pubkey: String,
    pub mint: String,
    pub deposit_limit_in_tokens: f64,
    pub borrow_limit_in_tokens: f64,
    pub deposit_limit_in_usd: f64,
    pub borrow_limit_in_usd: f64,
    pub lenders_count: u32,
    pub borrowers_count: u32,
    pub total_assets_in_tokens: f64,
    pub total_liabilities_in_tokens: f64,
    pub total_assets_in_usd: f64,
    pub total_liabilities_in_usd: f64,
    pub liquidity_vault_balance: f64,
    pub insurance_vault_balance: f64,
    pub fee_vault_balance: f64,
}

#[derive(Debug)]
pub struct LendingPoolBankMetrics {
    pub timestamp: i64,
    pub pubkey: Pubkey,
    pub mint: Pubkey,
    pub deposit_limit_in_tokens: f64,
    pub borrow_limit_in_tokens: f64,
    pub deposit_limit_in_usd: f64,
    pub borrow_limit_in_usd: f64,
    pub lenders_count: u32,
    pub borrowers_count: u32,
    pub total_assets_in_tokens: f64,
    pub total_liabilities_in_tokens: f64,
    pub total_assets_in_usd: f64,
    pub total_liabilities_in_usd: f64,
    pub liquidity_vault_balance: f64,
    pub insurance_vault_balance: f64,
    pub fee_vault_balance: f64,
}

impl LendingPoolBankMetrics {
    pub fn new(
        timestamp: i64,
        bank_pk: &Pubkey,
        bank_accounts: &BankAccounts,
        snapshot: &Snapshot,
    ) -> anyhow::Result<Self> {
        let total_asset_share = bank_accounts.bank.total_asset_shares;
        let total_liability_share = bank_accounts.bank.total_liability_shares;
        let price_feed_pk = bank_accounts.bank.config.get_pyth_oracle_key();
        let price = snapshot
            .price_feeds
            .get(&price_feed_pk)
            .ok_or_else(|| {
                anyhow!(
                    "Price feed {} not found for bank {}",
                    price_feed_pk,
                    bank_pk
                )
            })?
            .get_price();

        let deposit_limit_usd = calc_asset_value(
            bank_accounts.bank.config.deposit_limit.into(),
            price,
            bank_accounts.bank.mint_decimals,
            None,
        )?
            .to_num::<f64>();
        let borrow_limit_usd = calc_asset_value(
            bank_accounts.bank.config.borrow_limit.into(),
            price,
            bank_accounts.bank.mint_decimals,
            None,
        )?
            .to_num::<f64>();

        let asset_amount = bank_accounts
            .bank
            .get_asset_amount(total_asset_share.into())?;
        let asset_value_usd =
            calc_asset_value(asset_amount, price, bank_accounts.bank.mint_decimals, None)?
                .to_num::<f64>();
        let liability_amount = bank_accounts
            .bank
            .get_liability_amount(total_liability_share.into())?;
        let liability_value_usd = calc_asset_value(
            liability_amount,
            price,
            bank_accounts.bank.mint_decimals,
            None,
        )?
            .to_num::<f64>();

        Ok(Self {
            timestamp,
            pubkey: *bank_pk,
            mint: bank_accounts.bank.mint,
            deposit_limit_in_tokens: bank_accounts.bank.config.deposit_limit as f64,
            borrow_limit_in_tokens: bank_accounts.bank.config.borrow_limit as f64,
            deposit_limit_in_usd: deposit_limit_usd,
            borrow_limit_in_usd: borrow_limit_usd,
            lenders_count: snapshot
                .marginfi_accounts
                .iter()
                .filter(|(_, account)| {
                    account.lending_account.balances.iter().any(|a| {
                        a.active
                            && I80F48::from(a.asset_shares).gt(&ZERO_AMOUNT_THRESHOLD)
                            && a.bank_pk.eq(bank_pk)
                    })
                })
                .count() as u32,
            borrowers_count: snapshot
                .marginfi_accounts
                .iter()
                .filter(|(_, account)| {
                    account.lending_account.balances.iter().any(|a| {
                        a.active
                            && I80F48::from(a.liability_shares).gt(&ZERO_AMOUNT_THRESHOLD)
                            && a.bank_pk.eq(bank_pk)
                    })
                })
                .count() as u32,
            total_assets_in_tokens: asset_amount.to_num::<f64>()
                / (10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            total_liabilities_in_tokens: liability_amount.to_num::<f64>()
                / (10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            total_assets_in_usd: asset_value_usd,
            total_liabilities_in_usd: liability_value_usd,
            liquidity_vault_balance: (bank_accounts.liquidity_vault_token_account.amount as f64)
                / (10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            insurance_vault_balance: (bank_accounts.insurance_vault_token_account.amount as f64)
                / (10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
            fee_vault_balance: (bank_accounts.fee_vault_token_account.amount as f64)
                / (10i64.pow(bank_accounts.bank.mint_decimals as u32) as f64),
        })
    }

    pub fn to_row(&self) -> LendingPoolBankMetricsRow {
        LendingPoolBankMetricsRow {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now().format(DATE_FORMAT_STR).to_string(),
            timestamp: NaiveDateTime::from_timestamp_opt(self.timestamp, 0)
                .unwrap()
                .format(DATE_FORMAT_STR)
                .to_string(),
            pubkey: self.pubkey.to_string(),
            mint: self.mint.to_string(),
            deposit_limit_in_tokens: self.deposit_limit_in_tokens,
            borrow_limit_in_tokens: self.borrow_limit_in_tokens,
            deposit_limit_in_usd: self.deposit_limit_in_usd,
            borrow_limit_in_usd: self.borrow_limit_in_usd,
            lenders_count: self.lenders_count,
            borrowers_count: self.borrowers_count,
            total_assets_in_tokens: self.total_assets_in_tokens,
            total_liabilities_in_tokens: self.total_liabilities_in_tokens,
            total_assets_in_usd: self.total_assets_in_usd,
            total_liabilities_in_usd: self.total_liabilities_in_usd,
            liquidity_vault_balance: self.liquidity_vault_balance,
            insurance_vault_balance: self.insurance_vault_balance,
            fee_vault_balance: self.fee_vault_balance,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MarginfiAccountMetricsRow {
    pub id: String,
    pub created_at: String,
    pub timestamp: String,
    pub pubkey: String,
    pub marginfi_group: String,
    pub owner: String,
    pub total_assets_in_usd: f64,
    pub total_liabilities_in_usd: f64,
    pub health: f64,
}

#[derive(Debug)]
pub struct MarginfiAccountMetrics {
    pub timestamp: i64,
    pub pubkey: Pubkey,
    pub marginfi_group: Pubkey,
    pub owner: Pubkey,
    pub total_assets_in_usd: f64,
    pub total_liabilities_in_usd: f64,
    pub health: f64,
}

impl MarginfiAccountMetrics {
    pub fn new(
        timestamp: i64,
        marginfi_account_pk: &Pubkey,
        marginfi_account: &MarginfiAccount,
        snapshot: &Snapshot,
    ) -> anyhow::Result<Self> {
        let risk_engine = RiskEngine::new(
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
        )?;
        let health = risk_engine
            .get_account_health(RiskRequirementType::Maintenance, timestamp)?
            .to_num::<f64>();

        let (total_assets_usd, total_liabilities_usd) = marginfi_account
            .lending_account
            .balances
            .iter()
            .filter(|balance| balance.active)
            .try_fold(
                (0.0, 0.0),
                |mut sums, balance| -> anyhow::Result<(f64, f64)> {
                    let bank = snapshot
                        .banks
                        .get(&balance.bank_pk)
                        .ok_or_else(|| {
                            anyhow!(
                                "Bank {} not found for marginfi account {}",
                                balance.bank_pk,
                                marginfi_account_pk
                            )
                        })?
                        .bank;

                    let total_asset_share = balance.asset_shares;
                    let total_liability_share = balance.liability_shares;
                    let price_feed_pk = bank.config.get_pyth_oracle_key();
                    let price = snapshot
                        .price_feeds
                        .get(&price_feed_pk)
                        .ok_or_else(|| {
                            anyhow!(
                                "Price feed {} not found for bank {}",
                                price_feed_pk,
                                balance.bank_pk
                            )
                        })?
                        .get_price();

                    let asset_value_usd = calc_asset_value(
                        bank.get_asset_amount(total_asset_share.into())?,
                        price,
                        bank.mint_decimals,
                        None,
                    )?
                        .to_num::<f64>();
                    let liability_value_usd = calc_asset_value(
                        bank.get_liability_amount(total_liability_share.into())?,
                        price,
                        bank.mint_decimals,
                        None,
                    )?
                        .to_num::<f64>();

                    sums.0 += asset_value_usd;
                    sums.1 += liability_value_usd;

                    Ok(sums)
                },
            )?;

        Ok(Self {
            timestamp,
            pubkey: *marginfi_account_pk,
            marginfi_group: marginfi_account.group,
            owner: marginfi_account.authority,
            total_assets_in_usd: total_assets_usd,
            total_liabilities_in_usd: total_liabilities_usd,
            health,
        })
    }

    pub fn to_row(&self) -> MarginfiAccountMetricsRow {
        MarginfiAccountMetricsRow {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now().format(DATE_FORMAT_STR).to_string(),
            timestamp: NaiveDateTime::from_timestamp_opt(self.timestamp, 0)
                .unwrap()
                .format(DATE_FORMAT_STR)
                .to_string(),
            pubkey: self.pubkey.to_string(),
            marginfi_group: self.marginfi_group.to_string(),
            owner: self.owner.to_string(),
            total_assets_in_usd: self.total_assets_in_usd,
            total_liabilities_in_usd: self.total_liabilities_in_usd,
            health: self.health,
        }
    }
}
