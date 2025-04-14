use super::{marginfi_account::MAX_LENDING_ACCOUNT_BALANCES, marginfi_group::WrappedI80F48};
use crate::{assert_struct_align, assert_struct_size};
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use type_layout::TypeLayout;

pub const HEALTHY: u32 = 1;
pub const ENGINE_OK: u32 = 2;
pub const ORACLE_OK: u32 = 4;

assert_struct_size!(HealthCache, 312);
assert_struct_align!(HealthCache, 8);
#[repr(C)]
#[derive(
    AnchorDeserialize, AnchorSerialize, Copy, Clone, Zeroable, Pod, PartialEq, Eq, TypeLayout, Debug,
)]
/// A read-only cache of the internal risk engine's information. Only valid in borrow/withdraw if
/// the tx does not fail. To see the state in any context, e.g. to figure out if the risk engine is
/// failing due to some bad price information, use `pulse_health`.
pub struct HealthCache {
    pub asset_value: WrappedI80F48,
    pub liability_value: WrappedI80F48,
    /// Unix timestamp from the system clock when this cache was last updated
    pub timestamp: i64,
    /// The flags that indicate the state of the health cache. This is a u64 bitfield, where each
    /// bit represents a flag.
    ///
    /// * HEALTHY = 1 - If set, the account cannot be liquidated. If 0, the account is unhealthy and
    ///   can be liquidated.
    /// * ENGINE STATUS = 2 - If set, the engine did not error during the last health pulse. If 0,
    ///   the engine would have errored and this cache is likely invalid. `RiskEngineInitRejected`
    ///   is ignored and will allow the flag to be set anyways.
    /// * ORACLE OK = 4 - If set, the engine did not error due to an oracle issue. If 0, engine was
    ///   passed a bad bank or oracle account, or an oracle was stale. Check the order in which
    ///   accounts were passed and ensure each balance has the correct banks/oracles, and that
    ///   oracle cranks ran recently enough. Check `internal_err` and `err_index` for more details
    ///   in some circumstances.
    /// * 8, 16, 32, 64, 128, etc - reserved for future use
    pub flags: u32,
    /// If the engine errored, look here for the error code.
    pub mrgn_err: u32,
    /// Each price corresponds to that index of Balances in the LendingAccount. Useful for debugging
    /// or liquidator consumption, to determine how a user's position is priced internally.
    /// * If a price overflows u64, shows u64::MAX
    /// * If a price is negative for some reason (as several oracles support), pulse will panic
    pub prices: [WrappedI80F48; MAX_LENDING_ACCOUNT_BALANCES],
    /// Errors in asset oracles are ignored (with prices treated as zero). If you see a zero price
    /// and the `ORACLE_OK` flag is not set, check here to see what error was ignored internally.
    pub internal_err: u32,
    /// Index in `balances` where `internal_err` appeared
    pub err_index: u8,
    pub pad0: [u8; 3],
}

impl HealthCache {
    /// True if account is healthy (cannot be liquidated)
    pub fn is_healthy(&self) -> bool {
        self.flags & HEALTHY != 0
    }

    pub fn set_healthy(&mut self, healthy: bool) {
        if healthy {
            self.flags |= HEALTHY;
        } else {
            self.flags &= !HEALTHY;
        }
    }

    /// True if the engine did not error during the last health pulse.
    pub fn is_engine_ok(&self) -> bool {
        self.flags & ENGINE_OK != 0
    }

    pub fn set_engine_ok(&mut self, ok: bool) {
        if ok {
            self.flags |= ENGINE_OK;
        } else {
            self.flags &= !ENGINE_OK;
        }
    }

    /// True if the engine did not detect an oracle input issue in the last health pulse
    pub fn is_oracle_ok(&self) -> bool {
        self.flags & ORACLE_OK != 0
    }

    pub fn set_oracle_ok(&mut self, ok: bool) {
        if ok {
            self.flags |= ORACLE_OK;
        } else {
            self.flags &= !ORACLE_OK;
        }
    }
}
