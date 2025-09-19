use anchor_lang::prelude::*;
use fixed_macro::types::I80F48;
use marginfi_type_crate::constants::{ASSET_TAG_KAMINO, PYTH_PUSH_MIGRATED_DEPRECATED};
use marginfi_type_crate::types::{
    BankConfig, BankOperationalState, InterestRateConfig, OracleSetup, RiskTier, WrappedI80F48,
};

/// Used to configure Kamino banks. A simplified version of `BankConfigCompact` which omits most
/// values related to interest since Kamino banks cannot earn interest or be borrowed against.
// TODO: Jon mentioned there are some extra options he wants to see in config, investigate later.
#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Eq)]
pub struct KaminoConfigCompact {
    pub oracle: Pubkey,
    pub asset_weight_init: WrappedI80F48,
    pub asset_weight_maint: WrappedI80F48,
    pub deposit_limit: u64,
    /// Either `KaminoPythPush` or `KaminoSwitchboardPull`
    pub oracle_setup: OracleSetup,
    /// Bank operational state - allows starting banks in paused state
    pub operational_state: BankOperationalState,
    /// Risk tier - determines if assets can be borrowed in isolation
    pub risk_tier: RiskTier,
    /// Config flags for future-proofing
    pub config_flags: u8,
    pub total_asset_value_init_limit: u64,
    /// Currently unused: Kamino's oracle age applies to kamino banks.
    pub oracle_max_age: u16,
    /// Oracle confidence threshold (0 = use default 10%)
    pub oracle_max_confidence: u32,
}

impl KaminoConfigCompact {
    pub const LEN: usize = std::mem::size_of::<KaminoConfigCompact>();

    pub fn new(
        oracle: Pubkey,
        asset_weight_init: WrappedI80F48,
        asset_weight_maint: WrappedI80F48,
        deposit_limit: u64,
        oracle_setup: OracleSetup,
        operational_state: BankOperationalState,
        risk_tier: RiskTier,
        config_flags: u8,
        total_asset_value_init_limit: u64,
        oracle_max_age: u16,
        oracle_max_confidence: u32,
    ) -> Self {
        KaminoConfigCompact {
            oracle,
            asset_weight_init,
            asset_weight_maint,
            deposit_limit,
            oracle_setup,
            operational_state,
            risk_tier,
            config_flags,
            total_asset_value_init_limit,
            oracle_max_age,
            oracle_max_confidence,
        }
    }

    /// Convert to BankConfig with the reserve key for Kamino banks
    pub fn to_bank_config(&self, reserve_key: Pubkey) -> BankConfig {
        // These are placeholder values: Kamino positions do not support borrowing and likely
        // never will, thus they will earn no interest.
        // Note: Some placeholder values are non-zero to handle downstream validation checks.
        let default_ir_config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let keys = [
            self.oracle,
            reserve_key,
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
        ];

        BankConfig {
            asset_weight_init: self.asset_weight_init,
            asset_weight_maint: self.asset_weight_maint,
            liability_weight_init: I80F48!(1.5).into(), // placeholder
            liability_weight_maint: I80F48!(1.25).into(), // placeholder
            deposit_limit: self.deposit_limit,
            interest_rate_config: default_ir_config,
            operational_state: self.operational_state,
            oracle_setup: self.oracle_setup,
            oracle_keys: keys,
            _pad0: [0; 6],
            borrow_limit: 0, // Can't ever borrow kamino assets
            risk_tier: self.risk_tier,
            asset_tag: ASSET_TAG_KAMINO,
            config_flags: self.config_flags,
            _pad1: [0; 5],
            total_asset_value_init_limit: self.total_asset_value_init_limit,
            oracle_max_age: self.oracle_max_age,
            _padding0: [0; 2],
            oracle_max_confidence: self.oracle_max_confidence,
            _padding1: [0; 32],
        }
    }
}

impl Default for KaminoConfigCompact {
    fn default() -> Self {
        KaminoConfigCompact {
            oracle: Pubkey::default(),
            asset_weight_init: I80F48!(0.8).into(),
            asset_weight_maint: I80F48!(0.9).into(),
            deposit_limit: 1_000_000,
            oracle_setup: OracleSetup::KaminoPythPush,
            operational_state: BankOperationalState::Operational,
            risk_tier: RiskTier::Collateral,
            config_flags: PYTH_PUSH_MIGRATED_DEPRECATED,
            total_asset_value_init_limit: 1_000_000,
            oracle_max_age: 10,
            oracle_max_confidence: 0, // Use default 10%
        }
    }
}
