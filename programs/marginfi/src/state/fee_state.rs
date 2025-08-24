use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::{assert_struct_align, assert_struct_size};

use super::marginfi_group::WrappedI80F48;

/// Panic state for emergency protocol pausing
#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Zeroable, Pod, Debug, PartialEq, Eq,
)]
#[repr(C)]
pub struct PanicState {
    /// Whether the protocol is currently paused (1 = paused, 0 = not paused)
    pub is_paused: u8,
    /// Number of times paused today (resets every 24 hours)
    pub daily_pause_count: u8,
    /// Number of consecutive pauses (resets when unpause happens)
    pub consecutive_pause_count: u8,
    _reserved: [u8; 5],

    /// Timestamp when the current pause started (0 if not paused)
    pub pause_start_timestamp: i64,
    /// Timestamp of the last daily reset (for tracking daily pause count)
    pub last_daily_reset_timestamp: i64,
    /// Reserved for future use (making total struct 32 bytes)
    _reserved_space: [u8; 8],
}

impl PanicState {
    pub const PAUSE_DURATION_SECONDS: i64 = 15 * 60; // 15 minutes
    pub const MAX_CONSECUTIVE_PAUSES: u8 = 2;
    pub const MAX_DAILY_PAUSES: u8 = 3;
    pub const DAILY_RESET_INTERVAL: i64 = 24 * 60 * 60; // 24 hours

    pub fn is_paused(&self) -> bool {
        self.is_paused == 1
    }

    pub fn can_pause(&self, current_timestamp: i64) -> bool {
        // Reset daily count if 24 hours have passed
        let needs_daily_reset = current_timestamp - self.last_daily_reset_timestamp >= Self::DAILY_RESET_INTERVAL;
        
        let daily_count = if needs_daily_reset { 0 } else { self.daily_pause_count };
        
        // Check limits
        self.consecutive_pause_count < Self::MAX_CONSECUTIVE_PAUSES && 
        daily_count < Self::MAX_DAILY_PAUSES
    }

    pub fn is_expired(&self, current_timestamp: i64) -> bool {
        self.is_paused() && 
        (current_timestamp - self.pause_start_timestamp) >= Self::PAUSE_DURATION_SECONDS
    }

    pub fn pause(&mut self, current_timestamp: i64) -> Result<()> {
        require!(!self.is_paused(), crate::errors::MarginfiError::ProtocolAlreadyPaused);
        require!(self.can_pause(current_timestamp), crate::errors::MarginfiError::PauseLimitExceeded);

        // Reset daily count if needed
        if current_timestamp - self.last_daily_reset_timestamp >= Self::DAILY_RESET_INTERVAL {
            self.daily_pause_count = 0;
            self.last_daily_reset_timestamp = current_timestamp;
        }

        self.is_paused = 1;
        self.pause_start_timestamp = current_timestamp;
        self.daily_pause_count = self.daily_pause_count.saturating_add(1);
        self.consecutive_pause_count = self.consecutive_pause_count.saturating_add(1);

        Ok(())
    }

    pub fn unpause(&mut self) {
        self.is_paused = 0;
        self.pause_start_timestamp = 0;
        self.consecutive_pause_count = 0;
    }

    pub fn update_if_expired(&mut self, current_timestamp: i64) {
        if self.is_expired(current_timestamp) {
            self.unpause();
        }
    }
}

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
    /// to the canonical ATA of this wallet for that asset.
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
    /// Panic state for emergency protocol pausing
    pub panic_state: PanicState,
    // Reserved for future use
    _reserved0: [u8; 32],
    _reserved1: [u8; 32],
}

impl FeeState {
    pub const LEN: usize = std::mem::size_of::<FeeState>();
}

