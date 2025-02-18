use std::fmt::{Display, Formatter};

use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use fixed::types::I80F48;
use pyth_solana_receiver_sdk::price_update::FeedId;
use type_layout::TypeLayout;

use crate::{
    assert_struct_align, assert_struct_size, check,
    constants::{
        ASSET_TAG_DEFAULT, FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED,
        INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED,
        LIQUIDITY_VAULT_SEED, MAX_ORACLE_KEYS, MAX_PYTH_ORACLE_AGE, MAX_SWB_ORACLE_AGE,
        ORACLE_MIN_AGE, TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE,
    },
    MarginfiError, MarginfiResult,
};

use super::{
    bank_interest::{InterestRateConfig, InterestRateConfigCompact, InterestRateConfigOpt},
    marginfi_account::{BalanceSide, RequirementType},
    marginfi_group::WrappedI80F48,
    price::{OraclePriceFeedAdapter, OracleSetup},
};

#[repr(u8)]
#[derive(Copy, Clone, Debug, AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Default)]
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
impl Display for RiskTier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskTier::Collateral => write!(f, "Collateral"),
            RiskTier::Isolated => write!(f, "Isolated"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BankVaultType {
    Liquidity,
    Insurance,
    Fee,
}

impl BankVaultType {
    pub fn get_seed(self) -> &'static [u8] {
        match self {
            BankVaultType::Liquidity => LIQUIDITY_VAULT_SEED.as_bytes(),
            BankVaultType::Insurance => INSURANCE_VAULT_SEED.as_bytes(),
            BankVaultType::Fee => FEE_VAULT_SEED.as_bytes(),
        }
    }

    pub fn get_authority_seed(self) -> &'static [u8] {
        match self {
            BankVaultType::Liquidity => LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            BankVaultType::Insurance => INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            BankVaultType::Fee => FEE_VAULT_AUTHORITY_SEED.as_bytes(),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, AnchorDeserialize, AnchorSerialize, PartialEq, Eq)]
pub enum BankOperationalState {
    Paused,
    Operational,
    ReduceOnly,
}
unsafe impl Zeroable for BankOperationalState {}
unsafe impl Pod for BankOperationalState {}
impl Display for BankOperationalState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BankOperationalState::Paused => write!(f, "Paused"),
            BankOperationalState::Operational => write!(f, "Operational"),
            BankOperationalState::ReduceOnly => write!(f, "ReduceOnly"),
        }
    }
}

#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Eq)]
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

    pub _pad0: [u8; 6],

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
            _pad0: [0; 6],
            risk_tier: RiskTier::Isolated,
            asset_tag: ASSET_TAG_DEFAULT,
            total_asset_value_init_limit: TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE,
            oracle_max_age: 0,
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
            _pad1: [0; 6],
            total_asset_value_init_limit: config.total_asset_value_init_limit,
            oracle_max_age: config.oracle_max_age,
            _padding0: [0; 6],
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
            _pad0: [0; 6],
            total_asset_value_init_limit: config.total_asset_value_init_limit,
            oracle_max_age: config.oracle_max_age,
        }
    }
}

assert_struct_size!(BankConfig, 544);
assert_struct_align!(BankConfig, 8);
#[repr(C)]
#[derive(
    Debug, Clone, Copy, AnchorDeserialize, AnchorSerialize, Zeroable, Pod, PartialEq, Eq, TypeLayout,
)]
/// TODO: Convert weights to (u64, u64) to avoid precision loss (maybe?)
pub struct BankConfig {
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

    pub _pad1: [u8; 6],

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

    // Note: 6 bytes of padding to next 8 byte alignment, then end padding
    pub _padding0: [u8; 6],
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
            _pad1: [0; 6],
            total_asset_value_init_limit: TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE,
            oracle_max_age: 0,
            _padding0: [0; 6],
            _padding1: [0; 32],
        }
    }
}

impl BankConfig {
    #[inline]
    pub fn get_weights(&self, req_type: RequirementType) -> (I80F48, I80F48) {
        match req_type {
            RequirementType::Initial => (
                self.asset_weight_init.into(),
                self.liability_weight_init.into(),
            ),
            RequirementType::Maintenance => (
                self.asset_weight_maint.into(),
                self.liability_weight_maint.into(),
            ),
            RequirementType::Equity => (I80F48::ONE, I80F48::ONE),
        }
    }

