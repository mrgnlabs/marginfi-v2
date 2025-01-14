use crate::{
    assert_struct_size, check,
    constants::SECONDS_PER_YEAR,
    debug,
    prelude::MarginfiError,
    set_if_some,
    state::marginfi_group::{MarginfiGroup, WrappedI80F48},
    MarginfiResult,
};
use anchor_lang::prelude::borsh;
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use std::fmt::Debug;

#[cfg(any(feature = "test", feature = "client"))]
use type_layout::TypeLayout;

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
        set_if_some!(
            self.protocol_origination_fee,
            ir_config.protocol_origination_fee
        );
    }
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
pub fn calc_interest_rate_accrual_state_changes(
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

pub struct InterestRateStateChanges {
    pub new_asset_share_value: I80F48,
    pub new_liability_share_value: I80F48,
    pub insurance_fees_collected: I80F48,
    pub group_fees_collected: I80F48,
    pub protocol_fees_collected: I80F48,
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
pub fn calc_accrued_interest_payment_per_period(
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
pub fn calc_interest_payment_for_period(
    apr: I80F48,
    time_delta: u64,
    value: I80F48,
) -> Option<I80F48> {
    if apr.is_zero() {
        return Some(I80F48::ZERO);
    }

    let interest_payment = value
        .checked_mul(apr)?
        .checked_mul(time_delta.into())?
        .checked_div(SECONDS_PER_YEAR)?;

    Some(interest_payment)
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

    use crate::{
        constants::{PROTOCOL_FEE_FIXED_DEFAULT, PROTOCOL_FEE_RATE_DEFAULT},
        state::bank::{Bank, BankConfig},
    };

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
