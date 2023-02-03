use anchor_lang::prelude::*;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use solana_program::pubkey;

pub const LIQUIDITY_VAULT_AUTHORITY_SEED: &str = "liquidity_vault_auth";
pub const INSURANCE_VAULT_AUTHORITY_SEED: &str = "insurance_vault_auth";
pub const FEE_VAULT_AUTHORITY_SEED: &str = "fee_vault_auth";

pub const LIQUIDITY_VAULT_SEED: &str = "liquidity_vault";
pub const INSURANCE_VAULT_SEED: &str = "insurance_vault";
pub const FEE_VAULT_SEED: &str = "fee_vault";

pub const SHARES_TOKEN_MINT_SEED: &str = "shares_token_mint";
pub const SHARES_TOKEN_MINT_AUTHORITY_SEED: &str = "shares_token_mint_auth";

#[cfg(feature = "mainnet-beta")] // mainnet
pub const PYTH_ID: Pubkey = pubkey!("FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH");
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

pub const MAX_ORACLE_KEYS: usize = 5;

/// Any balance below 1 SPL token amount is treated as none,
/// this is to account for any artifacts resulting from binary fraction arithemtic.
pub const EMPTY_BALANCE_THRESHOLD: I80F48 = I80F48!(1);
