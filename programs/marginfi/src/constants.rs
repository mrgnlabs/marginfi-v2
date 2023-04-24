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

pub const EMISSIONS_AUTH_SEED: &str = "emissions_auth_seed";
pub const EMISSIONS_TOKEN_ACCOUNT_SEED: &str = "emissions_token_account_seed";

cfg_if::cfg_if! {
    if #[cfg(feature = "mainnet-beta")] {
        pub const PYTH_ID: Pubkey = pubkey!("FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH");
    } else if #[cfg(feature = "devnet")] {
        pub const PYTH_ID: Pubkey = pubkey!("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s");
    } else {
        pub const PYTH_ID: Pubkey = pubkey!("5rYvdyWAunZgD2EC1aKo7hQbutUUnkt7bBFM6xNq2z7Z");
    }
}

/// TODO: Make these variable per bank
pub const LIQUIDATION_LIQUIDATOR_FEE: I80F48 = I80F48!(0.025);
pub const LIQUIDATION_INSURANCE_FEE: I80F48 = I80F48!(0.025);

pub const SECONDS_PER_YEAR: I80F48 = I80F48!(31_536_000);

pub const MAX_PRICE_AGE_SEC: u64 = 20;

/// Range that contains 95% price data distribution
///
/// https://docs.pyth.network/pythnet-price-feeds/best-practices#confidence-intervals
pub const CONF_INTERVAL_MULTIPLE: I80F48 = I80F48!(2.12);

pub const USDC_EXPONENT: i32 = 6;

pub const MAX_ORACLE_KEYS: usize = 5;

/// Any balance below 1 SPL token amount is treated as none,
/// this is to account for any artifacts resulting from binary fraction arithemtic.
pub const EMPTY_BALANCE_THRESHOLD: I80F48 = I80F48!(1);

/// Comparios threshold used to account for arithmetic artifacts on balances
pub const ZERO_AMOUNT_THRESHOLD: I80F48 = I80F48!(0.0001);

pub const EMISSIONS_FLAG_BORROW_ACTIVE: u64 = 1 << 0;
pub const EMISSIONS_FLAG_LENDING_ACTIVE: u64 = 1 << 1;

/// Cutoff timestamp for balance last_update used in accounting collected emissions.
/// Any balance updates before this timestamp are ignored, and current_timestamp is used instead.
pub const MIN_EMISSIONS_START_TIME: u64 = 1681989983;

/// This constant combines the number of seconds per year, and the scale of the emissions rate (1e+6) to save on computation.
pub const EMISSION_CALC_SECS_PER_YEAR: I80F48 = I80F48!(31_536_000_000_000);
