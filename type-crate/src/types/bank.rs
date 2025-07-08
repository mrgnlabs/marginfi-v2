use crate::{
    assert_struct_align, assert_struct_size,
    constants::{
        discriminators, ASSET_TAG_DEFAULT, PYTH_PUSH_MIGRATED,
        TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE,
    },
    types::BankCache,
};

#[cfg(feature = "anchor")]
use {anchor_lang::prelude::*, type_layout::TypeLayout};

use bytemuck::{Pod, Zeroable};
use fixed::types::I80F48;

#[cfg(not(feature = "anchor"))]
use super::Pubkey;
use super::{EmodeSettings, WrappedI80F48};

pub const MAX_ORACLE_KEYS: usize = 5;

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

assert_struct_size!(BankConfig, 544);
assert_struct_align!(BankConfig, 8);
#[repr(C)]
#[cfg_attr(
    feature = "anchor",
    derive(AnchorDeserialize, AnchorSerialize, TypeLayout)
)]
#[derive(Debug, PartialEq, Pod, Zeroable, Copy, Clone, Eq)]
pub struct BankConfig {
    /// TODO: Convert weights to (u64, u64) to avoid precision loss (maybe?)
    pub asset_weight_init: WrappedI80F48,
    pub asset_weight_maint: WrappedI80F48,

    pub liability_weight_init: WrappedI80F48,
    pub liability_weight_maint: WrappedI80F48,

    pub deposit_limit: u64,

    pub interest_rate_config: InterestRateConfig,
    pub operational_state: BankOperationalState,

    pub oracle_setup: OracleSetup,
    pub oracle_keys: [Pubkey; MAX_ORACLE_KEYS],

    // Note: Pubkey is aligned 1, so borrow_limit is the first aligned-8 value after deposit_limit
    pub _pad0: [u8; 6], // Bank state (1) + Oracle Setup (1) + 6 = 8

    pub borrow_limit: u64,

    pub risk_tier: RiskTier,

    /// Determines what kinds of assets users of this bank can interact with.
    /// Options:
    /// * ASSET_TAG_DEFAULT (0) - A regular asset that can be comingled with any other regular asset
    ///   or with `ASSET_TAG_SOL`
    /// * ASSET_TAG_SOL (1) - Accounts with a SOL position can comingle with **either**
    /// `ASSET_TAG_DEFAULT` or `ASSET_TAG_STAKED` positions, but not both
    /// * ASSET_TAG_STAKED (2) - Staked SOL assets. Accounts with a STAKED position can only deposit
    /// other STAKED assets or SOL (`ASSET_TAG_SOL`) and can only borrow SOL
    pub asset_tag: u8,

    /// Flags for various config options
    /// * 1 - Always set if bank created in 0.1.4 or later, or if migrated to the new pyth
    ///   oracle setup from a prior version. Not set in 0.1.3 or earlier banks using pyth that have
    ///   not yet migrated. Does nothing for banks that use switchboard.
    /// * 2, 4, 8, 16, etc - reserved for future use.
    pub config_flags: u8,

    pub _pad1: [u8; 5],

    /// USD denominated limit for calculating asset value for initialization margin requirements.
    /// Example, if total SOL deposits are equal to $1M and the limit it set to $500K,
    /// then SOL assets will be discounted by 50%.
    ///
    /// In other words the max value of liabilities that can be backed by the asset is $500K.
    /// This is useful for limiting the damage of orcale attacks.
    ///
    /// Value is UI USD value, for example value 100 -> $100
    pub total_asset_value_init_limit: u64,

    /// Time window in seconds for the oracle price feed to be considered live.
    pub oracle_max_age: u16,

    // pad to next 4-byte alignment to meet u32's requirements.
    pub _padding0: [u8; 2],

    /// From 0-100%, if the confidence exceeds this value, the oracle is considered invalid. Note:
    /// the confidence adjustment is capped at 5% regardless of this value.
    /// * 0 falls back to using the default 10% instead, i.e., U32_MAX_DIV_10
    /// * A %, as u32, e.g. 100% = u32::MAX, 50% = u32::MAX/2, etc.
    pub oracle_max_confidence: u32,

