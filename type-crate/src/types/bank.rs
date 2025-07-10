use crate::{
    assert_struct_align, assert_struct_size,
    constants::discriminators,
    types::{BankCache, BankConfig},
};

#[cfg(feature = "anchor")]
use {anchor_lang::prelude::*, type_layout::TypeLayout};

use bytemuck::{Pod, Zeroable};

#[cfg(not(feature = "anchor"))]
use super::Pubkey;
use super::{EmodeSettings, WrappedI80F48};

assert_struct_size!(Bank, 1856);
assert_struct_align!(Bank, 8);
#[repr(C)]
#[cfg_attr(
    feature = "anchor",
    account(zero_copy),
    derive(Default, PartialEq, Eq, TypeLayout)
)]
#[cfg_attr(not(feature = "anchor"), derive(Zeroable))]
#[derive(Debug)]
pub struct Bank {
    pub mint: Pubkey,
    pub mint_decimals: u8,

    pub group: Pubkey,

    // Note: The padding is here, not after mint_decimals. Pubkey has alignment 1, so those 32
    // bytes can cross the alignment 8 threshold, but WrappedI80F48 has alignment 8 and cannot
    pub _pad0: [u8; 7], // 1x u8 + 7 = 8

    pub asset_share_value: WrappedI80F48,
    pub liability_share_value: WrappedI80F48,

    pub liquidity_vault: Pubkey,
    pub liquidity_vault_bump: u8,
    pub liquidity_vault_authority_bump: u8,

    pub insurance_vault: Pubkey,
    pub insurance_vault_bump: u8,
    pub insurance_vault_authority_bump: u8,

    pub _pad1: [u8; 4], // 4x u8 + 4 = 8

    /// Fees collected and pending withdraw for the `insurance_vault`
    pub collected_insurance_fees_outstanding: WrappedI80F48,

    pub fee_vault: Pubkey,
    pub fee_vault_bump: u8,
    pub fee_vault_authority_bump: u8,

    pub _pad2: [u8; 6], // 2x u8 + 6 = 8

    /// Fees collected and pending withdraw for the `fee_vault`
    pub collected_group_fees_outstanding: WrappedI80F48,

    pub total_liability_shares: WrappedI80F48,
    pub total_asset_shares: WrappedI80F48,

    pub last_update: i64,

    pub config: BankConfig,

    /// Bank Config Flags
    ///
    /// - EMISSIONS_FLAG_BORROW_ACTIVE: 1
    /// - EMISSIONS_FLAG_LENDING_ACTIVE: 2
    /// - PERMISSIONLESS_BAD_DEBT_SETTLEMENT: 4
    /// - FREEZE_SETTINGS: 8
    ///
    pub flags: u64,
    /// Emissions APR. Number of emitted tokens (emissions_mint) per 1e(bank.mint_decimal) tokens
    /// (bank mint) (native amount) per 1 YEAR.
    pub emissions_rate: u64,
    pub emissions_remaining: WrappedI80F48,
    pub emissions_mint: Pubkey,

    /// Fees collected and pending withdraw for the `FeeState.global_fee_wallet`'s canonical ATA for `mint`
    pub collected_program_fees_outstanding: WrappedI80F48,

    /// Controls this bank's emode configuration, which enables some banks to treat the assets of
    /// certain other banks more preferentially as collateral.
    pub emode: EmodeSettings,

    /// Set with `update_fees_destination_account`. Fees can be withdrawn to the canonical ATA of
    /// this wallet without the admin's input (withdraw_fees_permissionless). If pubkey default, the
    /// bank doesn't support this feature, and the fees must be collected manually (withdraw_fees).
    pub fees_destination_account: Pubkey,

    pub cache: BankCache,
    /// Number of user lending positions currently open in this bank
    /// * For banks created prior to 0.1.4, this is the number of positions opened/closed after
    ///   0.1.4 goes live, and may be negative.
    /// * For banks created in 0.1.4 or later, this is the number of positions open in total, and
    ///   the bank may safely be closed if this is zero. Will never go negative.
    pub lending_position_count: i32,
    /// Number of user borrowing positions currently open in this bank
    /// * For banks created prior to 0.1.4, this is the number of positions opened/closed after
    ///   0.1.4 goes live, and may be negative.
    /// * For banks created in 0.1.4 or later, this is the number of positions open in total, and
    ///   the bank may safely be closed if this is zero. Will never go negative.
    pub borrowing_position_count: i32,
    pub _padding_0: [u8; 16],
    pub _padding_1: [[u64; 2]; 19], // 8 * 2 * 19 = 304B
}

impl Bank {
    pub const LEN: usize = std::mem::size_of::<Bank>();
    pub const DISCRIMINATOR: [u8; 8] = discriminators::BANK;
}

#[repr(u8)]
#[cfg_attr(feature = "anchor", derive(AnchorDeserialize, AnchorSerialize))]
#[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
pub enum RiskTier {
    #[default]
    Collateral = 0,
    /// ## Isolated Risk
    /// Assets in this trance can be borrowed only in isolation.
    /// They can't be borrowed together with other assets.
    ///
    /// For example, if users has USDC, and wants to borrow XYZ which is isolated,
    /// they can't borrow XYZ together with SOL, only XYZ alone.
    Isolated = 1,
}
unsafe impl Zeroable for RiskTier {}
unsafe impl Pod for RiskTier {}

#[repr(u8)]
#[cfg_attr(feature = "anchor", derive(AnchorDeserialize, AnchorSerialize))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum BankOperationalState {
    Paused,
    Operational,
    ReduceOnly,
}
unsafe impl Zeroable for BankOperationalState {}
unsafe impl Pod for BankOperationalState {}

#[repr(u8)]
#[cfg_attr(feature = "anchor", derive(AnchorSerialize, AnchorDeserialize))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum OracleSetup {
    None,
    PythLegacy,
    SwitchboardV2,
    PythPushOracle,
    SwitchboardPull,
    StakedWithPythPush,
}
unsafe impl Zeroable for OracleSetup {}
unsafe impl Pod for OracleSetup {}

impl OracleSetup {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::PythLegacy),    // Deprecated
            2 => Some(Self::SwitchboardV2), // Deprecated
            3 => Some(Self::PythPushOracle),
            4 => Some(Self::SwitchboardPull),
            5 => Some(Self::StakedWithPythPush),
            _ => None,
        }
    }
}
