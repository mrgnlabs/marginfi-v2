use super::price::OracleSetup;
use crate::borsh::{BorshDeserialize, BorshSerialize};
use crate::{
    assert_struct_size, check,
    constants::{
        FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, INSURANCE_VAULT_AUTHORITY_SEED,
        INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED,
        MAX_ORACLE_KEYS, PYTH_ID,
    },
    prelude::MarginfiError,
    set_if_some, MarginfiResult,
};
use anchor_lang::prelude::borsh;
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use fixed::types::I80F48;
use pyth_sdk_solana::{state::SolanaPriceAccount, PriceFeed};
#[cfg(feature = "client")]
use std::fmt::Display;
use std::fmt::{Debug, Formatter};

#[cfg(any(feature = "test", feature = "client"))]
use type_layout::TypeLayout;

assert_struct_size!(MarginfiGroup, 1056);
#[account(zero_copy)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq, TypeLayout)
)]
#[derive(Default)]
pub struct MarginfiGroup {
    pub admin: Pubkey,
    /// Bitmask for group settings flags.
    /// * Bit 0: If set, program-level fees are enabled.
    /// * Bits 1-63: Reserved for future use.
    pub group_flags: u64,
    /// Caches information from the global `FeeState` so the FeeState can be omitted on certain ixes
    pub fee_state_cache: FeeStateCache,
    pub _padding_0: [[u64; 2]; 27],
    pub _padding_1: [[u64; 2]; 32],
    pub _padding_3: u64,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Zeroable, Pod, Debug, PartialEq, Eq,
)]
#[repr(C)]
pub struct FeeStateCache {
    pub global_fee_wallet: Pubkey,
    pub program_fee_fixed: WrappedI80F48,
    pub program_fee_rate: WrappedI80F48,
}

impl MarginfiGroup {
    const PROGRAM_FEES_ENABLED: u64 = 1;

    /// Bits in use for flag settings.
    const ALLOWED_FLAGS: u64 = Self::PROGRAM_FEES_ENABLED;
    // To add: const ALLOWED_FLAGS: u64 = PROGRAM_FEES_ENABLED | ANOTHER_FEATURE_BIT;

    /// Configure the group parameters.
    /// This function validates config values so the group remains in a valid state.
    /// Any modification of group config should happen through this function.
    pub fn configure(&mut self, config: &GroupConfig) -> MarginfiResult {
        set_if_some!(self.admin, config.admin);

        Ok(())
    }

    /// Set the group parameters when initializing a group.
    /// This should be called only when the group is first initialized.
    /// Both margin requirements are initially set to 100% and should be configured before use.
    #[allow(clippy::too_many_arguments)]
    pub fn set_initial_configuration(&mut self, admin_pk: Pubkey) {
        self.admin = admin_pk;
        self.group_flags = Self::PROGRAM_FEES_ENABLED;
    }

    pub fn get_group_bank_config(&self) -> GroupBankConfig {
        GroupBankConfig {
            program_fees: self.group_flags == Self::PROGRAM_FEES_ENABLED,
        }
    }

    /// Validates that only allowed flags are being set.
    pub fn validate_flags(flag: u64) -> MarginfiResult {
        // Note: 0xnnnn & 0x1110, is nonzero for 0x1000 & 0x1110
        let flag_ok = flag & !Self::ALLOWED_FLAGS == 0;
        check!(flag_ok, MarginfiError::IllegalFlag);

        Ok(())
    }

    /// Sets flag and errors if a disallowed flag is set
    pub fn set_flags(&mut self, flag: u64) -> MarginfiResult {
        Self::validate_flags(flag)?;
        self.group_flags = flag;
        Ok(())
    }

    /// True if program fees are enabled
    pub fn program_fees_enabled(&self) -> bool {
        (self.group_flags & Self::PROGRAM_FEES_ENABLED) != 0
    }
}

#[cfg_attr(any(feature = "test", feature = "client"), derive(TypeLayout))]
#[derive(AnchorSerialize, AnchorDeserialize, Default, Debug, Clone)]
pub struct GroupConfig {
    pub admin: Option<Pubkey>,
}

/// Load and validate a pyth price feed account.
pub fn load_pyth_price_feed(ai: &AccountInfo) -> MarginfiResult<PriceFeed> {
    check!(ai.owner.eq(&PYTH_ID), MarginfiError::InvalidOracleAccount);
    let price_feed = SolanaPriceAccount::account_info_to_feed(ai)
        .map_err(|_| MarginfiError::InvalidOracleAccount)?;
    Ok(price_feed)
}

/// Group level configuration to be used in bank accounts.
#[derive(Clone, Debug)]
pub struct GroupBankConfig {
    pub program_fees: bool,
}

#[repr(u8)]
#[cfg_attr(any(feature = "test", feature = "client"), derive(PartialEq, Eq))]
#[derive(Copy, Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum BankOperationalState {
    Paused,
    Operational,
    ReduceOnly,
}

#[cfg(feature = "client")]
impl Display for BankOperationalState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BankOperationalState::Paused => write!(f, "Paused"),
            BankOperationalState::Operational => write!(f, "Operational"),
            BankOperationalState::ReduceOnly => write!(f, "ReduceOnly"),
        }
    }
}

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

#[zero_copy]
#[repr(C, align(8))]
#[cfg_attr(any(feature = "test", feature = "client"), derive(TypeLayout))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct WrappedI80F48 {
    pub value: [u8; 16],
}

impl Debug for WrappedI80F48 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", I80F48::from_le_bytes(self.value))
    }
}

impl From<I80F48> for WrappedI80F48 {
    fn from(i: I80F48) -> Self {
        Self {
            value: i.to_le_bytes(),
        }
    }
}

impl From<WrappedI80F48> for I80F48 {
    fn from(w: WrappedI80F48) -> Self {
        Self::from_le_bytes(w.value)
    }
}

impl PartialEq for WrappedI80F48 {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for WrappedI80F48 {}

#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(PartialEq, Eq, TypeLayout)
)]
#[derive(Clone, Copy, AnchorDeserialize, AnchorSerialize, Debug)]
pub struct OracleConfig {
    pub setup: OracleSetup,
    pub keys: [Pubkey; MAX_ORACLE_KEYS],
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