    pub _padding1: [u8; 32],
}

impl Default for BankConfig {
    fn default() -> Self {
        Self {
            asset_weight_init: I80F48::ZERO.into(),
            asset_weight_maint: I80F48::ZERO.into(),
            liability_weight_init: I80F48::ONE.into(),
            liability_weight_maint: I80F48::ONE.into(),
            deposit_limit: 0,
            borrow_limit: 0,
            interest_rate_config: Default::default(),
            operational_state: BankOperationalState::Paused,
            oracle_setup: OracleSetup::None,
            oracle_keys: [Pubkey::default(); MAX_ORACLE_KEYS],
            _pad0: [0; 6],
            risk_tier: RiskTier::Isolated,
            asset_tag: ASSET_TAG_DEFAULT,
            config_flags: 0,
            _pad1: [0; 5],
            total_asset_value_init_limit: TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE,
            oracle_max_age: 0,
            _padding0: [0; 2],
            oracle_max_confidence: 0,
            _padding1: [0; 32],
        }
    }
}

#[cfg_attr(
    feature = "anchor",
    derive(AnchorDeserialize, AnchorSerialize, TypeLayout)
)]
#[derive(Default, Clone, PartialEq, Eq)]
pub struct BankConfigOpt {
    pub asset_weight_init: Option<WrappedI80F48>,
    pub asset_weight_maint: Option<WrappedI80F48>,

    pub liability_weight_init: Option<WrappedI80F48>,
    pub liability_weight_maint: Option<WrappedI80F48>,

    pub deposit_limit: Option<u64>,
    pub borrow_limit: Option<u64>,

    pub operational_state: Option<BankOperationalState>,

    pub interest_rate_config: Option<InterestRateConfigOpt>,

    pub risk_tier: Option<RiskTier>,

    pub asset_tag: Option<u8>,

    pub total_asset_value_init_limit: Option<u64>,

    pub oracle_max_confidence: Option<u32>,

    pub oracle_max_age: Option<u16>,

    pub permissionless_bad_debt_settlement: Option<bool>,

    pub freeze_settings: Option<bool>,
}

#[repr(C)]
#[cfg_attr(feature = "anchor", derive(AnchorDeserialize, AnchorSerialize))]
#[derive(Debug, PartialEq, Eq)]
/// TODO: Convert weights to (u64, u64) to avoid precision loss (maybe?)
pub struct BankConfigCompact {
    pub asset_weight_init: WrappedI80F48,
    pub asset_weight_maint: WrappedI80F48,

    pub liability_weight_init: WrappedI80F48,
    pub liability_weight_maint: WrappedI80F48,

    pub deposit_limit: u64,

    pub interest_rate_config: InterestRateConfigCompact,
    pub operational_state: BankOperationalState,

    pub borrow_limit: u64,

    pub risk_tier: RiskTier,

    /// Determines what kinds of assets users of this bank can interact with.
    /// Options:
    /// * ASSET_TAG_DEFAULT (0) - A regular asset that can be comingled with any other regular asset
    ///   or with `ASSET_TAG_SOL`
    /// * ASSET_TAG_SOL (1) - Accounts with a SOL position can comingle with **either**
    /// `ASSET_TAG_DEFAULT` or `ASSET_TAG_STAKED` positions, but not both
    /// * ASSET_TAG_STAKED (2) - Staked SOL assets. Accounts with a STAKED position can only deposit
    /// other STAKED assets or SOL (`ASSET_TAG_SOL`) and can only borrow SOL
    pub asset_tag: u8,

    /// Flags for various config options
    /// * 1 - Always set if bank created in 0.1.4 or later, or if migrated to the new oracle
    ///   setup from a prior version. Not set in 0.1.3 or earlier banks that have not yet migrated.
    /// * 2, 4, 8, 16, etc - reserved for future use.
    pub config_flags: u8,
    pub _pad0: [u8; 5],

