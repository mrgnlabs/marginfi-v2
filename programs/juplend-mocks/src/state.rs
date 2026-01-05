use anchor_lang::prelude::*;

use crate::JuplendMocksError;

/// Precision used for exchange prices in JupLend (1e12).
///
/// Source: JupLend lending program constant `EXCHANGE_PRICES_PRECISION`.
pub const EXCHANGE_PRICES_PRECISION: u128 = 1_000_000_000_000;

#[account]
pub struct Lending {
    pub mint: Pubkey,
    pub f_token_mint: Pubkey,

    pub lending_id: u16,

    /// number of decimals for the fToken, same as underlying mint
    pub decimals: u8,

    /// PDA of rewards rate model (LRRM)
    pub rewards_rate_model: Pubkey,

    /// exchange price in the liquidity layer (no rewards)
    pub liquidity_exchange_price: u64,

    /// exchange price between fToken and underlying (with rewards)
    pub token_exchange_price: u64,

    /// unix timestamp when exchange prices were updated last
    pub last_update_timestamp: u64,

    pub token_reserves_liquidity: Pubkey,
    pub supply_position_on_liquidity: Pubkey,

    pub bump: u8,
}

impl Lending {
    /// Returns true if the lending exchange rate is not updated for the current timestamp.
    ///
    /// Marginfi uses a strict equality check (same-slot/same-time) to ensure exact math.
    #[inline]
    pub fn is_stale(&self, current_timestamp: i64) -> bool {
        self.last_update_timestamp as i64 != current_timestamp
    }

    /// Core adjustment: raw * token_exchange_price / EXCHANGE_PRICES_PRECISION
    #[inline]
    fn adjust_u128(&self, raw: u128) -> Result<u128> {
        let rate = self.token_exchange_price as u128;
        raw.checked_mul(rate)
            .ok_or_else(|| error!(JuplendMocksError::MathError))?
            .checked_div(EXCHANGE_PRICES_PRECISION)
            .ok_or_else(|| error!(JuplendMocksError::MathError))
    }

    /// Adjust i64 oracle prices (Pyth price)
    #[inline]
    pub fn adjust_i64(&self, raw: i64) -> Result<i64> {
        let raw_u128 = u128::try_from(raw).map_err(|_| error!(JuplendMocksError::MathError))?;
        let adjusted = self.adjust_u128(raw_u128)?;
        adjusted
            .try_into()
            .map_err(|_| error!(JuplendMocksError::MathError))
    }

    /// Adjust u64 oracle confidences (Pyth confidence interval)
    #[inline]
    pub fn adjust_u64(&self, raw: u64) -> Result<u64> {
        let adjusted = self.adjust_u128(raw as u128)?;
        adjusted
            .try_into()
            .map_err(|_| error!(JuplendMocksError::MathError))
    }

    /// Adjust i128 oracle values (Switchboard Pull value/std_dev)
    #[inline]
    pub fn adjust_i128(&self, raw: i128) -> Result<i128> {
        let raw_u128 = u128::try_from(raw).map_err(|_| error!(JuplendMocksError::MathError))?;
        let adjusted = self.adjust_u128(raw_u128)?;
        adjusted
            .try_into()
            .map_err(|_| error!(JuplendMocksError::MathError))
    }
}
