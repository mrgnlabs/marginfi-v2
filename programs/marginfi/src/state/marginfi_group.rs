use super::{
    marginfi_account::{BalanceSide, RequirementType},
    price::{OraclePriceFeedAdapter, OracleSetup},
};
#[cfg(not(feature = "client"))]
use crate::events::{GroupEventHeader, LendingPoolBankAccrueInterestEvent};
use crate::{
    assert_struct_align, assert_struct_size, check,
    constants::{
        EMISSION_FLAGS, FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, GROUP_FLAGS,
        INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED,
        LIQUIDITY_VAULT_SEED, MAX_ORACLE_KEYS, MAX_PYTH_ORACLE_AGE, MAX_SWB_ORACLE_AGE,
        PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG, PYTH_ID, SECONDS_PER_YEAR,
        TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE,
    },
    debug, math_error,
    prelude::MarginfiError,
    set_if_some,
    state::marginfi_account::calc_value,
    MarginfiResult,
};
use crate::{
    borsh::{BorshDeserialize, BorshSerialize},
    constants::ASSET_TAG_DEFAULT,
};
use anchor_lang::prelude::borsh;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::*;
use bytemuck::{Pod, Zeroable};
use fixed::types::I80F48;
use pyth_sdk_solana::{state::SolanaPriceAccount, PriceFeed};
use pyth_solana_receiver_sdk::price_update::FeedId;
#[cfg(feature = "client")]
use std::fmt::Display;
use std::{
    fmt::{Debug, Formatter},
    ops::Not,
};

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

#[repr(C)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(PartialEq, Eq, TypeLayout)
)]
#[derive(Default, Debug, AnchorDeserialize, AnchorSerialize)]
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

assert_struct_size!(InterestRateConfig, 240);
#[zero_copy]
#[repr(C)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(PartialEq, Eq, TypeLayout)
)]
#[derive(Default, Debug)]
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

impl InterestRateConfig {
    pub fn create_interest_rate_calculator(&self, group: &MarginfiGroup) -> InterestRateCalc {
        let group_bank_config = &group.get_group_bank_config();
        debug!(
            "Creating interest rate calculator with protocol fees: {}",
            group_bank_config.program_fees
        );
        InterestRateCalc {
            optimal_utilization_rate: self.optimal_utilization_rate.into(),
            plateau_interest_rate: self.plateau_interest_rate.into(),
            max_interest_rate: self.max_interest_rate.into(),
            insurance_fixed_fee: self.insurance_fee_fixed_apr.into(),
            insurance_rate_fee: self.insurance_ir_fee.into(),
            protocol_fixed_fee: self.protocol_fixed_fee_apr.into(),
            protocol_rate_fee: self.protocol_ir_fee.into(),
            add_program_fees: group_bank_config.program_fees,
            program_fee_fixed: group.fee_state_cache.program_fee_fixed.into(),
            program_fee_rate: group.fee_state_cache.program_fee_rate.into(),
        }
    }

    pub fn validate(&self) -> MarginfiResult {
        let optimal_ur: I80F48 = self.optimal_utilization_rate.into();
        let plateau_ir: I80F48 = self.plateau_interest_rate.into();
        let max_ir: I80F48 = self.max_interest_rate.into();

        check!(
            optimal_ur > I80F48::ZERO && optimal_ur < I80F48::ONE,
            MarginfiError::InvalidConfig
        );
        check!(plateau_ir > I80F48::ZERO, MarginfiError::InvalidConfig);
        check!(max_ir > I80F48::ZERO, MarginfiError::InvalidConfig);
        check!(plateau_ir < max_ir, MarginfiError::InvalidConfig);

        Ok(())
    }

    pub fn update(&mut self, ir_config: &InterestRateConfigOpt) {
        set_if_some!(
            self.optimal_utilization_rate,
            ir_config.optimal_utilization_rate
        );
        set_if_some!(self.plateau_interest_rate, ir_config.plateau_interest_rate);
        set_if_some!(self.max_interest_rate, ir_config.max_interest_rate);
        set_if_some!(
            self.insurance_fee_fixed_apr,
            ir_config.insurance_fee_fixed_apr
        );
        set_if_some!(self.insurance_ir_fee, ir_config.insurance_ir_fee);
        set_if_some!(
            self.protocol_fixed_fee_apr,
            ir_config.protocol_fixed_fee_apr
        );
        set_if_some!(self.protocol_ir_fee, ir_config.protocol_ir_fee);
    }
}

#[derive(Debug, Clone)]
/// Short for calculator
pub struct InterestRateCalc {
    optimal_utilization_rate: I80F48,
    plateau_interest_rate: I80F48,
    max_interest_rate: I80F48,

    // Fees
    insurance_fixed_fee: I80F48,
    insurance_rate_fee: I80F48,
    /// AKA group fixed fee
    protocol_fixed_fee: I80F48,
    /// AKA group rate fee
    protocol_rate_fee: I80F48,

    program_fee_fixed: I80F48,
    program_fee_rate: I80F48,

    add_program_fees: bool,
}

