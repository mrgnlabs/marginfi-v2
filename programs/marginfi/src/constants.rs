use anchor_lang::prelude::*;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use solana_program::pubkey;

pub const LIQUIDITY_VAULT_AUTHORITY_SEED: &[u8] = b"liquidity_vault_auth";
pub const INSURANCE_VAULT_AUTHORITY_SEED: &[u8] = b"insurance_vault_auth";
pub const FEE_VAULT_AUTHORITY_SEED: &[u8] = b"fee_vault_auth";

pub const LIQUIDITY_VAULT_SEED: &[u8] = b"liquidity_vault";
pub const INSURANCE_VAULT_SEED: &[u8] = b"insurance_vault";
pub const FEE_VAULT_SEED: &[u8] = b"fee_vault";

pub const LENDING_POOL_BANK_SEED: &[u8] = b"lending_pool_bank";

#[cfg(feature = "mainnet-beta")] // mainnet
pub const PYTH_ID: Pubkey = pubkey!("5rYvdyWAunZgD2EC1aKo7hQbutUUnkt7bBFM6xNq2z7Z");
#[cfg(feature = "devnet")] // devnet
pub const PYTH_ID: Pubkey = pubkey!("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s");
#[cfg(all(not(feature = "mainnet-beta"), not(feature = "devnet")))] // other
pub const PYTH_ID: Pubkey = pubkey!("5rYvdyWAunZgD2EC1aKo7hQbutUUnkt7bBFM6xNq2z7Z");

/// TODO: Make these variable per bank
pub const LIQUIDATION_LIQUIDATOR_FEE: I80F48 = I80F48!(0.025);
pub const LIQUIDATION_INSURANCE_FEE: I80F48 = I80F48!(0.025);

pub const SECONDS_PER_YEAR: I80F48 = I80F48!(31_536_000);

pub const MAX_PRICE_AGE_SEC: u64 = 20;

/// Range that contains 95% price data distribution
///
/// https://docs.pyth.network/pythnet-price-feeds/best-practices#confidence-intervals
pub const CONF_INTERVAL_MULTIPLE: I80F48 = I80F48!(4.24);

pub const USDC_EXPONENT: i32 = 6;
