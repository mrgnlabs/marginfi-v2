use crate::{assert_struct_align, assert_struct_size};
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use fixed::types::I80F48;
use type_layout::TypeLayout;

use super::marginfi_group::WrappedI80F48;

assert_struct_size!(BankCache, 160);
assert_struct_align!(BankCache, 8);
#[repr(C)]
#[derive(
    AnchorDeserialize, AnchorSerialize, Copy, Clone, Zeroable, Pod, PartialEq, Eq, TypeLayout, Debug,
)]
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

impl BankCache {
    pub fn update_interest_rates(&mut self, interest_rates: &ComputedInterestRates) {
        self.base_rate = apr_to_u32(interest_rates.base_rate_apr);
        self.lending_rate = apr_to_u32(interest_rates.lending_rate_apr);
        self.borrowing_rate = apr_to_u32(interest_rates.borrowing_rate_apr);
    }
}

pub fn apr_to_u32(value: I80F48) -> u32 {
    let max_percent = I80F48::from_num(10.0); // 1000%
    let clamped = value.min(max_percent);
    let ratio = clamped / max_percent;
    (ratio * I80F48::from_num(u32::MAX)).to_num::<u32>()
}

#[derive(Debug, Clone)]
pub struct ComputedInterestRates {
    pub base_rate_apr: I80F48,
    pub lending_rate_apr: I80F48,
    pub borrowing_rate_apr: I80F48,
    pub group_fee_apr: I80F48,
    pub insurance_fee_apr: I80F48,
    pub protocol_fee_apr: I80F48,
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixed::types::I80F48;

    #[test]
    fn test_apr_to_u32_boundaries_and_midpoints() {
        let zero_apr = I80F48::from_num(0.0);
        let full_apr = I80F48::from_num(10.0); // 1000%
        let one_apr = I80F48::from_num(1.0); // 100%
        let five_apr = I80F48::from_num(5.0); // 500%
        let over_apr = I80F48::from_num(15.0); // over max

        assert_eq!(apr_to_u32(zero_apr), 0);
        assert_eq!(apr_to_u32(full_apr), u32::MAX);
        assert_eq!(apr_to_u32(one_apr), u32::MAX / 10);
        assert_eq!(apr_to_u32(five_apr), u32::MAX / 2);
        assert_eq!(apr_to_u32(over_apr), u32::MAX); // clamped by to_num::<u32>()
    }
}