    #[inline]
    pub fn get_weight(
        &self,
        requirement_type: RequirementType,
        balance_side: BalanceSide,
    ) -> I80F48 {
        match (requirement_type, balance_side) {
            (RequirementType::Initial, BalanceSide::Assets) => self.asset_weight_init.into(),
            (RequirementType::Initial, BalanceSide::Liabilities) => {
                self.liability_weight_init.into()
            }
            (RequirementType::Maintenance, BalanceSide::Assets) => self.asset_weight_maint.into(),
            (RequirementType::Maintenance, BalanceSide::Liabilities) => {
                self.liability_weight_maint.into()
            }
            (RequirementType::Equity, _) => I80F48::ONE,
        }
    }

    pub fn validate(&self) -> MarginfiResult {
        let asset_init_w = I80F48::from(self.asset_weight_init);
        let asset_maint_w = I80F48::from(self.asset_weight_maint);

        check!(
            asset_init_w >= I80F48::ZERO && asset_init_w <= I80F48::ONE,
            MarginfiError::InvalidConfig
        );
        check!(asset_maint_w >= asset_init_w, MarginfiError::InvalidConfig);

        let liab_init_w = I80F48::from(self.liability_weight_init);
        let liab_maint_w = I80F48::from(self.liability_weight_maint);

        check!(liab_init_w >= I80F48::ONE, MarginfiError::InvalidConfig);
        check!(
            liab_maint_w <= liab_init_w && liab_maint_w >= I80F48::ONE,
            MarginfiError::InvalidConfig
        );

        self.interest_rate_config.validate()?;

        if self.risk_tier == RiskTier::Isolated {
            check!(asset_init_w == I80F48::ZERO, MarginfiError::InvalidConfig);
            check!(asset_maint_w == I80F48::ZERO, MarginfiError::InvalidConfig);
        }

        Ok(())
    }

    #[inline]
    pub fn is_deposit_limit_active(&self) -> bool {
        self.deposit_limit != u64::MAX
    }

    #[inline]
    pub fn is_borrow_limit_active(&self) -> bool {
        self.borrow_limit != u64::MAX
    }

    /// * lst_mint, stake_pool, sol_pool - required only if configuring
    ///   `OracleSetup::StakedWithPythPush` on initial setup. If configuring a staked bank after
    ///   initial setup, can be omitted
    pub fn validate_oracle_setup(
        &self,
        ais: &[AccountInfo],
        lst_mint: Option<Pubkey>,
        stake_pool: Option<Pubkey>,
        sol_pool: Option<Pubkey>,
    ) -> MarginfiResult {
        OraclePriceFeedAdapter::validate_bank_config(self, ais, lst_mint, stake_pool, sol_pool)?;
        Ok(())
    }

    pub fn validate_oracle_age(&self) -> MarginfiResult {
        check!(
            self.oracle_max_age >= ORACLE_MIN_AGE,
            MarginfiError::InvalidOracleSetup
        );
        Ok(())
    }

    pub fn usd_init_limit_active(&self) -> bool {
        self.total_asset_value_init_limit != TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE
    }

    #[inline]
    pub fn get_oracle_max_age(&self) -> u64 {
        match (self.oracle_max_age, self.oracle_setup) {
            (0, OracleSetup::SwitchboardV2) => MAX_SWB_ORACLE_AGE,
            (0, OracleSetup::PythLegacy | OracleSetup::PythPushOracle) => MAX_PYTH_ORACLE_AGE,
            (n, _) => n as u64,
        }
    }

    pub fn get_pyth_push_oracle_feed_id(&self) -> Option<&FeedId> {
        if matches!(
            self.oracle_setup,
            OracleSetup::PythPushOracle | OracleSetup::StakedWithPythPush
        ) {
            let bytes: &[u8; 32] = self.oracle_keys[0].as_ref().try_into().unwrap();
            Some(bytes)
        } else {
            None
        }
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Default, Clone, PartialEq, Eq, TypeLayout)]
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

    pub oracle_max_age: Option<u16>,

    pub permissionless_bad_debt_settlement: Option<bool>,

    pub freeze_settings: Option<bool>,
}
