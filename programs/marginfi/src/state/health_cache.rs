use super::{marginfi_account::MAX_LENDING_ACCOUNT_BALANCES, marginfi_group::WrappedI80F48};
use crate::{assert_struct_align, assert_struct_size};
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use type_layout::TypeLayout;

assert_struct_size!(HealthCache, 208);
assert_struct_align!(HealthCache, 8);
#[repr(C)]
#[derive(
    AnchorDeserialize, AnchorSerialize, Copy, Clone, Zeroable, Pod, PartialEq, Eq, TypeLayout,
)]
pub struct HealthCache {
    pub asset_value: WrappedI80F48,
    pub liability_value: WrappedI80F48,
    /// Unix timestamp from the system clock when this cache was last updated
    pub timestamp: i64,
    /// The flags that indicate the state of the health cache This is u64 bitfield, where each bit
    /// represents a flag.
    ///
    /// * HEALTHY = 1 - If set, the account cannot be liquidated. If 0, the account is unhealthy and
    ///   can be liquidated.
    /// * 2, 4, 8, 16, 32, 64, 128, etc - reserved for future use
    pub flags: u64,
    /// Each price corresponds to that index of Balances in the LendingAccount. Useful for debugging
    /// or liquidator consumption, to determine how a user's position is priced internally.
    pub prices: [u64; MAX_LENDING_ACCOUNT_BALANCES],

    pub _padding: [u8; 32],
}
