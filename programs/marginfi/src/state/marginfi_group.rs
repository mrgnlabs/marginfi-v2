use crate::{assert_struct_size, prelude::MarginfiError, MarginfiResult};
use anchor_lang::prelude::borsh;
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use marginfi_type_crate::types::WrappedI80F48;
use std::fmt::Debug;
use type_layout::TypeLayout;

pub const PROGRAM_FEES_ENABLED: u64 = 1;
pub const ARENA_GROUP: u64 = 2;

assert_struct_size!(MarginfiGroup, 1056);
#[account(zero_copy)]
#[derive(Default, Debug, PartialEq, Eq, TypeLayout)]
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
    pub _padding_4: u64,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Zeroable, Pod, Debug, PartialEq, Eq,
)]
#[repr(C)]
pub struct FeeStateCache {
    pub global_fee_wallet: Pubkey,
    pub program_fee_fixed: WrappedI80F48,
    pub program_fee_rate: WrappedI80F48,
    pub last_update: i64,
}

impl MarginfiGroup {
    pub fn update_admin(&mut self, new_admin: Pubkey) {
        if self.admin == new_admin {
            msg!("No change to admin: {:?}", new_admin);
            // do nothing
        } else {
            msg!("Set admin from {:?} to {:?}", self.admin, new_admin);
            self.admin = new_admin;
        }
    }

    pub fn update_emode_admin(&mut self, new_emode_admin: Pubkey) {
        if self.emode_admin == new_emode_admin {
            msg!("No change to emode admin: {:?}", new_emode_admin);
            // do nothing
        } else {
            msg!(
                "Set emode admin from {:?} to {:?}",
                self.emode_admin,
                new_emode_admin
            );
            self.emode_admin = new_emode_admin;
        }
    }

    /// Set the group parameters when initializing a group.
    /// This should be called only when the group is first initialized.
    #[allow(clippy::too_many_arguments)]
    pub fn set_initial_configuration(&mut self, admin_pk: Pubkey) {
        self.admin = admin_pk;
        self.set_program_fee_enabled(true);
    }

    pub fn get_group_bank_config(&self) -> GroupBankConfig {
        GroupBankConfig {
            program_fees: self.group_flags == PROGRAM_FEES_ENABLED,
        }
    }

    pub fn set_program_fee_enabled(&mut self, fee_enabled: bool) {
        if fee_enabled {
            self.group_flags |= PROGRAM_FEES_ENABLED;
        } else {
            self.group_flags &= !PROGRAM_FEES_ENABLED;
        }
    }

    /// Set the `ARENA_GROUP` if `is_arena` is true. If trying to set as arena and the group already
    /// has more than two banks, fails. If trying to set an arena bank as non-arena, fails.
    pub fn set_arena_group(&mut self, is_arena: bool) -> MarginfiResult {
        // If enabling arena mode, ensure the group doesn't already have more than two banks.
        if is_arena && self.banks > 2 {
            return err!(MarginfiError::ArenaBankLimit);
        }

        // If the group is currently marked as arena, disallow switching it back to non-arena.
        if self.is_arena_group() && !is_arena {
            return err!(MarginfiError::ArenaSettingCannotChange);
        }

        if is_arena {
            self.group_flags |= ARENA_GROUP;
        } else {
            self.group_flags &= !ARENA_GROUP;
        }
        Ok(())
    }

    /// True if program fees are enabled
    pub fn program_fees_enabled(&self) -> bool {
        (self.group_flags & PROGRAM_FEES_ENABLED) != 0
    }

    /// True if this is an arena group
    pub fn is_arena_group(&self) -> bool {
        (self.group_flags & ARENA_GROUP) != 0
    }

    // Increment the bank count by 1. If this is an arena group, which only supports two banks,
    // errors if trying to add a third bank. If you managed to create 16,000 banks, congrats, does
    // nothing.
    pub fn add_bank(&mut self) -> MarginfiResult {
        if self.is_arena_group() && self.banks >= 2 {
            return err!(MarginfiError::ArenaBankLimit);
        }
        self.banks = self.banks.saturating_add(1);

        let clock = Clock::get()?;
        self.fee_state_cache.last_update = clock.unix_timestamp;

        Ok(())
    }
}

/// Group level configuration to be used in bank accounts.
#[derive(Clone, Debug)]
pub struct GroupBankConfig {
    pub program_fees: bool,
}
