use crate::DriftMocksError;
use anchor_lang::Result;

// Drift precision constants
pub const SPOT_BALANCE_PRECISION: u128 = 1_000_000_000; // 10^9
pub const SPOT_BALANCE_PRECISION_U64: u64 = 1_000_000_000; // 10^9
pub const SPOT_CUMULATIVE_INTEREST_PRECISION: u128 = 10_000_000_000; // expo = -10
pub const PRICE_PRECISION: i64 = 1_000_000; // 10^6
pub const PRICE_PRECISION_U64: u64 = 1_000_000; // 10^6
pub const PERCENTAGE_PRECISION: u128 = 1_000_000; // 10^6
pub const BASIS_PRECISION: u32 = 10_000; // 10^4
pub const BASIS_PRECISION_U128: u128 = 10_000; // 10^4

// Drift uses 10^(19 - decimals) for precision increase calculations
pub const DRIFT_PRECISION_EXP: u32 = 19;

// All Drift scaled balances use 9 decimal precision, regardless of the underlying token decimals
pub const DRIFT_SCALED_BALANCE_DECIMALS: u8 = 9;

// Maximum number of positions in drift
pub const MAX_SPOT_POSITIONS: usize = 8;
pub const MAX_PERP_POSITIONS: usize = 8;

// Balance type constants
pub const SPOT_BALANCE_TYPE_DEPOSIT: u8 = 0;
pub const SPOT_BALANCE_TYPE_BORROW: u8 = 1;

pub const MAX_EXP_10: usize = 20;
pub const EXP_10: [u128; MAX_EXP_10] = [
    1,                          // 10^0
    10,                         // 10^1
    100,                        // 10^2
    1_000,                      // 10^3
    10_000,                     // 10^4
    100_000,                    // 10^5
    1_000_000,                  // 10^6
    10_000_000,                 // 10^7
    100_000_000,                // 10^8
    1_000_000_000,              // 10^9
    10_000_000_000,             // 10^10
    100_000_000_000,            // 10^11
    1_000_000_000_000,          // 10^12
    10_000_000_000_000,         // 10^13
    100_000_000_000_000,        // 10^14
    1_000_000_000_000_000,      // 10^15
    10_000_000_000_000_000,     // 10^16
    100_000_000_000_000_000,    // 10^17
    1_000_000_000_000_000_000,  // 10^18
    10_000_000_000_000_000_000, // 10^19
];

pub fn get_precision_increase(token_decimals: u32) -> Result<u128> {
    let exponent = DRIFT_PRECISION_EXP
        .checked_sub(token_decimals)
        .ok_or(DriftMocksError::MathError)?;
    Ok(EXP_10[exponent as usize])
}
