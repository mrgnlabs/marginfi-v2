use crate::{assert_struct_align, assert_struct_size, types::WrappedI80F48};

use bytemuck::Zeroable;
#[cfg(feature = "anchor")]
use {anchor_lang::prelude::*, bytemuck::Pod, type_layout::TypeLayout};

assert_struct_size!(BankCache, 160);
assert_struct_align!(BankCache, 8);
#[repr(C)]
#[cfg_attr(
    feature = "anchor",
    derive(
        AnchorDeserialize,
        AnchorSerialize,
        Copy,
        Clone,
        Pod,
        PartialEq,
        Eq,
        TypeLayout,
    )
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

    _reserved0: [u8; 128],
}

impl Default for BankCache {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// Useful when converting an I80F48 apr into a BankCache u32 from 0-1000. Clamps to 1000% if
/// exceeding that amount. Invalid for negative inputs.
pub fn apr_to_u32(value: I80F48) -> u32 {
    let max_percent = I80F48::from_num(10.0); // 1000%
    let clamped = value.min(max_percent);
    let ratio = clamped / max_percent;
    (ratio * I80F48::from_num(u32::MAX)).to_num::<u32>()
}
