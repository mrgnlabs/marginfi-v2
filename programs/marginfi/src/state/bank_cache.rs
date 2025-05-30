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
    /// Actual interest/fee rates of the bank.
    /// * APR (annual percentage rate) values
    pub interest_rates: WrappedInterestRates,
    pub accumulated_since_last_update: WrappedI80F48,
    _reserved0: [u8; 48],
}

impl Default for BankCache {
    fn default() -> Self {
        Self::zeroed()
    }
}

assert_struct_size!(WrappedInterestRates, 96);
assert_struct_align!(WrappedInterestRates, 8);
#[repr(C)]
#[derive(
    AnchorDeserialize,
    AnchorSerialize,
    Copy,
    Clone,
    Zeroable,
    Pod,
    PartialEq,
    Eq,
    TypeLayout,
    Debug,
    Default,
)]
pub struct WrappedInterestRates {
    pub base_rate: WrappedI80F48,
    pub lending_rate: WrappedI80F48,
    pub borrowing_rate: WrappedI80F48,
    pub group_fee: WrappedI80F48,
    pub insurance_fee: WrappedI80F48,
    pub protocol_fee: WrappedI80F48,
}

impl BankCache {
    pub fn update_interest_rates(&mut self, interest_rates: &ComputedInterestRates) {
        self.interest_rates.base_rate = interest_rates.base_rate_apr.into();
        self.interest_rates.lending_rate = interest_rates.lending_rate_apr.into();
        self.interest_rates.borrowing_rate = interest_rates.borrowing_rate_apr.into();
        self.interest_rates.group_fee = interest_rates.group_fee_apr.into();
        self.interest_rates.insurance_fee = interest_rates.insurance_fee_apr.into();
        self.interest_rates.protocol_fee = interest_rates.protocol_fee_apr.into();
    }

    pub fn get_interest_rates(&self) -> ComputedInterestRates {
        ComputedInterestRates {
            base_rate_apr: self.interest_rates.base_rate.into(),
            lending_rate_apr: self.interest_rates.lending_rate.into(),
            borrowing_rate_apr: self.interest_rates.borrowing_rate.into(),
            group_fee_apr: self.interest_rates.group_fee.into(),
            insurance_fee_apr: self.interest_rates.insurance_fee.into(),
            protocol_fee_apr: self.interest_rates.protocol_fee.into(),
        }
    }
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
