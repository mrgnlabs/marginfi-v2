use crate::DriftMocksError;
use anchor_lang::prelude::*;
use fixed::types::I80F48;

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

/// Calculates precision increase factor for converting between token decimals and Drift's internal precision.
///
/// # Important
/// Drift cannot support tokens with more than 19 decimals (DRIFT_PRECISION_EXP).
/// This is because Drift uses 10^(19 - decimals) for precision calculations, and tokens
/// with > 19 decimals would require a negative exponent (< 1.0 multiplier), which is
/// not supported in the u128 integer math.
///
/// # Errors
/// Returns `DriftMocksError::MathError` if `token_decimals > DRIFT_PRECISION_EXP` (19).
pub fn get_precision_increase(token_decimals: u32) -> Result<u128> {
    if token_decimals > DRIFT_PRECISION_EXP {
        return Err(DriftMocksError::MathError.into());
    }
    Ok(EXP_10[(DRIFT_PRECISION_EXP - token_decimals) as usize])
}

/// Scale a native deposit limit to Drift's fixed 9-decimal balance units.
pub fn scale_drift_deposit_limit(deposit_limit: u64, mint_decimals: u8) -> Result<I80F48> {
    let limit = I80F48::from_num(deposit_limit);

    if mint_decimals == DRIFT_SCALED_BALANCE_DECIMALS {
        return Ok(limit);
    }

    if mint_decimals < DRIFT_SCALED_BALANCE_DECIMALS {
        let diff = (DRIFT_SCALED_BALANCE_DECIMALS - mint_decimals) as usize;
        let scale = *EXP_10
            .get(diff)
            .ok_or_else(|| error!(DriftMocksError::MathError))?;
        return limit
            .checked_mul(I80F48::from_num(scale as i128))
            .ok_or_else(|| error!(DriftMocksError::MathError));
    }

    let diff = (mint_decimals - DRIFT_SCALED_BALANCE_DECIMALS) as usize;
    let scale = *EXP_10
        .get(diff)
        .ok_or_else(|| error!(DriftMocksError::MathError))?;
    limit
        .checked_div(I80F48::from_num(scale as i128))
        .ok_or_else(|| error!(DriftMocksError::MathError))
}