#[cfg(test)]
mod panic_state_tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let panic_state = PanicState::default();
        assert!(!panic_state.is_paused());
        assert_eq!(panic_state.daily_pause_count, 0);
        assert_eq!(panic_state.consecutive_pause_count, 0);
        assert_eq!(panic_state.pause_start_timestamp, 0);
        assert_eq!(panic_state.last_daily_reset_timestamp, 0);
    }

    #[test]
    fn test_can_pause_initially() {
        let panic_state = PanicState::default();
        assert!(panic_state.can_pause(1000));
    }

    #[test]
    fn test_pause_success() {
        let mut panic_state = PanicState::default();
        let timestamp = 100; // Use smaller timestamp to avoid daily reset
        
        panic_state.pause(timestamp).unwrap();
        
        assert!(panic_state.is_paused());
        assert_eq!(panic_state.pause_start_timestamp, timestamp);
        assert_eq!(panic_state.daily_pause_count, 1);
        assert_eq!(panic_state.consecutive_pause_count, 1);
        // Since timestamp (100) < DAILY_RESET_INTERVAL (86400), no daily reset occurs
        assert_eq!(panic_state.last_daily_reset_timestamp, 0);
    }

    #[test]
    fn test_pause_already_paused_fails() {
        let mut panic_state = PanicState::default();
        panic_state.pause(1000).unwrap();
        
        let result = panic_state.pause(2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_consecutive_pause_limit() {
        let mut panic_state = PanicState::default();
        let base_timestamp = 1000;
        
        // First pause - should work
        panic_state.pause(base_timestamp).unwrap();
        assert_eq!(panic_state.consecutive_pause_count, 1);
        
        // Let it expire and update
        panic_state.update_if_expired(base_timestamp + PanicState::PAUSE_DURATION_SECONDS);
        assert!(!panic_state.is_paused());
        assert_eq!(panic_state.consecutive_pause_count, 0); // Reset on auto-unpause
        
        // Second pause - should work
        panic_state.pause(base_timestamp + PanicState::PAUSE_DURATION_SECONDS + 1).unwrap();
        assert_eq!(panic_state.consecutive_pause_count, 1);
        
        // Let it expire and update
        panic_state.update_if_expired(base_timestamp + 2 * PanicState::PAUSE_DURATION_SECONDS + 1);
        assert!(!panic_state.is_paused());
        assert_eq!(panic_state.consecutive_pause_count, 0);
    }

    #[test]
    fn test_daily_pause_limit() {
        let mut panic_state = PanicState::default();
        let base_timestamp = 1000;
        
        // Exhaust daily limit
        for i in 0..PanicState::MAX_DAILY_PAUSES {
            panic_state.pause(base_timestamp + i as i64).unwrap();
            panic_state.unpause();
        }
        
        // Next pause should fail
        let result = panic_state.pause(base_timestamp + PanicState::MAX_DAILY_PAUSES as i64);
        assert!(result.is_err());
        assert!(!panic_state.can_pause(base_timestamp + PanicState::MAX_DAILY_PAUSES as i64));
    }

    #[test]
    fn test_daily_reset() {
        let mut panic_state = PanicState::default();
        let base_timestamp = 1000;
        
        // Exhaust daily limit
        for i in 0..PanicState::MAX_DAILY_PAUSES {
            panic_state.pause(base_timestamp + i as i64).unwrap();
            panic_state.unpause();
        }
        
        // Move forward 24+ hours
        let next_day = base_timestamp + PanicState::DAILY_RESET_INTERVAL + 1;
        
        // Should be able to pause again
        assert!(panic_state.can_pause(next_day));
        panic_state.pause(next_day).unwrap();
        
        // Daily count should reset
        assert_eq!(panic_state.daily_pause_count, 1);
        assert_eq!(panic_state.last_daily_reset_timestamp, next_day);
    }

    #[test]
    fn test_is_expired() {
        let mut panic_state = PanicState::default();
        let start_time = 1000;
        
        panic_state.pause(start_time).unwrap();
        
        // Not expired within duration
        assert!(!panic_state.is_expired(start_time + PanicState::PAUSE_DURATION_SECONDS - 1));
        
        // Expired after duration
        assert!(panic_state.is_expired(start_time + PanicState::PAUSE_DURATION_SECONDS));
        assert!(panic_state.is_expired(start_time + PanicState::PAUSE_DURATION_SECONDS + 100));
    }

    #[test]
    fn test_not_expired_when_not_paused() {
        let panic_state = PanicState::default();
        assert!(!panic_state.is_expired(1000));
    }

    #[test]
    fn test_update_if_expired() {
        let mut panic_state = PanicState::default();
        let start_time = 1000;
        
        panic_state.pause(start_time).unwrap();
        assert!(panic_state.is_paused());
        
        // Update before expiration - should remain paused
        panic_state.update_if_expired(start_time + PanicState::PAUSE_DURATION_SECONDS - 1);
        assert!(panic_state.is_paused());
        
        // Update after expiration - should auto-unpause
        panic_state.update_if_expired(start_time + PanicState::PAUSE_DURATION_SECONDS);
        assert!(!panic_state.is_paused());
        assert_eq!(panic_state.consecutive_pause_count, 0);
        assert_eq!(panic_state.pause_start_timestamp, 0);
    }

    #[test]
    fn test_unpause() {
        let mut panic_state = PanicState::default();
        panic_state.pause(1000).unwrap();
        
        assert!(panic_state.is_paused());
        assert_eq!(panic_state.consecutive_pause_count, 1);
        
        panic_state.unpause();
        
        assert!(!panic_state.is_paused());
        assert_eq!(panic_state.consecutive_pause_count, 0);
        assert_eq!(panic_state.pause_start_timestamp, 0);
        // Daily count should remain unchanged
        assert_eq!(panic_state.daily_pause_count, 1);
    }

    #[test]
    fn test_pause_constants() {
        assert_eq!(PanicState::PAUSE_DURATION_SECONDS, 15 * 60);
        assert_eq!(PanicState::MAX_CONSECUTIVE_PAUSES, 2);
        assert_eq!(PanicState::MAX_DAILY_PAUSES, 3);
        assert_eq!(PanicState::DAILY_RESET_INTERVAL, 24 * 60 * 60);
    }

    #[test]
    fn test_multiple_pause_unpause_cycles() {
        let mut panic_state = PanicState::default();
        let base_timestamp = 1000;
        
        // First cycle
        panic_state.pause(base_timestamp).unwrap();
        assert_eq!(panic_state.consecutive_pause_count, 1);
        assert_eq!(panic_state.daily_pause_count, 1);
        panic_state.unpause();
        
        // Second cycle
        panic_state.pause(base_timestamp + 100).unwrap();
        assert_eq!(panic_state.consecutive_pause_count, 1);
        assert_eq!(panic_state.daily_pause_count, 2);
        panic_state.unpause();
        
        // Third cycle
        panic_state.pause(base_timestamp + 200).unwrap();
        assert_eq!(panic_state.consecutive_pause_count, 1);
        assert_eq!(panic_state.daily_pause_count, 3);
        panic_state.unpause();
        
        // Fourth cycle should fail daily limit
        let result = panic_state.pause(base_timestamp + 300);
        assert!(result.is_err());
    }

    #[test]
    fn test_consecutive_limit_resets_after_unpause() {
        let mut panic_state = PanicState::default();
        let base_timestamp = 1000;
        
        // First pause
        panic_state.pause(base_timestamp).unwrap();
        panic_state.unpause();
        
        // Second pause
        panic_state.pause(base_timestamp + 100).unwrap();
        assert_eq!(panic_state.consecutive_pause_count, 1); // Reset after unpause
        panic_state.unpause();
        
        // Should be able to pause again since consecutive count reset
        panic_state.pause(base_timestamp + 200).unwrap();
        assert_eq!(panic_state.consecutive_pause_count, 1);
    }

    #[test]
    fn test_consecutive_limit_without_unpause() {
        let mut panic_state = PanicState::default();
        let base_timestamp = 1000;
        
        // First pause that expires automatically
        panic_state.pause(base_timestamp).unwrap();
        panic_state.update_if_expired(base_timestamp + PanicState::PAUSE_DURATION_SECONDS);
        assert_eq!(panic_state.consecutive_pause_count, 0); // Reset after expiration
        
        // Second pause that expires automatically  
        panic_state.pause(base_timestamp + PanicState::PAUSE_DURATION_SECONDS + 1).unwrap();
        panic_state.update_if_expired(base_timestamp + 2 * PanicState::PAUSE_DURATION_SECONDS + 1);
        assert_eq!(panic_state.consecutive_pause_count, 0); // Reset after expiration
        
        // Should still be able to pause since consecutive count resets on expiration
        panic_state.pause(base_timestamp + 2 * PanicState::PAUSE_DURATION_SECONDS + 2).unwrap();
        assert_eq!(panic_state.consecutive_pause_count, 1);
    }
}
