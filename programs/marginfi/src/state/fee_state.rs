use anchor_lang::prelude::*;

use crate::{assert_struct_align, assert_struct_size};

use super::marginfi_group::WrappedI80F48;

assert_struct_size!(FeeState, 256);
assert_struct_align!(FeeState, 8);

/// Unique per-program. The Program Owner uses this account to administrate fees collected by the protocol
#[account(zero_copy)]
#[repr(C)]
pub struct FeeState {
    /// The fee state's own key. A PDA derived from just `b"feestate"`
    pub key: Pubkey,
    /// Can modify fees
    pub global_fee_admin: Pubkey,
    /// The base wallet for all protocol fees. All SOL fees go to this wallet. All non-SOL fees go
    /// to the cannonical ATA of this wallet for that asset.
    pub global_fee_wallet: Pubkey,
    // Reserved for future use, forces 8-byte alignment
    pub placeholder0: u64,
    /// Flat fee assessed when a new bank is initialized, in lamports.
    /// * In SOL, in native decimals.
    pub bank_init_flat_sol_fee: u32,
    pub bump_seed: u8,
    // Pad to next 8-byte multiple
    _padding0: [u8; 4],
    // Pad to 128 bytes
    _padding1: [u8; 15],
    /// Fee collected by the program owner from all groups
    pub program_fee_fixed: WrappedI80F48,
    /// Fee collected by the program owner from all groups
    pub program_fee_rate: WrappedI80F48,
    /// Flat fee assessed at flashloan end, in lamports.
    /// * In SOL, in native decimals.
    pub flashloan_flat_sol_fee: u32,
    // Reserved for future use
    _reserved0: [u8; 28],
    _reserved1: [u8; 64],
}

impl FeeState {
    pub const LEN: usize = std::mem::size_of::<FeeState>();
}
