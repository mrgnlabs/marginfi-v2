#[cfg(not(feature = "anchor"))]
use {
    super::Pubkey,
    bytemuck::{Pod, Zeroable},
};

#[cfg(feature = "anchor")]
use {
    anchor_lang::prelude::{
        borsh::{BorshDeserialize, BorshSerialize},
        zero_copy, *,
    },
    type_layout::TypeLayout,
};

use super::{Balance, LendingAccount, WrappedI80F48, MAX_LENDING_ACCOUNT_BALANCES};
use crate::{assert_struct_align, assert_struct_size, constants::discriminators};

assert_struct_size!(LiquidationRecord, 1800);
assert_struct_align!(LiquidationRecord, 8);
#[repr(C)]
#[cfg_attr(feature = "anchor", account(zero_copy))]
#[cfg_attr(
    not(feature = "anchor"),
    derive(Debug, PartialEq, Pod, Zeroable, Copy, Clone)
)]
pub struct LiquidationRecord {
    /// This account's own key. A PDA derived from `marginfi_account`
    pub key: Pubkey,
    /// Account this record tracks
    pub marginfi_account: Pubkey,
    /// The last liquidator to take receivership of the `marginfi_account` and complete a liquidation.
    pub liquidation_receiver: Pubkey,
    /// Basic data for the last few liquidation events on this account
    pub entries: [LiquidationEntry; 4],
    pub cache: LiquidationCache,

    _reserved0: [u8; 256],
}

/// Used to record key details of the last few liquidation events on the account
#[repr(C)]
#[cfg_attr(
    feature = "anchor",
    zero_copy,
    derive(Default, BorshDeserialize, BorshSerialize, TypeLayout)
)]
#[cfg_attr(not(feature = "anchor"), derive(Default, Clone, Copy, Pod, Zeroable))]
pub struct LiquidationEntry {
    pub asset_amount_seized: u64,
    pub liab_amount_repaid: u64,
    pub liab_fees_taken: u64,
    pub timestamp: i64,
    _reserved0: [u8; 16],
}

// Stores data used by the liquidator during the liquidation process
#[repr(C)]
#[cfg_attr(
    feature = "anchor",
    zero_copy,
    derive(Default, BorshDeserialize, BorshSerialize, TypeLayout)
)]
#[cfg_attr(not(feature = "anchor"), derive(Default, Clone, Copy, Pod, Zeroable))]
pub struct LiquidationCache {
    /// Internal risk engine asset value snapshot taken when liquidation begins, using maintenance
    /// weight with all confidence adjustments.
    /// * Uses SPOT price
    /// * In dollars
    pub asset_value_maint: WrappedI80F48,
    /// Internal risk engine liability value snapshot taken when liquidation begins, using
    /// maintenance weight with all confidence adjustments.
    /// * Uses SPOT price
    /// * In dollars
    pub liability_value_maint: WrappedI80F48,
    /// Actual cash value of assets pre-liquidation
    /// * Liquidator is allowed to seize up to `liability_value_equity` - this amount
    /// * Uses EMA price
    /// * In dollars
    pub asset_value_equity: WrappedI80F48,
    /// Actual cash value of liabilities pre-liquidation
    /// * Liquidator is allowed to seize up to this amount - `asset_value_equity`
    /// * Uses EMA price
    /// * In dollars
    pub liability_value_equity: WrappedI80F48,
    /// A compact snapshot of the lending account taken when liquidation begins
    pub mini_lending_account: MiniLendingAccount,
    pub _placeholder: u64,
    _reserved0: [u8; 32],
}

// A compact LendingAccount snapshot for use during liquidation
#[repr(C)]
#[cfg_attr(
    feature = "anchor",
    zero_copy,
    derive(Default, BorshDeserialize, BorshSerialize, TypeLayout)
)]
#[cfg_attr(not(feature = "anchor"), derive(Defalt, Clone, Copy, Pod, Zeroable))]
pub struct MiniLendingAccount {
    pub balances: [MiniLendingAccountBalance; MAX_LENDING_ACCOUNT_BALANCES],
}

impl MiniLendingAccount {
    /// Convert a `LendingAccount` into its “mini” form, copying each `Balance` slot in index order.
    pub fn from_lending_account(src: &LendingAccount) -> Self {
        let mut mini = MiniLendingAccount::default();
        for (i, bal) in src.balances.iter().enumerate() {
            mini.balances[i] = MiniLendingAccountBalance::from(bal);
        }
        mini
    }
}

// A compact account Balance for use during liquidation
#[repr(C)]
#[cfg_attr(
    feature = "anchor",
    zero_copy,
    derive(Default, BorshDeserialize, BorshSerialize, TypeLayout)
)]
#[cfg_attr(not(feature = "anchor"), derive(Default, Clone, Copy, Pod, Zeroable))]
pub struct MiniLendingAccountBalance {
    pub bank_pk: Pubkey,
    pub asset_shares: WrappedI80F48,
    pub liability_shares: WrappedI80F48,
    pub active: u8,
    pub bank_asset_tag: u8,
    _padding: [u8; 6],
}

impl From<&Balance> for MiniLendingAccountBalance {
    fn from(b: &Balance) -> Self {
        MiniLendingAccountBalance {
            asset_shares: b.asset_shares,
            liability_shares: b.liability_shares,
            bank_pk: b.bank_pk,
            active: b.active,
            bank_asset_tag: b.bank_asset_tag,
            _padding: [0; 6],
        }
    }
}

impl LiquidationRecord {
    pub const LEN: usize = std::mem::size_of::<LiquidationRecord>();
    pub const DISCRIMINATOR: [u8; 8] = discriminators::LIQUIDATION_RECORD;

    /// Generally used at the start of liquidation to snapshot the account's key health information.
    pub fn new(
        key: Pubkey,
        marginfi_account: Pubkey,
        liquidation_receiver: Pubkey,
        asset_value_maint: WrappedI80F48,
        liability_value_maint: WrappedI80F48,
        asset_value_equity: WrappedI80F48,
        liability_value_equity: WrappedI80F48,
        lending_account: LendingAccount,
    ) -> Self {
        let mut cache = LiquidationCache::default();
        cache.asset_value_maint = asset_value_maint;
        cache.liability_value_maint = liability_value_maint;
        cache.asset_value_equity = asset_value_equity;
        cache.liability_value_equity = liability_value_equity;
        cache.mini_lending_account = MiniLendingAccount::from_lending_account(&lending_account);
        LiquidationRecord {
            key,
            marginfi_account,
            liquidation_receiver,
            entries: [LiquidationEntry::default(); 4],
            cache,
            _reserved0: [0u8; 256],
        }
    }
}

impl Default for LiquidationRecord {
    fn default() -> Self {
        LiquidationRecord {
            key: Pubkey::default(),
            marginfi_account: Pubkey::default(),
            liquidation_receiver: Pubkey::default(),
            entries: [LiquidationEntry::default(); 4],
            cache: LiquidationCache::default(),
            _reserved0: [0u8; 256],
        }
    }
}