impl InterestRateCalc {
    /// Return interest rate charged to borrowers and to depositors.
    /// Rate is denominated in APR (0-).
    ///
    /// Return ComputedInterestRates
    pub fn calc_interest_rate(&self, utilization_ratio: I80F48) -> Option<ComputedInterestRates> {
        let Fees {
            insurance_fee_rate,
            insurance_fee_fixed,
            group_fee_rate,
            group_fee_fixed,
            protocol_fee_rate,
            protocol_fee_fixed,
        } = self.get_fees();

        let fee_ir = insurance_fee_rate + group_fee_rate + protocol_fee_rate;
        let fee_fixed = insurance_fee_fixed + group_fee_fixed + protocol_fee_fixed;

        let base_rate = self.interest_rate_curve(utilization_ratio)?;

        // Lending rate is adjusted for utilization ratio to symmetrize payments between borrowers and depositors.
        let lending_rate_apr = base_rate.checked_mul(utilization_ratio)?;

        // Borrowing rate is adjusted for fees.
        // borrowing_rate = base_rate + base_rate * rate_fee + total_fixed_fee_apr
        let borrowing_rate_apr = base_rate
            .checked_mul(I80F48::ONE.checked_add(fee_ir)?)?
            .checked_add(fee_fixed)?;

        let group_fee_apr = calc_fee_rate(base_rate, group_fee_rate, group_fee_fixed)?;
        let insurance_fee_apr = calc_fee_rate(base_rate, insurance_fee_rate, insurance_fee_fixed)?;
        let protocol_fee_apr = calc_fee_rate(base_rate, protocol_fee_rate, protocol_fee_fixed)?;

        assert!(lending_rate_apr >= I80F48::ZERO);
        assert!(borrowing_rate_apr >= I80F48::ZERO);
        assert!(group_fee_apr >= I80F48::ZERO);
        assert!(insurance_fee_apr >= I80F48::ZERO);
        assert!(protocol_fee_apr >= I80F48::ZERO);

        // TODO: Add liquidation discount check
        Some(ComputedInterestRates {
            lending_rate_apr,
            borrowing_rate_apr,
            group_fee_apr,
            insurance_fee_apr,
            protocol_fee_apr,
        })
    }

    /// Piecewise linear interest rate function.
    /// The curves approaches the `plateau_interest_rate` as the utilization ratio approaches the `optimal_utilization_rate`,
    /// once the utilization ratio exceeds the `optimal_utilization_rate`, the curve approaches the `max_interest_rate`.
    ///
    /// To be clear we don't particularly appreciate the piecewise linear nature of this "curve", but it is what it is.
    #[inline]
    fn interest_rate_curve(&self, ur: I80F48) -> Option<I80F48> {
        let optimal_ur: I80F48 = self.optimal_utilization_rate;
        let plateau_ir: I80F48 = self.plateau_interest_rate;
        let max_ir: I80F48 = self.max_interest_rate;

        if ur <= optimal_ur {
            ur.checked_div(optimal_ur)?.checked_mul(plateau_ir)
        } else {
            (ur - optimal_ur)
                .checked_div(I80F48::ONE - optimal_ur)?
                .checked_mul(max_ir - plateau_ir)?
                .checked_add(plateau_ir)
        }
    }

    pub fn get_fees(&self) -> Fees {
        let (protocol_fee_rate, protocol_fee_fixed) = if self.add_program_fees {
            (self.program_fee_rate, self.program_fee_fixed)
        } else {
            (I80F48::ZERO, I80F48::ZERO)
        };

        Fees {
            insurance_fee_rate: self.insurance_rate_fee,
            insurance_fee_fixed: self.insurance_fixed_fee,
            group_fee_rate: self.protocol_rate_fee,
            group_fee_fixed: self.protocol_fixed_fee,
            protocol_fee_rate,
            protocol_fee_fixed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Fees {
    pub insurance_fee_rate: I80F48,
    pub insurance_fee_fixed: I80F48,
    pub group_fee_rate: I80F48,
    pub group_fee_fixed: I80F48,
    pub protocol_fee_rate: I80F48,
    pub protocol_fee_fixed: I80F48,
}

#[derive(Debug, Clone)]
pub struct ComputedInterestRates {
    pub lending_rate_apr: I80F48,
    pub borrowing_rate_apr: I80F48,
    pub group_fee_apr: I80F48,
    pub insurance_fee_apr: I80F48,
    pub protocol_fee_apr: I80F48,
}

#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq, TypeLayout)
)]
#[derive(AnchorDeserialize, AnchorSerialize, Default, Clone)]
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

/// Group level configuration to be used in bank accounts.
#[derive(Clone, Debug)]
pub struct GroupBankConfig {
    pub program_fees: bool,
}

assert_struct_size!(Bank, 1856);
assert_struct_align!(Bank, 8);
#[account(zero_copy(unsafe))]
#[repr(C)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq, TypeLayout)
)]
#[derive(Default)]
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
    ///
    pub flags: u64,
    /// Emissions APR.
    /// Number of emitted tokens (emissions_mint) per 1e(bank.mint_decimal) tokens (bank mint) (native amount) per 1 YEAR.
    pub emissions_rate: u64,
    pub emissions_remaining: WrappedI80F48,
    pub emissions_mint: Pubkey,

    /// Fees collected and pending withdraw for the `FeeState.global_fee_wallet`'s cannonical ATA for `mint`
    pub collected_program_fees_outstanding: WrappedI80F48,

    pub _padding_0: [[u64; 2]; 27],
    pub _padding_1: [[u64; 2]; 32], // 16 * 2 * 32 = 1024B
}