    /// USD denominated limit for calculating asset value for initialization margin requirements.
    /// Example, if total SOL deposits are equal to $1M and the limit it set to $500K,
    /// then SOL assets will be discounted by 50%.
    ///
    /// In other words the max value of liabilities that can be backed by the asset is $500K.
    /// This is useful for limiting the damage of orcale attacks.
    ///
    /// Value is UI USD value, for example value 100 -> $100
    pub total_asset_value_init_limit: u64,

    /// Time window in seconds for the oracle price feed to be considered live.
    pub oracle_max_age: u16,

    /// From 0-100%, if the confidence exceeds this value, the oracle is considered invalid. Note:
    /// the confidence adjustment is capped at 5% regardless of this value.
    /// * 0% = use the default (10%)
    /// * A %, as u32, e.g. 100% = u32::MAX, 50% = u32::MAX/2, etc.
    pub oracle_max_confidence: u32,
}

impl Default for BankConfigCompact {
    fn default() -> Self {
        Self {
            asset_weight_init: I80F48::ZERO.into(),
            asset_weight_maint: I80F48::ZERO.into(),
            liability_weight_init: I80F48::ONE.into(),
            liability_weight_maint: I80F48::ONE.into(),
            deposit_limit: 0,
            borrow_limit: 0,
            interest_rate_config: InterestRateConfigCompact::default(),
            operational_state: BankOperationalState::Paused,
            config_flags: PYTH_PUSH_MIGRATED,
            _pad0: [0; 5],
            risk_tier: RiskTier::Isolated,
            asset_tag: ASSET_TAG_DEFAULT,
            total_asset_value_init_limit: TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE,
            oracle_max_age: 0,
            oracle_max_confidence: 0,
        }
    }
}

impl From<BankConfigCompact> for BankConfig {
    fn from(config: BankConfigCompact) -> Self {
        let keys = [
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
        ];
        Self {
            asset_weight_init: config.asset_weight_init,
            asset_weight_maint: config.asset_weight_maint,
            liability_weight_init: config.liability_weight_init,
            liability_weight_maint: config.liability_weight_maint,
            deposit_limit: config.deposit_limit,
            interest_rate_config: config.interest_rate_config.into(),
            operational_state: config.operational_state,
            oracle_setup: OracleSetup::None,
            oracle_keys: keys,
            _pad0: [0; 6],
            borrow_limit: config.borrow_limit,
            risk_tier: config.risk_tier,
            asset_tag: config.asset_tag,
            config_flags: config.config_flags,
            _pad1: [0; 5],
            total_asset_value_init_limit: config.total_asset_value_init_limit,
            oracle_max_age: config.oracle_max_age,
            _padding0: [0; 2],
            oracle_max_confidence: config.oracle_max_confidence,
            _padding1: [0; 32],
        }
    }
}

impl From<BankConfig> for BankConfigCompact {
    fn from(config: BankConfig) -> Self {
        Self {
            asset_weight_init: config.asset_weight_init,
            asset_weight_maint: config.asset_weight_maint,
            liability_weight_init: config.liability_weight_init,
            liability_weight_maint: config.liability_weight_maint,
            deposit_limit: config.deposit_limit,
            interest_rate_config: config.interest_rate_config.into(),
            operational_state: config.operational_state,
            borrow_limit: config.borrow_limit,
            risk_tier: config.risk_tier,
            asset_tag: config.asset_tag,
            config_flags: PYTH_PUSH_MIGRATED,
            _pad0: [0; 5],
            total_asset_value_init_limit: config.total_asset_value_init_limit,
            oracle_max_age: config.oracle_max_age,
            oracle_max_confidence: config.oracle_max_confidence,
        }
    }
}

assert_struct_size!(InterestRateConfig, 240);
#[repr(C)]
#[cfg_attr(
    feature = "anchor",
    derive(AnchorDeserialize, AnchorSerialize, TypeLayout)
)]
#[derive(Debug, PartialEq, Eq, Copy, Clone, Pod, Zeroable, Default)]
pub struct InterestRateConfig {
    // Curve Params
    pub optimal_utilization_rate: WrappedI80F48,
    pub plateau_interest_rate: WrappedI80F48,
    pub max_interest_rate: WrappedI80F48,

