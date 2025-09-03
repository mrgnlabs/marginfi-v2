use crate::{check, MarginfiError, MarginfiResult};
use marginfi_type_crate::types::PanicState;

pub trait PanicStateImpl {
    fn pause(&self, current_timestamp: i64) -> MarginfiResult;
    fn unpause(&mut self);
    fn update_if_expired(&mut self, current_timestamp: i64);
}

impl PanicStateImpl for PanicState {
    fn pause(&mut self, current_timestamp: i64) -> Result<()> {
        require!(!self.is_paused(), MarginfiError::ProtocolAlreadyPaused);
        require!(
            self.can_pause(current_timestamp),
            MarginfiError::PauseLimitExceeded
        );

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

    fn unpause(&mut self) {
        self.is_paused = 0;
        self.pause_start_timestamp = 0;
        self.consecutive_pause_count = 0;
    }

    fn update_if_expired(&mut self, current_timestamp: i64) {
        if self.is_expired(current_timestamp) {
            self.unpause();
        }
    }
}
