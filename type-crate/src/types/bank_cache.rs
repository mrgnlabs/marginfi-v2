use crate::{assert_struct_align, assert_struct_size, types::WrappedI80F48};

use bytemuck::Zeroable;
#[cfg(feature = "anchor")]
use {anchor_lang::prelude::*, bytemuck::Pod};

assert_struct_size!(BankCache, 160);
assert_struct_align!(BankCache, 8);
#[repr(C)]
#[cfg_attr(
    feature = "anchor",
    derive(AnchorDeserialize, AnchorSerialize, Copy, Clone, Pod, PartialEq, Eq,)
)]
#[derive(Zeroable, Debug)]
/// A read-only cache of the bank's key metrics, e.g. spot interest/fee rates.
pub struct BankCache {
    /// Actual (spot) interest/fee rates of the bank, based on utilization
    /// * APR (annual percentage rate) values
    /// * From 0-1000%, as u32, e.g. u32::MAX = 1000%, u:32::MAX/2 = 500%, etc
    pub base_rate: u32,
    /// Equivalent to `base_rate` * utilization
    /// * From 0-1000%, as u32, e.g. u32::MAX = 1000%, u:32::MAX/2 = 500%, etc
    pub lending_rate: u32,
    /// Equivalent to `base_rate` * (1 + ir_fees) + fixed_fees
    /// * From 0-1000%, as u32, e.g. u32::MAX = 1000%, u:32::MAX/2 = 500%, etc
    pub borrowing_rate: u32,

    /// * in seconds
    pub interest_accumulated_for: u32,
    /// equivalent to (share value increase in the last `interest_accumulated_for` seconds *
    /// shares), i.e. the delta in `asset_share_value`, in token.
    /// * Note: if the tx that triggered this cache update increased or decreased the net shares,
    ///   this value still reports using the PRE-CHANGE share amount, since interest is always
    ///   earned on that amount.
    /// * in token, in native decimals, as I80F48
    pub accumulated_since_last_update: WrappedI80F48,

    /// Oracle price used in the last instruction that consumed an oracle price
    /// * Only updated when instruction uses an oracle price, not updated for operations that don't
    ///   require prices (e.g., deposit, repay)
    /// * Price in USD, with no price bias
    /// * Zero if never updated
    pub last_oracle_price: WrappedI80F48,

    /// Unix timestamp (seconds) when last_oracle_price was last updated
    /// * Used to determine staleness of cached price
    /// * Zero if never updated
    pub last_oracle_price_timestamp: i64,

    /// Confidence interval reported by the oracle when last_oracle_price was fetched
    /// * Always non-negative
    /// * Zero if never updated
    /// * Note: this value is the confidence reported by oracles, multiplied by `STD_DEV_MULTIPLE`
    pub last_oracle_price_confidence: WrappedI80F48,
    _padding: [u8; 24],
    _reserved0: [u8; 64],
}

impl Default for BankCache {
    fn default() -> Self {
        Self::zeroed()
    }
}
