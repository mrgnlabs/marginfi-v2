use crate::utils::big_query::DATE_FORMAT_STR;
use crate::utils::snapshot::{BankAccounts, OracleData, Snapshot};
use anyhow::anyhow;
use chrono::{NaiveDateTime, Utc};
use fixed::types::I80F48;
use itertools::Itertools;
use marginfi::constants::ZERO_AMOUNT_THRESHOLD;
use marginfi::prelude::MarginfiGroup;
use marginfi::state::marginfi_account::{
    calc_asset_value, MarginfiAccount, RiskEngine, RiskRequirementType, WeightType,
};
use marginfi::state::marginfi_group::BankOperationalState;
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

        let (
            total_assets_usd,
            total_liabilities_usd,
            _total_assets_usd_maint,
            _total_liabilities_usd_maint,
        ) = group_banks_iter.clone().try_fold(
            (0.0, 0.0, 0.0, 0.0),
            |mut sums, (bank_pk, bank_accounts)| -> anyhow::Result<(f64, f64, f64, f64)> {
                let total_asset_share = bank_accounts.bank.total_asset_shares;
                let total_liability_share = bank_accounts.bank.total_liability_shares;
                let price_feed_pk = bank_accounts.bank.config.get_pyth_oracle_key();
                let (asset_weight, liability_weight) = bank_accounts
                    .bank
                    .config
                    .get_weights(WeightType::Maintenance);
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
                let asset_value_usd_maint = calc_asset_value(
                    bank_accounts
                        .bank
                        .get_asset_amount(total_asset_share.into())?,
                    price,
                    bank_accounts.bank.mint_decimals,
                    Some(asset_weight),
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
                let liability_value_usd_maint = calc_asset_value(
                    bank_accounts
                        .bank
                        .get_liability_amount(total_liability_share.into())?,
                    price,
                    bank_accounts.bank.mint_decimals,
                    Some(liability_weight),
                )?
                .to_num::<f64>();

                sums.0 += asset_value_usd;
                sums.1 += liability_value_usd;
                sums.2 += asset_value_usd_maint;
                sums.3 += liability_value_usd_maint;

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
    pub marginfi_group: String,
    pub mint: String,
    pub usd_price: f64,
    pub operational_state: String,
    pub asset_weight_maintenance: f64,
    pub liability_weight_maintenance: f64,
    pub asset_weight_initial: f64,
    pub liability_weight_initial: f64,
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
    pub marginfi_group: Pubkey,
    pub mint: Pubkey,
    pub usd_price: f64,
    pub operational_state: BankOperationalState,
    pub asset_weight_maintenance: f64,
    pub liability_weight_maintenance: f64,
    pub asset_weight_initial: f64,
    pub liability_weight_initial: f64,
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
        let (asset_weight_maintenance, liability_weight_maintenance) = bank_accounts
            .bank
            .config
            .get_weights(WeightType::Maintenance);
        let (asset_weight_initial, liability_weight_initial) =
            bank_accounts.bank.config.get_weights(WeightType::Initial);
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
            marginfi_group: bank_accounts.bank.group,
            mint: bank_accounts.bank.mint,
            usd_price: price.to_num::<f64>(),
            operational_state: bank_accounts.bank.config.operational_state,
            asset_weight_maintenance: asset_weight_maintenance.to_num::<f64>(),
            liability_weight_maintenance: liability_weight_maintenance.to_num::<f64>(),
            asset_weight_initial: asset_weight_initial.to_num::<f64>(),
            liability_weight_initial: liability_weight_initial.to_num::<f64>(),
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
            marginfi_group: self.marginfi_group.to_string(),
            mint: self.mint.to_string(),
            usd_price: self.usd_price,
            operational_state: self.operational_state.to_string(),
            asset_weight_maintenance: self.asset_weight_maintenance,
            liability_weight_maintenance: self.liability_weight_maintenance,
            asset_weight_initial: self.asset_weight_initial,
            liability_weight_initial: self.liability_weight_initial,
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
    pub total_assets_in_usd_maintenance: f64,
    pub total_liabilities_in_usd_maintenance: f64,
    pub total_assets_in_usd_initial: f64,
    pub total_liabilities_in_usd_initial: f64,
}

#[derive(Debug)]
pub struct MarginfiAccountMetrics {
    pub timestamp: i64,
    pub pubkey: Pubkey,
    pub marginfi_group: Pubkey,
    pub owner: Pubkey,
    pub total_assets_in_usd: f64,
    pub total_liabilities_in_usd: f64,
    pub total_assets_in_usd_maintenance: f64,
    pub total_liabilities_in_usd_maintenance: f64,
    pub total_assets_in_usd_initial: f64,
    pub total_liabilities_in_usd_initial: f64,
}

impl MarginfiAccountMetrics {
    pub fn new(
        timestamp: i64,
        marginfi_account_pk: &Pubkey,
        marginfi_account: &MarginfiAccount,
        snapshot: &Snapshot,
    ) -> anyhow::Result<Self> {
        let banks = HashMap::from_iter(
            snapshot
                .banks
                .iter()
                .map(|(bank_pk, bank_accounts)| (*bank_pk, bank_accounts.clone().bank)),
        );
        let price_feeds =
            HashMap::from_iter(snapshot.price_feeds.iter().map(|(oracle_pk, oracle_data)| {
                match oracle_data {
                    OracleData::Pyth(price_feed) => (*oracle_pk, *price_feed),
                }
            }));

        let risk_engine = RiskEngine::new(marginfi_account, &banks, &price_feeds)?;

        let (total_assets_usd, total_liabilities_usd) =
            risk_engine.get_equity_components(timestamp)?;
        let (total_assets_usd, total_liabilities_usd) = (
            total_assets_usd.to_num::<f64>(),
            total_liabilities_usd.to_num::<f64>(),
        );
        let (total_assets_usd_maintenance, total_liabilities_usd_maintenance) = risk_engine
            .get_account_health_components(RiskRequirementType::Maintenance, timestamp)?;
        let (total_assets_usd_maintenance, total_liabilities_usd_maintenance) = (
            total_assets_usd_maintenance.to_num::<f64>(),
            total_liabilities_usd_maintenance.to_num::<f64>(),
        );
        let (total_assets_usd_initial, total_liabilities_usd_initial) =
            risk_engine.get_account_health_components(RiskRequirementType::Initial, timestamp)?;
        let (total_assets_usd_initial, total_liabilities_usd_initial) = (
            total_assets_usd_initial.to_num::<f64>(),
            total_liabilities_usd_initial.to_num::<f64>(),
        );

        Ok(Self {
            timestamp,
            pubkey: *marginfi_account_pk,
            marginfi_group: marginfi_account.group,
            owner: marginfi_account.authority,
            total_assets_in_usd: total_assets_usd,
            total_liabilities_in_usd: total_liabilities_usd,
            total_assets_in_usd_maintenance: total_assets_usd_maintenance,
            total_liabilities_in_usd_maintenance: total_liabilities_usd_maintenance,
            total_assets_in_usd_initial: total_assets_usd_initial,
            total_liabilities_in_usd_initial: total_liabilities_usd_initial,
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
            total_assets_in_usd_maintenance: self.total_assets_in_usd_maintenance,
            total_liabilities_in_usd_maintenance: self.total_liabilities_in_usd_maintenance,
            total_assets_in_usd_initial: self.total_assets_in_usd_initial,
            total_liabilities_in_usd_initial: self.total_liabilities_in_usd_initial,
        }
    }
}
