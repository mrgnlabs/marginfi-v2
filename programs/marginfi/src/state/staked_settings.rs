use anchor_lang::prelude::*;

use crate::{assert_struct_align, assert_struct_size};

use super::marginfi_group::{RiskTier, WrappedI80F48};

assert_struct_size!(StakedSettings, 256);
assert_struct_align!(StakedSettings, 8);

/// Unique per-group. Staked Collateral banks created under a group automatically use these
/// settings. Groups that have not created this struct cannot use staked collateral positions. When
/// this struct updates, changes must be permissionlessly propogated to staked collateral banks.
/// Administrators can also edit the bank manually, i.e. with configure_bank, to temporarily make
/// changes such as raising the deposit limit for a single bank.
#[account(zero_copy)]
#[repr(C)]
pub struct StakedSettings {
    /// This account's own key. A PDA derived from `marginfi_group` and `b"staked_settings"`
    pub key: Pubkey,
    /// Group for which these settings apply
    pub marginfi_group: Pubkey,
    /// Generally, the Pyth push oracle for SOL
    pub oracle: Pubkey,

    pub asset_weight_init: WrappedI80F48,
    pub asset_weight_maint: WrappedI80F48,

    pub deposit_limit: u64,
    pub total_asset_value_init_limit: u64,

    pub oracle_max_age: u16,
    pub risk_tier: RiskTier,
    _pad0: [u8; 5],

    /// The following values are irrelevant because staked collateral positions do not support
    /// borrowing.
    // * interest_config,
    // * liability_weight_init
    // * liability_weight_maint 
    // * borrow_limit

    _reserved0: [u8; 8],
    _reserved1: [u8; 32],
    _reserved2: [u8; 64],
}

impl StakedSettings {
    pub const LEN: usize = std::mem::size_of::<StakedSettings>();
}
