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
    if #[cfg(feature = "devnet")] {
        pub const PYTH_ID: Pubkey = pubkey!("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s");
    } else if #[cfg(feature = "mainnet-beta")] {
        pub const PYTH_ID: Pubkey = pubkey!("FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH");
    } else {
        pub const PYTH_ID: Pubkey = pubkey!("5rYvdyWAunZgD2EC1aKo7hQbutUUnkt7bBFM6xNq2z7Z");
    }
}

/// TODO: Make these variable per bank
pub const LIQUIDATION_LIQUIDATOR_FEE: I80F48 = I80F48!(0.025);
pub const LIQUIDATION_INSURANCE_FEE: I80F48 = I80F48!(0.025);

pub const SECONDS_PER_YEAR: I80F48 = I80F48!(31_536_000);

pub const MAX_PRICE_AGE_SEC: u64 = 60;

/// Range that contains 95% price data distribution
///
/// https://docs.pyth.network/pythnet-price-feeds/best-practices#confidence-intervals
pub const CONF_INTERVAL_MULTIPLE: I80F48 = I80F48!(2.12);
pub const MAX_CONF_INTERVAL: I80F48 = I80F48!(0.05);

pub const USDC_EXPONENT: i32 = 6;

pub const MAX_ORACLE_KEYS: usize = 5;

/// Any balance below 1 SPL token amount is treated as none,
/// this is to account for any artifacts resulting from binary fraction arithemtic.
pub const EMPTY_BALANCE_THRESHOLD: I80F48 = I80F48!(1);

/// Any account with assets below this threshold is considered bankrupt.
/// The account also needs to have more liabilities than assets.
///
/// This is USD denominated, so 0.001 = $0.1
pub const BANKRUPT_THRESHOLD: I80F48 = I80F48!(0.1);

/// Comparios threshold used to account for arithmetic artifacts on balances
pub const ZERO_AMOUNT_THRESHOLD: I80F48 = I80F48!(0.0001);

pub const EMISSIONS_FLAG_BORROW_ACTIVE: u64 = 1 << 0;
pub const EMISSIONS_FLAG_LENDING_ACTIVE: u64 = 1 << 1;

/// Cutoff timestamp for balance last_update used in accounting collected emissions.
/// Any balance updates before this timestamp are ignored, and current_timestamp is used instead.
pub const MIN_EMISSIONS_START_TIME: u64 = 1681989983;

pub const MAX_EXP_10_I80F48: usize = 24;
pub const EXP_10_I80F48: [I80F48; MAX_EXP_10_I80F48] = [
    I80F48!(1),                        // 10^0
    I80F48!(10),                       // 10^1
    I80F48!(100),                      // 10^2
    I80F48!(1000),                     // 10^3
    I80F48!(10000),                    // 10^4
    I80F48!(100000),                   // 10^5
    I80F48!(1000000),                  // 10^6
    I80F48!(10000000),                 // 10^7
    I80F48!(100000000),                // 10^8
    I80F48!(1000000000),               // 10^9
    I80F48!(10000000000),              // 10^10
    I80F48!(100000000000),             // 10^11
    I80F48!(1000000000000),            // 10^12
    I80F48!(10000000000000),           // 10^13
    I80F48!(100000000000000),          // 10^14
    I80F48!(1000000000000000),         // 10^15
    I80F48!(10000000000000000),        // 10^16
    I80F48!(100000000000000000),       // 10^17
    I80F48!(1000000000000000000),      // 10^18
    I80F48!(10000000000000000000),     // 10^19
    I80F48!(100000000000000000000),    // 10^20
    I80F48!(1000000000000000000000),   // 10^21
    I80F48!(10000000000000000000000),  // 10^22
    I80F48!(100000000000000000000000), // 10^23
];

pub const MAX_EXP_10: usize = 21;
pub const EXP_10: [i128; MAX_EXP_10] = [
    1,                     // 10^0
    10,                    // 10^1
    100,                   // 10^2
    1000,                  // 10^3
    10000,                 // 10^4
    100000,                // 10^5
    1000000,               // 10^6
    10000000,              // 10^7
    100000000,             // 10^8
    1000000000,            // 10^9
    10000000000,           // 10^10
    100000000000,          // 10^11
    1000000000000,         // 10^12
    10000000000000,        // 10^13
    100000000000000,       // 10^14
    1000000000000000,      // 10^15
    10000000000000000,     // 10^16
    100000000000000000,    // 10^17
    1000000000000000000,   // 10^18
    10000000000000000000,  // 10^19
    100000000000000000000, // 10^20
];

/// Value where total_asset_value_init_limit is considered inactive
pub const TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE: u64 = 0;
