use bytemuck::{Pod, Zeroable};

use super::{Pubkey, WrappedI80F48};

#[repr(C)]
#[derive(Debug, PartialEq, Pod, Zeroable, Copy, Clone)]
pub struct MarginfiGroup {
    pub admin: Pubkey,
    /// Bitmask for group settings flags.
    /// * 0: `PROGRAM_FEES_ENABLED` If set, program-level fees are enabled.
    /// * 1: `ARENA_GROUP` If set, this is an arena group, which can only have two banks
    /// * Bits 1-63: Reserved for future use.
    pub group_flags: u64,
    /// Caches information from the global `FeeState` so the FeeState can be omitted on certain ixes
    pub fee_state_cache: FeeStateCache,
    // For groups initialized in versions 0.1.2 or greater (roughly the public launch of Arena),
    // this is an authoritative count of the number of banks under this group. For groups
    // initialized prior to 0.1.2, a non-authoritative count of the number of banks initiated after
    // 0.1.2 went live.
    pub banks: u16,
    pub pad0: [u8; 6],
    /// This admin can configure collateral ratios above (but not below) the collateral ratio of
    /// certain banks , e.g. allow SOL to count as 90% collateral when borrowing an LST instead of
    /// the default rate.
    pub emode_admin: Pubkey,

    pub _padding_0: [[u64; 2]; 24],
    pub _padding_1: [[u64; 2]; 32],
    pub _padding_3: u64,
    pub _padding_4: u64,
}

#[repr(C)]
#[derive(Debug, PartialEq, Pod, Zeroable, Copy, Clone)]
pub struct FeeStateCache {
    pub global_fee_wallet: Pubkey,
    pub program_fee_fixed: WrappedI80F48,
    pub program_fee_rate: WrappedI80F48,
}