    // Fees
    /// Goes to insurance, funds `collected_insurance_fees_outstanding`
    pub insurance_fee_fixed_apr: WrappedI80F48,
    /// Goes to insurance, funds `collected_insurance_fees_outstanding`
    pub insurance_ir_fee: WrappedI80F48,
    /// Earned by the group, goes to `collected_group_fees_outstanding`
    pub protocol_fixed_fee_apr: WrappedI80F48,
    /// Earned by the group, goes to `collected_group_fees_outstanding`
    pub protocol_ir_fee: WrappedI80F48,
    pub protocol_origination_fee: WrappedI80F48,

    pub _padding0: [u8; 16],
    pub _padding1: [[u8; 32]; 3],
}

#[cfg_attr(
    feature = "anchor",
    derive(AnchorDeserialize, AnchorSerialize, TypeLayout)
)]
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct InterestRateConfigOpt {
    pub optimal_utilization_rate: Option<WrappedI80F48>,
    pub plateau_interest_rate: Option<WrappedI80F48>,
    pub max_interest_rate: Option<WrappedI80F48>,

    pub insurance_fee_fixed_apr: Option<WrappedI80F48>,
    pub insurance_ir_fee: Option<WrappedI80F48>,
    pub protocol_fixed_fee_apr: Option<WrappedI80F48>,
    pub protocol_ir_fee: Option<WrappedI80F48>,
    pub protocol_origination_fee: Option<WrappedI80F48>,
}

#[repr(C)]
#[cfg_attr(feature = "anchor", derive(AnchorDeserialize, AnchorSerialize))]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct InterestRateConfigCompact {
    // Curve Params
    pub optimal_utilization_rate: WrappedI80F48,
    pub plateau_interest_rate: WrappedI80F48,
    pub max_interest_rate: WrappedI80F48,

    // Fees
    pub insurance_fee_fixed_apr: WrappedI80F48,
    pub insurance_ir_fee: WrappedI80F48,
    pub protocol_fixed_fee_apr: WrappedI80F48,
    pub protocol_ir_fee: WrappedI80F48,
    pub protocol_origination_fee: WrappedI80F48,
}

impl From<InterestRateConfigCompact> for InterestRateConfig {
    fn from(ir_config: InterestRateConfigCompact) -> Self {
        InterestRateConfig {
            optimal_utilization_rate: ir_config.optimal_utilization_rate,
            plateau_interest_rate: ir_config.plateau_interest_rate,
            max_interest_rate: ir_config.max_interest_rate,
            insurance_fee_fixed_apr: ir_config.insurance_fee_fixed_apr,
            insurance_ir_fee: ir_config.insurance_ir_fee,
            protocol_fixed_fee_apr: ir_config.protocol_fixed_fee_apr,
            protocol_ir_fee: ir_config.protocol_ir_fee,
            protocol_origination_fee: ir_config.protocol_origination_fee,
            _padding0: [0; 16],
            _padding1: [[0; 32]; 3],
        }
    }
}

impl From<InterestRateConfig> for InterestRateConfigCompact {
    fn from(ir_config: InterestRateConfig) -> Self {
        InterestRateConfigCompact {
            optimal_utilization_rate: ir_config.optimal_utilization_rate,
            plateau_interest_rate: ir_config.plateau_interest_rate,
            max_interest_rate: ir_config.max_interest_rate,
            insurance_fee_fixed_apr: ir_config.insurance_fee_fixed_apr,
            insurance_ir_fee: ir_config.insurance_ir_fee,
            protocol_fixed_fee_apr: ir_config.protocol_fixed_fee_apr,
            protocol_ir_fee: ir_config.protocol_ir_fee,
            protocol_origination_fee: ir_config.protocol_origination_fee,
        }
    }
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