impl Bank {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        marginfi_group_pk: Pubkey,
        config: BankConfig,
        mint: Pubkey,
        mint_decimals: u8,
        liquidity_vault: Pubkey,
        insurance_vault: Pubkey,
        fee_vault: Pubkey,
        current_timestamp: i64,
        liquidity_vault_bump: u8,
        liquidity_vault_authority_bump: u8,
        insurance_vault_bump: u8,
        insurance_vault_authority_bump: u8,
        fee_vault_bump: u8,
        fee_vault_authority_bump: u8,
    ) -> Bank {
        Bank {
            mint,
            mint_decimals,
            group: marginfi_group_pk,
            asset_share_value: I80F48::ONE.into(),
            liability_share_value: I80F48::ONE.into(),
            liquidity_vault,
            liquidity_vault_bump,
            liquidity_vault_authority_bump,
            insurance_vault,
            insurance_vault_bump,
            insurance_vault_authority_bump,
            collected_insurance_fees_outstanding: I80F48::ZERO.into(),
            fee_vault,
            fee_vault_bump,
            fee_vault_authority_bump,
            collected_group_fees_outstanding: I80F48::ZERO.into(),
            total_liability_shares: I80F48::ZERO.into(),
            total_asset_shares: I80F48::ZERO.into(),
            last_update: current_timestamp,
            config,
            flags: 0,
            emissions_rate: 0,
            emissions_remaining: I80F48::ZERO.into(),
            emissions_mint: Pubkey::default(),
            collected_program_fees_outstanding: I80F48::ZERO.into(),
            ..Default::default()
        }
    }

    pub fn get_liability_amount(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.liability_share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub fn get_asset_amount(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.asset_share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub fn get_liability_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.liability_share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub fn get_asset_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.asset_share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub fn change_asset_shares(
        &mut self,
        shares: I80F48,
        bypass_deposit_limit: bool,
    ) -> MarginfiResult {
        let total_asset_shares: I80F48 = self.total_asset_shares.into();
        self.total_asset_shares = total_asset_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?
            .into();

        if shares.is_positive() && self.config.is_deposit_limit_active() && !bypass_deposit_limit {
            let total_deposits_amount = self.get_asset_amount(self.total_asset_shares.into())?;
            let deposit_limit = I80F48::from_num(self.config.deposit_limit);

            check!(
                total_deposits_amount < deposit_limit,
                crate::prelude::MarginfiError::BankAssetCapacityExceeded
            )
        }

        Ok(())
    }

    pub fn maybe_get_asset_weight_init_discount(
        &self,
        price: I80F48,
    ) -> MarginfiResult<Option<I80F48>> {
        if self.config.usd_init_limit_active() {
            let bank_total_assets_value = calc_value(
                self.get_asset_amount(self.total_asset_shares.into())?,
                price,
                self.mint_decimals,
                None,
            )?;

            let total_asset_value_init_limit =
                I80F48::from_num(self.config.total_asset_value_init_limit);

            #[cfg(target_os = "solana")]
            debug!(
                "Init limit active, limit: {}, total_assets: {}",
                total_asset_value_init_limit, bank_total_assets_value
            );

            if bank_total_assets_value > total_asset_value_init_limit {
                let discount = total_asset_value_init_limit
                    .checked_div(bank_total_assets_value)
                    .ok_or_else(math_error!())?;

                #[cfg(target_os = "solana")]
                debug!(
                    "Discounting assets by {:.2} because of total deposits {} over {} usd cap",
                    discount, bank_total_assets_value, total_asset_value_init_limit
                );

                Ok(Some(discount))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn change_liability_shares(
        &mut self,
        shares: I80F48,
        bypass_borrow_limit: bool,
    ) -> MarginfiResult {
        let total_liability_shares: I80F48 = self.total_liability_shares.into();
        self.total_liability_shares = total_liability_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?
            .into();

        if bypass_borrow_limit.not() && shares.is_positive() && self.config.is_borrow_limit_active()
        {
            let total_liability_amount =
                self.get_liability_amount(self.total_liability_shares.into())?;
            let borrow_limit = I80F48::from_num(self.config.borrow_limit);

            check!(
                total_liability_amount < borrow_limit,
                crate::prelude::MarginfiError::BankLiabilityCapacityExceeded
            )
        }

        Ok(())
    }

    pub fn check_utilization_ratio(&self) -> MarginfiResult {
        let total_assets = self.get_asset_amount(self.total_asset_shares.into())?;
        let total_liabilities = self.get_liability_amount(self.total_liability_shares.into())?;

        check!(
            total_assets >= total_liabilities,
            crate::prelude::MarginfiError::IllegalUtilizationRatio
        );

        Ok(())
    }

    pub fn configure(&mut self, config: &BankConfigOpt) -> MarginfiResult {
        set_if_some!(self.config.asset_weight_init, config.asset_weight_init);
        set_if_some!(self.config.asset_weight_maint, config.asset_weight_maint);
        set_if_some!(
            self.config.liability_weight_init,
            config.liability_weight_init
        );
        set_if_some!(
            self.config.liability_weight_maint,
            config.liability_weight_maint
        );
        set_if_some!(self.config.deposit_limit, config.deposit_limit);

        set_if_some!(self.config.borrow_limit, config.borrow_limit);

        set_if_some!(self.config.operational_state, config.operational_state);

        set_if_some!(self.config.oracle_setup, config.oracle.map(|o| o.setup));

        set_if_some!(self.config.oracle_keys, config.oracle.map(|o| o.keys));

        if let Some(ir_config) = &config.interest_rate_config {
            self.config.interest_rate_config.update(ir_config);
        }

        set_if_some!(self.config.risk_tier, config.risk_tier);

        set_if_some!(self.config.asset_tag, config.asset_tag);

        set_if_some!(
            self.config.total_asset_value_init_limit,
            config.total_asset_value_init_limit
        );

        set_if_some!(self.config.oracle_max_age, config.oracle_max_age);

        if let Some(flag) = config.permissionless_bad_debt_settlement {
            self.update_flag(flag, PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG);
        }

        self.config.validate()?;

        Ok(())
    }

    /// Calculate the interest rate accrual state changes for a given time period
    ///
    /// Collected protocol and insurance fees are stored in state.
    /// A separate instruction is required to withdraw these fees.
    pub fn accrue_interest(
        &mut self,
        current_timestamp: i64,
        group: &MarginfiGroup,
        #[cfg(not(feature = "client"))] bank: Pubkey,
    ) -> MarginfiResult<()> {
        #[cfg(all(not(feature = "client"), feature = "debug"))]
        solana_program::log::sol_log_compute_units();

        let time_delta: u64 = (current_timestamp - self.last_update).try_into().unwrap();

        if time_delta == 0 {
            return Ok(());
        }

        let total_assets = self.get_asset_amount(self.total_asset_shares.into())?;
        let total_liabilities = self.get_liability_amount(self.total_liability_shares.into())?;

        self.last_update = current_timestamp;

        if (total_assets == I80F48::ZERO) || (total_liabilities == I80F48::ZERO) {
            #[cfg(not(feature = "client"))]
            emit!(LendingPoolBankAccrueInterestEvent {
                header: GroupEventHeader {
                    marginfi_group: self.group,
                    signer: None
                },
                bank,
                mint: self.mint,
                delta: time_delta,
                fees_collected: 0.,
                insurance_collected: 0.,
            });

            return Ok(());
        }
        let ir_calc = self
            .config
            .interest_rate_config
            .create_interest_rate_calculator(group);

        let InterestRateStateChanges {
            new_asset_share_value: asset_share_value,
            new_liability_share_value: liability_share_value,
            insurance_fees_collected,
            group_fees_collected,
            protocol_fees_collected,
        } = calc_interest_rate_accrual_state_changes(
            time_delta,
            total_assets,
            total_liabilities,
            &ir_calc,
            self.asset_share_value.into(),
            self.liability_share_value.into(),
        )
        .ok_or_else(math_error!())?;

        debug!("deposit share value: {}\nliability share value: {}\nfees collected: {}\ninsurance collected: {}",
            asset_share_value, liability_share_value, group_fees_collected, insurance_fees_collected);

        self.asset_share_value = asset_share_value.into();
        self.liability_share_value = liability_share_value.into();

        if group_fees_collected > I80F48::ZERO {
            self.collected_group_fees_outstanding = {
                group_fees_collected
                    .checked_add(self.collected_group_fees_outstanding.into())
                    .ok_or_else(math_error!())?
                    .into()
            };
        }

        if insurance_fees_collected > I80F48::ZERO {
            self.collected_insurance_fees_outstanding = {
                insurance_fees_collected
                    .checked_add(self.collected_insurance_fees_outstanding.into())
                    .ok_or_else(math_error!())?
                    .into()
            };
        }
        if protocol_fees_collected > I80F48::ZERO {
            self.collected_program_fees_outstanding = {
                protocol_fees_collected
                    .checked_add(self.collected_program_fees_outstanding.into())
                    .ok_or_else(math_error!())?
                    .into()
            };
        }

        #[cfg(not(feature = "client"))]
        {
            #[cfg(feature = "debug")]
            solana_program::log::sol_log_compute_units();

            emit!(LendingPoolBankAccrueInterestEvent {
                header: GroupEventHeader {
                    marginfi_group: self.group,
                    signer: None
                },
                bank,
                mint: self.mint,
                delta: time_delta,
                fees_collected: group_fees_collected.to_num::<f64>(),
                insurance_collected: insurance_fees_collected.to_num::<f64>(),
            });
        }

        Ok(())
    }

    pub fn deposit_spl_transfer<'info>(
        &self,
        amount: u64,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        maybe_mint: Option<&InterfaceAccount<'info, Mint>>,
        program: AccountInfo<'info>,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> MarginfiResult {
        check!(
            to.key.eq(&self.liquidity_vault),
            MarginfiError::InvalidTransfer
        );

        debug!(
            "deposit_spl_transfer: amount: {} from {} to {}, auth {}",
            amount, from.key, to.key, authority.key
        );

        if let Some(mint) = maybe_mint {
            spl_token_2022::onchain::invoke_transfer_checked(
                program.key,
                from,
                mint.to_account_info(),
                to,
                authority,
                remaining_accounts,
                amount,
                mint.decimals,
                &[],
            )?;
        } else {
            #[allow(deprecated)]
            transfer(
                CpiContext::new_with_signer(
                    program,
                    Transfer {
                        from,
                        to,
                        authority,
                    },
                    &[],
                ),
                amount,
            )?;
        }

        Ok(())
    }

    pub fn withdraw_spl_transfer<'info>(
        &self,
        amount: u64,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        maybe_mint: Option<&InterfaceAccount<'info, Mint>>,
        program: AccountInfo<'info>,
        signer_seeds: &[&[&[u8]]],
        remaining_accounts: &[AccountInfo<'info>],
    ) -> MarginfiResult {
        debug!(
            "withdraw_spl_transfer: amount: {} from {} to {}, auth {}",
            amount, from.key, to.key, authority.key
        );

        if let Some(mint) = maybe_mint {
            spl_token_2022::onchain::invoke_transfer_checked(
                program.key,
                from,
                mint.to_account_info(),
                to,
                authority,
                remaining_accounts,
                amount,
                mint.decimals,
                signer_seeds,
            )?;
        } else {
            // `transfer_checked` and `transfer` does the same thing, the additional `_checked` logic
            // is only to assert the expected attributes by the user (mint, decimal scaling),
            //
            // Security of `transfer` is equal to `transfer_checked`.
            #[allow(deprecated)]
            transfer(
                CpiContext::new_with_signer(
                    program,
                    Transfer {
                        from,
                        to,
                        authority,
                    },
                    signer_seeds,
                ),
                amount,
            )?;
        }

        Ok(())
    }

    /// Socialize a loss `loss_amount` among depositors,
    /// the `total_deposit_shares` stays the same, but total value of deposits is
    /// reduced by `loss_amount`;
    pub fn socialize_loss(&mut self, loss_amount: I80F48) -> MarginfiResult {
        let total_asset_shares: I80F48 = self.total_asset_shares.into();
        let old_asset_share_value: I80F48 = self.asset_share_value.into();

        let new_share_value = total_asset_shares
            .checked_mul(old_asset_share_value)
            .ok_or_else(math_error!())?
            .checked_sub(loss_amount)
            .ok_or_else(math_error!())?
            .checked_div(total_asset_shares)
            .ok_or_else(math_error!())?;

        self.asset_share_value = new_share_value.into();

        Ok(())
    }

    pub fn assert_operational_mode(
        &self,
        is_asset_or_liability_amount_increasing: Option<bool>,
    ) -> Result<()> {
        match self.config.operational_state {
            BankOperationalState::Paused => Err(MarginfiError::BankPaused.into()),
            BankOperationalState::Operational => Ok(()),
            BankOperationalState::ReduceOnly => {
                if let Some(is_asset_or_liability_amount_increasing) =
                    is_asset_or_liability_amount_increasing
                {
                    check!(
                        !is_asset_or_liability_amount_increasing,
                        MarginfiError::BankReduceOnly
                    );
                }

                Ok(())
            }
        }
    }

    pub fn get_flag(&self, flag: u64) -> bool {
        (self.flags & flag) == flag
    }

    pub(crate) fn override_emissions_flag(&mut self, flag: u64) {
        assert!(Self::verify_emissions_flags(flag));
        self.flags = flag;
    }

    pub(crate) fn update_flag(&mut self, value: bool, flag: u64) {
        assert!(Self::verify_group_flags(flag));

        if value {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }

    const fn verify_emissions_flags(flags: u64) -> bool {
        flags & EMISSION_FLAGS == flags
    }

    const fn verify_group_flags(flags: u64) -> bool {
        flags & GROUP_FLAGS == flags
    }
}

/// We use a simple interest rate model that auto settles the accrued interest into the lending account balances.
/// The plan is to move to a compound interest model in the future.
///
/// Simple interest rate model:
/// - `P` - principal
/// - `i` - interest rate (per second)
/// - `t` - time (in seconds)
///
/// `P_t = P_0 * (1 + i) * t`
///
/// We use two interest rates, one for lending and one for borrowing.
///
/// Lending interest rate:
/// - `i_l` - lending interest rate
/// - `i` - base interest rate
/// - `ur` - utilization rate
///
/// `i_l` = `i` * `ur`
///
/// Borrowing interest rate:
/// - `i_b` - borrowing interest rate
/// - `i` - base interest rate
/// - `f_i` - interest rate fee
/// - `f_f` - fixed fee
///
/// `i_b = i * (1 + f_i) + f_f`
///
fn calc_interest_rate_accrual_state_changes(
    time_delta: u64,
    total_assets_amount: I80F48,
    total_liabilities_amount: I80F48,
    interest_rate_calc: &InterestRateCalc,
    asset_share_value: I80F48,
    liability_share_value: I80F48,
) -> Option<InterestRateStateChanges> {
    let utilization_rate = total_liabilities_amount.checked_div(total_assets_amount)?;
    let computed_rates = interest_rate_calc.calc_interest_rate(utilization_rate)?;

    debug!(
        "Utilization rate: {}, time delta {}s",
        utilization_rate, time_delta
    );
    debug!("{:#?}", computed_rates);

    let ComputedInterestRates {
        lending_rate_apr,
        borrowing_rate_apr,
        group_fee_apr,
        insurance_fee_apr,
        protocol_fee_apr,
    } = computed_rates;

    Some(InterestRateStateChanges {
        new_asset_share_value: calc_accrued_interest_payment_per_period(
            lending_rate_apr,
            time_delta,
            asset_share_value,
        )?,
        new_liability_share_value: calc_accrued_interest_payment_per_period(
            borrowing_rate_apr,
            time_delta,
            liability_share_value,
        )?,
        insurance_fees_collected: calc_interest_payment_for_period(
            insurance_fee_apr,
            time_delta,
            total_liabilities_amount,
        )?,
        group_fees_collected: calc_interest_payment_for_period(
            group_fee_apr,
            time_delta,
            total_liabilities_amount,
        )?,
        protocol_fees_collected: calc_interest_payment_for_period(
            protocol_fee_apr,
            time_delta,
            total_liabilities_amount,
        )?,
    })
}

struct InterestRateStateChanges {
    new_asset_share_value: I80F48,
    new_liability_share_value: I80F48,
    insurance_fees_collected: I80F48,
    group_fees_collected: I80F48,
    protocol_fees_collected: I80F48,
}

/// Calculates the fee rate for a given base rate and fees specified.
/// The returned rate is only the fee rate without the base rate.
///
/// Used for calculating the fees charged to the borrowers.
fn calc_fee_rate(base_rate: I80F48, rate_fees: I80F48, fixed_fees: I80F48) -> Option<I80F48> {
    if rate_fees.is_zero() {
        return Some(fixed_fees);
    }

    base_rate.checked_mul(rate_fees)?.checked_add(fixed_fees)
}

/// Calculates the accrued interest payment per period `time_delta` in a principal value `value` for interest rate (in APR) `arp`.
/// Result is the new principal value.
fn calc_accrued_interest_payment_per_period(
    apr: I80F48,
    time_delta: u64,
    value: I80F48,
) -> Option<I80F48> {
    let ir_per_period = apr
        .checked_mul(time_delta.into())?
        .checked_div(SECONDS_PER_YEAR)?;

    let new_value = value.checked_mul(I80F48::ONE.checked_add(ir_per_period)?)?;

    Some(new_value)
}

/// Calculates the interest payment for a given period `time_delta` in a principal value `value` for interest rate (in APR) `arp`.
/// Result is the interest payment.
fn calc_interest_payment_for_period(apr: I80F48, time_delta: u64, value: I80F48) -> Option<I80F48> {
    if apr.is_zero() {
        return Some(I80F48::ZERO);
    }

    let interest_payment = value
        .checked_mul(apr)?
        .checked_mul(time_delta.into())?
        .checked_div(SECONDS_PER_YEAR)?;

    Some(interest_payment)
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

#[repr(C)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(PartialEq, Eq, TypeLayout)
)]
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
/// TODO: Convert weights to (u64, u64) to avoid precision loss (maybe?)
pub struct BankConfigCompact {
    pub asset_weight_init: WrappedI80F48,
    pub asset_weight_maint: WrappedI80F48,

    pub liability_weight_init: WrappedI80F48,
    pub liability_weight_maint: WrappedI80F48,

    pub deposit_limit: u64,

    pub interest_rate_config: InterestRateConfigCompact,
    pub operational_state: BankOperationalState,

    pub oracle_setup: OracleSetup,
    pub oracle_key: Pubkey,

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

impl From<BankConfigCompact> for BankConfig {
    fn from(config: BankConfigCompact) -> Self {
        let keys = [
            config.oracle_key,
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
            oracle_setup: config.oracle_setup,
            oracle_keys: keys,
            _pad0: [0; 6],
            borrow_limit: config.borrow_limit,
            risk_tier: config.risk_tier,
            asset_tag: config.asset_tag,
            _pad1: [0; 6],
            total_asset_value_init_limit: config.total_asset_value_init_limit,
            oracle_max_age: config.oracle_max_age,
            _padding: [0; 38],
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
            oracle_setup: config.oracle_setup,
            oracle_key: config.oracle_keys[0],
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
#[zero_copy(unsafe)]
#[repr(C)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(PartialEq, Eq, TypeLayout)
)]
#[derive(Debug)]
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
    pub _padding: [u8; 38],
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
            _padding: [0; 38],
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
        if matches!(self.oracle_setup, OracleSetup::PythPushOracle) {
            let bytes: &[u8; 32] = self.oracle_keys[0].as_ref().try_into().unwrap();
            Some(bytes)
        } else {
            None
        }
    }
}

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
    derive(Clone, PartialEq, Eq, TypeLayout)
)]
#[derive(AnchorDeserialize, AnchorSerialize, Default)]
pub struct BankConfigOpt {
    pub asset_weight_init: Option<WrappedI80F48>,
    pub asset_weight_maint: Option<WrappedI80F48>,

    pub liability_weight_init: Option<WrappedI80F48>,
    pub liability_weight_maint: Option<WrappedI80F48>,

    pub deposit_limit: Option<u64>,
    pub borrow_limit: Option<u64>,

    pub operational_state: Option<BankOperationalState>,

    pub oracle: Option<OracleConfig>,

    pub interest_rate_config: Option<InterestRateConfigOpt>,

    pub risk_tier: Option<RiskTier>,

    pub asset_tag: Option<u8>,

    pub total_asset_value_init_limit: Option<u64>,

    pub oracle_max_age: Option<u16>,

    pub permissionless_bad_debt_settlement: Option<bool>,
}

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

#[macro_export]
macro_rules! assert_eq_with_tolerance {
    ($test_val:expr, $val:expr, $tolerance:expr) => {
        assert!(
            ($test_val - $val).abs() <= $tolerance,
            "assertion failed: `({} - {}) <= {}`",
            $test_val,
            $val,
            $tolerance
        );
    };
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::constants::{PROTOCOL_FEE_FIXED_DEFAULT, PROTOCOL_FEE_RATE_DEFAULT};

    use super::*;
    use fixed_macro::types::I80F48;

    #[test]
    /// Tests that the interest payment for a 1 year period with 100% APR is 1.
    fn interest_payment_100apr_1year() {
        let apr = I80F48::ONE;
        let time_delta = 31_536_000; // 1 year
        let value = I80F48::ONE;

        assert_eq_with_tolerance!(
            calc_interest_payment_for_period(apr, time_delta, value).unwrap(),
            I80F48::ONE,
            I80F48!(0.001)
        );
    }

    /// Tests that the interest payment for a 1 year period with 50% APR is 0.5.
    #[test]
    fn interest_payment_50apr_1year() {
        let apr = I80F48::from_num(0.5);
        let time_delta = 31_536_000; // 1 year
        let value = I80F48::ONE;

        assert_eq_with_tolerance!(
            calc_interest_payment_for_period(apr, time_delta, value).unwrap(),
            I80F48::from_num(0.5),
            I80F48!(0.001)
        );
    }
    /// P: 1_000_000
    /// Apr: 12%
    /// Time: 1 second
    #[test]
    fn interest_payment_12apr_1second() {
        let apr = I80F48!(0.12);
        let time_delta = 1;
        let value = I80F48!(1_000_000);

        assert_eq_with_tolerance!(
            calc_interest_payment_for_period(apr, time_delta, value).unwrap(),
            I80F48!(0.0038),
            I80F48!(0.001)
        );
    }

    #[test]
    /// apr: 100%
    /// time: 1 year
    /// principal: 2
    /// expected: 4
    fn accrued_interest_apr100_year1() {
        assert_eq_with_tolerance!(
            calc_accrued_interest_payment_per_period(I80F48!(1), 31_536_000, I80F48!(2)).unwrap(),
            I80F48!(4),
            I80F48!(0.001)
        );
    }

    #[test]
    /// apr: 50%
    /// time: 1 year
    /// principal: 2
    /// expected: 3
    fn accrued_interest_apr50_year1() {
        assert_eq_with_tolerance!(
            calc_accrued_interest_payment_per_period(I80F48!(0.5), 31_536_000, I80F48!(2)).unwrap(),
            I80F48!(3),
            I80F48!(0.001)
        );
    }

    #[test]
    /// apr: 12%
    /// time: 1 second
    /// principal: 1_000_000
    /// expected: 1_038
    fn accrued_interest_apr12_year1() {
        assert_eq_with_tolerance!(
            calc_accrued_interest_payment_per_period(I80F48!(0.12), 1, I80F48!(1_000_000)).unwrap(),
            I80F48!(1_000_000.0038),
            I80F48!(0.001)
        );
    }

    #[test]
    /// ur: 0
    /// protocol_fixed_fee: 0.01
    fn ir_config_calc_interest_rate_pff_01() {
        let config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.6).into(),
            plateau_interest_rate: I80F48!(0.40).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            ..Default::default()
        };

        let ComputedInterestRates {
            lending_rate_apr: lending_apr,
            borrowing_rate_apr: borrow_apr,
            group_fee_apr: group_fees_apr,
            insurance_fee_apr: insurance_apr,
            protocol_fee_apr,
        } = config
            .create_interest_rate_calculator(&MarginfiGroup::default())
            .calc_interest_rate(I80F48!(0.6))
            .unwrap();

        assert_eq_with_tolerance!(lending_apr, I80F48!(0.24), I80F48!(0.001));
        assert_eq_with_tolerance!(borrow_apr, I80F48!(0.41), I80F48!(0.001));
        assert_eq_with_tolerance!(group_fees_apr, I80F48!(0.01), I80F48!(0.001));
        assert_eq_with_tolerance!(insurance_apr, I80F48!(0), I80F48!(0.001));
        assert_eq_with_tolerance!(protocol_fee_apr, I80F48!(0), I80F48!(0.001));
    }

    #[test]
    /// ur: 0.5
    /// protocol_fixed_fee: 0.01
    /// optimal_utilization_rate: 0.5
    /// plateau_interest_rate: 0.4
    fn ir_config_calc_interest_rate_pff_01_ur_05() {
        let config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.5).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let ComputedInterestRates {
            lending_rate_apr: lending_apr,
            borrowing_rate_apr: borrow_apr,
            group_fee_apr: group_fees_apr,
            insurance_fee_apr: insurance_apr,
            protocol_fee_apr: _,
        } = config
            .create_interest_rate_calculator(&MarginfiGroup::default())
            .calc_interest_rate(I80F48!(0.5))
            .unwrap();

        assert_eq_with_tolerance!(lending_apr, I80F48!(0.2), I80F48!(0.001));
        assert_eq_with_tolerance!(borrow_apr, I80F48!(0.45), I80F48!(0.001));
        assert_eq_with_tolerance!(group_fees_apr, I80F48!(0.01), I80F48!(0.001));
        assert_eq_with_tolerance!(insurance_apr, I80F48!(0.04), I80F48!(0.001));
    }

    #[test]
    fn calc_fee_rate_1() {
        let rate = I80F48!(0.4);
        let fee_ir = I80F48!(0.05);
        let fee_fixed = I80F48!(0.01);

        assert_eq!(
            calc_fee_rate(rate, fee_ir, fee_fixed).unwrap(),
            I80F48!(0.03)
        );
    }

    /// ur: 0.8
    /// protocol_fixed_fee: 0.01
    /// optimal_utilization_rate: 0.5
    /// plateau_interest_rate: 0.4
    /// max_interest_rate: 3
    /// insurance_ir_fee: 0.1
    #[test]
    fn ir_config_calc_interest_rate_pff_01_ur_08() {
        let config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let ComputedInterestRates {
            lending_rate_apr: lending_apr,
            borrowing_rate_apr: borrow_apr,
            group_fee_apr: group_fees_apr,
            insurance_fee_apr: insurance_apr,
            protocol_fee_apr: _,
        } = config
            .create_interest_rate_calculator(&MarginfiGroup::default())
            .calc_interest_rate(I80F48!(0.7))
            .unwrap();

        assert_eq_with_tolerance!(lending_apr, I80F48!(1.19), I80F48!(0.001));
        assert_eq_with_tolerance!(borrow_apr, I80F48!(1.88), I80F48!(0.001));
        assert_eq_with_tolerance!(group_fees_apr, I80F48!(0.01), I80F48!(0.001));
        assert_eq_with_tolerance!(insurance_apr, I80F48!(0.17), I80F48!(0.001));
    }

    #[test]
    fn ir_accrual_failing_fuzz_test_example() -> anyhow::Result<()> {
        let ir_config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut bank = Bank {
            asset_share_value: I80F48::ONE.into(),
            liability_share_value: I80F48::ONE.into(),
            total_liability_shares: I80F48!(207_112_621_602).into(),
            total_asset_shares: I80F48!(10_000_000_000_000).into(),
            last_update: current_timestamp,
            config: BankConfig {
                asset_weight_init: I80F48!(0.5).into(),
                asset_weight_maint: I80F48!(0.75).into(),
                liability_weight_init: I80F48!(1.5).into(),
                liability_weight_maint: I80F48!(1.25).into(),
                borrow_limit: u64::MAX,
                deposit_limit: u64::MAX,
                interest_rate_config: ir_config,
                ..Default::default()
            },
            ..Default::default()
        };

        let pre_net_assets = bank.get_asset_amount(bank.total_asset_shares.into())?
            - bank.get_liability_amount(bank.total_liability_shares.into())?;

        let mut clock = Clock::default();

        clock.unix_timestamp = current_timestamp + 3600;

        bank.accrue_interest(
            current_timestamp,
            &MarginfiGroup::default(),
            #[cfg(not(feature = "client"))]
            Pubkey::default(),
        )
        .unwrap();

        let post_collected_fees = I80F48::from(bank.collected_group_fees_outstanding)
            + I80F48::from(bank.collected_insurance_fees_outstanding);

        let post_net_assets = bank.get_asset_amount(bank.total_asset_shares.into())?
            + post_collected_fees
            - bank.get_liability_amount(bank.total_liability_shares.into())?;

        assert_eq_with_tolerance!(pre_net_assets, post_net_assets, I80F48!(1));

        Ok(())
    }

    #[test]
    fn interest_rate_accrual_test_0() -> anyhow::Result<()> {
        let ir_config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let ur = I80F48!(207_112_621_602) / I80F48!(10_000_000_000_000);
        let mut group = MarginfiGroup::default();
        group.group_flags = 1;
        group.fee_state_cache.program_fee_fixed = PROTOCOL_FEE_FIXED_DEFAULT.into();
        group.fee_state_cache.program_fee_rate = PROTOCOL_FEE_RATE_DEFAULT.into();

        let ComputedInterestRates {
            lending_rate_apr: lending_apr,
            borrowing_rate_apr: borrow_apr,
            group_fee_apr,
            insurance_fee_apr,
            protocol_fee_apr,
        } = ir_config
            .create_interest_rate_calculator(&group)
            .calc_interest_rate(ur)
            .expect("interest rate calculation failed");

        println!("ur: {}", ur);
        println!("lending_apr: {}", lending_apr);
        println!("borrow_apr: {}", borrow_apr);
        println!("group_fee_apr: {}", group_fee_apr);
        println!("insurance_fee_apr: {}", insurance_fee_apr);

        assert_eq_with_tolerance!(
            borrow_apr,
            (lending_apr / ur) + group_fee_apr + insurance_fee_apr + protocol_fee_apr,
            I80F48!(0.001)
        );

        Ok(())
    }

    #[test]
    fn interest_rate_accrual_test_0_no_protocol_fees() -> anyhow::Result<()> {
        let ir_config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let ur = I80F48!(207_112_621_602) / I80F48!(10_000_000_000_000);

        let ComputedInterestRates {
            lending_rate_apr: lending_apr,
            borrowing_rate_apr: borrow_apr,
            group_fee_apr,
            insurance_fee_apr,
            protocol_fee_apr,
        } = ir_config
            .create_interest_rate_calculator(&MarginfiGroup::default())
            .calc_interest_rate(ur)
            .expect("interest rate calculation failed");

        println!("ur: {}", ur);
        println!("lending_apr: {}", lending_apr);
        println!("borrow_apr: {}", borrow_apr);
        println!("group_fee_apr: {}", group_fee_apr);
        println!("insurance_fee_apr: {}", insurance_fee_apr);

        assert!(protocol_fee_apr.is_zero());

        assert_eq_with_tolerance!(
            borrow_apr,
            (lending_apr / ur) + group_fee_apr + insurance_fee_apr,
            I80F48!(0.001)
        );

        Ok(())
    }

    #[test]
    fn test_accruing_interest() -> anyhow::Result<()> {
        let ir_config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let mut group = MarginfiGroup::default();
        group.group_flags = 1;
        group.fee_state_cache.program_fee_fixed = PROTOCOL_FEE_FIXED_DEFAULT.into();
        group.fee_state_cache.program_fee_rate = PROTOCOL_FEE_RATE_DEFAULT.into();

        let liab_share_value = I80F48!(1.0);
        let asset_share_value = I80F48!(1.0);

        let total_liability_shares = I80F48!(207_112_621_602);
        let total_asset_shares = I80F48!(10_000_000_000_000);

        let old_total_liability_amount = liab_share_value * total_liability_shares;
        let old_total_asset_amount = asset_share_value * total_asset_shares;

        let InterestRateStateChanges {
            new_asset_share_value,
            new_liability_share_value: new_liab_share_value,
            insurance_fees_collected: insurance_collected,
            group_fees_collected,
            protocol_fees_collected,
        } = calc_interest_rate_accrual_state_changes(
            3600,
            total_asset_shares,
            total_liability_shares,
            &ir_config.create_interest_rate_calculator(&group),
            asset_share_value,
            liab_share_value,
        )
        .unwrap();

        let new_total_liability_amount = total_liability_shares * new_liab_share_value;
        let new_total_asset_amount = total_asset_shares * new_asset_share_value;

        println!("new_asset_share_value: {}", new_asset_share_value);
        println!("new_liab_share_value: {}", new_liab_share_value);
        println!("group_fees_collected: {}", group_fees_collected);
        println!("insurance_collected: {}", insurance_collected);
        println!("protocol_fees_collected: {}", protocol_fees_collected);

        println!("new_total_liability_amount: {}", new_total_liability_amount);
        println!("new_total_asset_amount: {}", new_total_asset_amount);

        println!("old_total_liability_amount: {}", old_total_liability_amount);
        println!("old_total_asset_amount: {}", old_total_asset_amount);

        let total_fees_collected =
            group_fees_collected + insurance_collected + protocol_fees_collected;

        println!("total_fee_collected: {}", total_fees_collected);

        println!(
            "diff: {}",
            ((new_total_asset_amount - new_total_liability_amount) + total_fees_collected)
                - (old_total_asset_amount - old_total_liability_amount)
        );

        assert_eq_with_tolerance!(
            (new_total_asset_amount - new_total_liability_amount) + total_fees_collected,
            old_total_asset_amount - old_total_liability_amount,
            I80F48::ONE
        );

        Ok(())
    }
}
