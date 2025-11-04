use crate::{prelude::MarginfiError, MarginfiResult};
use anchor_lang::prelude::*;
use marginfi_type_crate::types::MarginfiGroup;
use std::fmt::Debug;

pub const PROGRAM_FEES_ENABLED: u64 = 1;
pub const ARENA_GROUP: u64 = 2;

pub trait MarginfiGroupImpl {
    fn update_admin(&mut self, new_admin: Pubkey);
    fn update_emode_admin(&mut self, new_emode_admin: Pubkey);
    fn update_curve_admin(&mut self, new_curve_admin: Pubkey);
    fn update_limit_admin(&mut self, new_limit_admin: Pubkey);
    fn update_emissions_admin(&mut self, new_emissions_admin: Pubkey);
    fn update_risk_admin(&mut self, new_risk_admin: Pubkey);
    fn set_initial_configuration(&mut self, admin_pk: Pubkey);
    fn get_group_bank_config(&self) -> GroupBankConfig;
    fn set_program_fee_enabled(&mut self, fee_enabled: bool);
    fn set_arena_group(&mut self, is_arena: bool) -> MarginfiResult;
    fn program_fees_enabled(&self) -> bool;
    fn is_arena_group(&self) -> bool;
    fn add_bank(&mut self) -> MarginfiResult;
    fn is_protocol_paused(&self) -> bool;
}

impl MarginfiGroupImpl for MarginfiGroup {
    fn update_admin(&mut self, new_admin: Pubkey) {
        if self.admin == new_admin {
            msg!("No change to admin: {:?}", new_admin);
            // do nothing
        } else {
            msg!("Set admin from {:?} to {:?}", self.admin, new_admin);
            self.admin = new_admin;
        }
    }

    fn update_emode_admin(&mut self, new_emode_admin: Pubkey) {
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

    fn update_curve_admin(&mut self, new_curve_admin: Pubkey) {
        if self.delegate_curve_admin == new_curve_admin {
            msg!("No change to curve admin: {:?}", new_curve_admin);
            // do nothing
        } else {
            msg!(
                "Set curve admin from {:?} to {:?}",
                self.delegate_curve_admin,
                new_curve_admin
            );
            self.delegate_curve_admin = new_curve_admin;
        }
    }

    fn update_limit_admin(&mut self, new_limit_admin: Pubkey) {
        if self.delegate_limit_admin == new_limit_admin {
            msg!("No change to limit admin: {:?}", new_limit_admin);
            // do nothing
        } else {
            msg!(
                "Set limit admin from {:?} to {:?}",
                self.delegate_limit_admin,
                new_limit_admin
            );
            self.delegate_limit_admin = new_limit_admin;
        }
    }

    fn update_emissions_admin(&mut self, new_emissions_admin: Pubkey) {
        if self.delegate_emissions_admin == new_emissions_admin {
            msg!("No change to emissions admin: {:?}", new_emissions_admin);
            // do nothing
        } else {
            msg!(
                "Set emissions admin from {:?} to {:?}",
                self.delegate_emissions_admin,
                new_emissions_admin
            );
            self.delegate_emissions_admin = new_emissions_admin;
        }
    }

    fn update_risk_admin(&mut self, new_risk_admin: Pubkey) {
        if self.risk_admin == new_risk_admin {
            msg!("No change to risk admin: {:?}", new_risk_admin);
            // do nothing
        } else {
            msg!(
                "Set risk admin from {:?} to {:?}",
                self.risk_admin,
                new_risk_admin
            );
            self.risk_admin = new_risk_admin;
        }
    }

    /// Set the group parameters when initializing a group.
    /// This should be called only when the group is first initialized.
    #[allow(clippy::too_many_arguments)]
    fn set_initial_configuration(&mut self, admin_pk: Pubkey) {
        self.admin = admin_pk;
        self.set_program_fee_enabled(true);
    }

    fn get_group_bank_config(&self) -> GroupBankConfig {
        GroupBankConfig {
            program_fees: self.group_flags == PROGRAM_FEES_ENABLED,
        }
    }

    fn set_program_fee_enabled(&mut self, fee_enabled: bool) {
        if fee_enabled {
            self.group_flags |= PROGRAM_FEES_ENABLED;
        } else {
            self.group_flags &= !PROGRAM_FEES_ENABLED;
        }
    }

    /// Set the `ARENA_GROUP` if `is_arena` is true. If trying to set as arena and the group already
    /// has more than two banks, fails. If trying to set an arena bank as non-arena, fails.
    fn set_arena_group(&mut self, is_arena: bool) -> MarginfiResult {
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
    fn program_fees_enabled(&self) -> bool {
        (self.group_flags & PROGRAM_FEES_ENABLED) != 0
    }

    /// True if this is an arena group
    fn is_arena_group(&self) -> bool {
        (self.group_flags & ARENA_GROUP) != 0
    }

    // Increment the bank count by 1. If this is an arena group, which only supports two banks,
    // errors if trying to add a third bank. If you managed to create 16,000 banks, congrats, does
    // nothing.
    fn add_bank(&mut self) -> MarginfiResult {
        if self.is_arena_group() && self.banks >= 2 {
            return err!(MarginfiError::ArenaBankLimit);
        }
        self.banks = self.banks.saturating_add(1);

        let clock = Clock::get()?;
        self.fee_state_cache.last_update = clock.unix_timestamp;

        Ok(())
    }

    /// Returns true if the protocol is in a paused state and the time has not yet expired, false it
    /// not paused or timer has expired.
    fn is_protocol_paused(&self) -> bool {
        // Note: In rare event clock fails to unwrap, time = 0 always fails the is_expired check.
        let current_timestamp = Clock::get().map(|c| c.unix_timestamp).unwrap_or(0);

        self.panic_state_cache.is_paused_flag()
            && !self.panic_state_cache.is_expired(current_timestamp)
    }
}

/// Group level configuration to be used in bank accounts.
#[derive(Clone, Debug)]
pub struct GroupBankConfig {
    pub program_fees: bool,
}
