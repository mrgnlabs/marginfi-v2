use crate::constants::{discriminators, ASSET_TAG_DEFAULT, EMPTY_BALANCE_THRESHOLD};
use bytemuck::{Pod, Zeroable};
use fixed::types::I80F48;

use super::{HealthCache, Pubkey, WrappedI80F48};

#[repr(C)]
#[derive(Debug, PartialEq, Pod, Zeroable, Copy, Clone)]
pub struct MarginfiAccount {
    pub group: Pubkey,                   // 32
    pub authority: Pubkey,               // 32
    pub lending_account: LendingAccount, // 1728
    /// The flags that indicate the state of the account. This is u64 bitfield, where each bit
    /// represents a flag.
    ///
    /// Flags:MarginfiAccount
    /// - 1: `ACCOUNT_DISABLED` - Indicates that the account is disabled and no further actions can
    /// be taken on it.
    /// - 2: `ACCOUNT_IN_FLASHLOAN` - Only set when an account is within a flash loan, e.g. when
    ///   start_flashloan is called, then unset when the flashloan ends.
    /// - 4: `ACCOUNT_FLAG_DEPRECATED` - Deprecated, available for future use
    /// - 8: `ACCOUNT_TRANSFER_AUTHORITY_ALLOWED` - the admin has flagged with account to be moved,
    ///   original owner can now call `set_account_transfer_authority`
    pub account_flags: u64, // 8
    /// Set with `update_emissions_destination_account`. Emissions rewards can be withdrawn to the
    /// cannonical ATA of this wallet without the user's input (withdraw_emissions_permissionless).
    /// If pubkey default, the user has not opted into this feature, and must claim emissions
    /// manually (withdraw_emissions).
    pub emissions_destination_account: Pubkey, // 32
    pub migrated_from: Pubkey,                 // 32
    pub health_cache: HealthCache,
    pub _padding0: [u64; 17],
}

impl MarginfiAccount {
    pub const LEN: usize = std::mem::size_of::<MarginfiAccount>();
    pub const DISCRIMINATOR: [u8; 8] = discriminators::ACCOUNT;
}

pub const ACCOUNT_DISABLED: u64 = 1 << 0;
pub const ACCOUNT_IN_FLASHLOAN: u64 = 1 << 1;
pub const ACCOUNT_FLAG_DEPRECATED: u64 = 1 << 2;
pub const ACCOUNT_TRANSFER_AUTHORITY_ALLOWED: u64 = 1 << 3;

pub const MAX_LENDING_ACCOUNT_BALANCES: usize = 16;

#[repr(C)]
#[derive(Debug, PartialEq, Pod, Zeroable, Copy, Clone)]
pub struct LendingAccount {
    pub balances: [Balance; MAX_LENDING_ACCOUNT_BALANCES], // 104 * 16 = 1664
    pub _padding: [u64; 8],                                // 8 * 8 = 64
}

impl LendingAccount {
    pub fn get_balance(&self, bank_pk: &Pubkey) -> Option<&Balance> {
        self.balances
            .iter()
            .find(|balance| balance.is_active() && balance.bank_pk.eq(bank_pk))
    }

    pub fn get_active_balances_iter(&self) -> impl Iterator<Item = &Balance> {
        self.balances.iter().filter(|b| b.is_active())
    }
}

pub enum BalanceSide {
    Assets,
    Liabilities,
}

#[repr(C)]
#[derive(Debug, PartialEq, Pod, Zeroable, Copy, Clone)]
pub struct Balance {
    pub active: u8,
    pub bank_pk: Pubkey,
    /// Inherited from the bank when the position is first created and CANNOT BE CHANGED after that.
    /// Note that all balances created before the addition of this feature use `ASSET_TAG_DEFAULT`
    pub bank_asset_tag: u8,
    pub _pad0: [u8; 6],
    pub asset_shares: WrappedI80F48,
    pub liability_shares: WrappedI80F48,
    pub emissions_outstanding: WrappedI80F48,
    pub last_update: u64,
    pub _padding: [u64; 1],
}

impl Balance {
    pub fn is_active(&self) -> bool {
        self.active != 0
    }

    pub fn set_active(&mut self, value: bool) {
        self.active = value as u8;
    }

    /// Check whether a balance is empty while accounting for any rounding errors
    /// that might have occured during depositing/withdrawing.
    #[inline]
    pub fn is_empty(&self, side: BalanceSide) -> bool {
        let shares: I80F48 = match side {
            BalanceSide::Assets => self.asset_shares,
            BalanceSide::Liabilities => self.liability_shares,
        }
        .into();

        shares < EMPTY_BALANCE_THRESHOLD
    }

    pub fn get_side(&self) -> Option<BalanceSide> {
        let asset_shares = I80F48::from(self.asset_shares);
        let liability_shares = I80F48::from(self.liability_shares);

        assert!(
            asset_shares < EMPTY_BALANCE_THRESHOLD || liability_shares < EMPTY_BALANCE_THRESHOLD
        );

        if I80F48::from(self.liability_shares) >= EMPTY_BALANCE_THRESHOLD {
            Some(BalanceSide::Liabilities)
        } else if I80F48::from(self.asset_shares) >= EMPTY_BALANCE_THRESHOLD {
            Some(BalanceSide::Assets)
        } else {
            None
        }
    }

    pub fn empty_deactivated() -> Self {
        Balance {
            active: 0,
            bank_pk: Pubkey::default(),
            bank_asset_tag: ASSET_TAG_DEFAULT,
            _pad0: [0; 6],
            asset_shares: WrappedI80F48::from(I80F48::ZERO),
            liability_shares: WrappedI80F48::from(I80F48::ZERO),
            emissions_outstanding: WrappedI80F48::from(I80F48::ZERO),
            last_update: 0,
            _padding: [0; 1],
        }
    }
}
