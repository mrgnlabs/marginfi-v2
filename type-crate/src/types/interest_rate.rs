use crate::assert_struct_size;

#[cfg(feature = "anchor")]
use anchor_lang::prelude::*;

use bytemuck::{Pod, Zeroable};

use super::WrappedI80F48;

assert_struct_size!(InterestRateConfig, 240);
#[repr(C)]
#[cfg_attr(feature = "anchor", derive(AnchorDeserialize, AnchorSerialize))]
#[derive(Debug, PartialEq, Eq, Copy, Clone, Pod, Zeroable, Default)]
pub struct InterestRateConfig {
    // TODO deprecate
    pub optimal_utilization_rate: WrappedI80F48,
    // TODO deprecate
    pub plateau_interest_rate: WrappedI80F48,
    // TODO deprecate
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

    /// The base rate at utilizatation = 0
    /// * a %, as u32, out of 1000%, e.g. 100% = 0.1 * u32::MAX
    pub zero_util_rate: u32,
    /// The base rate at utilizatation = 100
    /// * a %, as u32, out of 1000%, e.g. 100% = 0.1 * u32::MAX
    pub hundred_util_rate: u32,
    /// The base rate at various points between 0 and 100%, exclusive. Essentially a piece-wise
    /// linear curve.
    /// * always in ascending order, e.g. points[0] = first kink point, points[1] = second kink
    ///   point, and so forth.
    /// * points where util = 0 are unused
    pub points: [RatePoint; 5],

    pub _padding0: [u8; 8],
    pub _padding1: [[u8; 32]; 2],
}

#[cfg_attr(feature = "anchor", derive(AnchorDeserialize, AnchorSerialize))]
#[derive(Clone, Copy, Default, Zeroable, Pod, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct RatePoint {
    /// The base rate that applies
    /// * a %, as u32, out of 1000%, e.g. 100% = 0.1 * u32::MAX
    rate: u32,
    /// The utilization rate where `rate` applies
    /// * a %, as u32, out of 100%, e.g. 50% = .5 * u32::MAX
    util: u32,
}

#[cfg_attr(feature = "anchor", derive(AnchorDeserialize, AnchorSerialize))]
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct InterestRateConfigOpt {
    // TODO deprecate
    pub optimal_utilization_rate: Option<WrappedI80F48>,
    // TODO deprecate
    pub plateau_interest_rate: Option<WrappedI80F48>,
    // TODO deprecate
    pub max_interest_rate: Option<WrappedI80F48>,

    pub insurance_fee_fixed_apr: Option<WrappedI80F48>,
    pub insurance_ir_fee: Option<WrappedI80F48>,
    pub protocol_fixed_fee_apr: Option<WrappedI80F48>,
    pub protocol_ir_fee: Option<WrappedI80F48>,
    pub protocol_origination_fee: Option<WrappedI80F48>,
    /// The base rate at utilizatation = 0
    /// * a %, as u32, out of 1000%, e.g. 100% = 0.1 * u32::MAX
    pub zero_util_rate: Option<u32>,
    /// The base rate at utilizatation = 100
    /// * a %, as u32, out of 1000%, e.g. 100% = 0.1 * u32::MAX
    pub hundred_util_rate: Option<u32>,
    /// The base rate at various points between 0 and 100%, exclusive. Essentially a piece-wise
    /// linear curve.
    /// * always in ascending order, e.g. points[0] = first kink point, points[1] = second kink
    ///   point, and so forth.
    /// * points where util = 0 are unused
    pub points: Option<[RatePoint; 5]>,
}

#[repr(C)]
#[cfg_attr(feature = "anchor", derive(AnchorDeserialize, AnchorSerialize))]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct InterestRateConfigCompact {
    // TODO deprecate
    pub optimal_utilization_rate: WrappedI80F48,
    // TODO deprecate
    pub plateau_interest_rate: WrappedI80F48,
    // TODO deprecate
    pub max_interest_rate: WrappedI80F48,

    // Fees
    pub insurance_fee_fixed_apr: WrappedI80F48,
    pub insurance_ir_fee: WrappedI80F48,
    pub protocol_fixed_fee_apr: WrappedI80F48,
    pub protocol_ir_fee: WrappedI80F48,
    pub protocol_origination_fee: WrappedI80F48,

    /// The base rate at utilizatation = 0
    /// * a %, as u32, out of 1000%, e.g. 100% = 0.1 * u32::MAX
    pub zero_util_rate: u32,
    /// The base rate at utilizatation = 100
    /// * a %, as u32, out of 1000%, e.g. 100% = 0.1 * u32::MAX
    pub hundred_util_rate: u32,
    /// The base rate at various points between 0 and 100%, exclusive. Essentially a piece-wise
    /// linear curve.
    /// * always in ascending order, e.g. points[0] = first kink point, points[1] = second kink
    ///   point, and so forth.
    /// * points where util = 0 are unused
    pub points: [RatePoint; 5],
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
            zero_util_rate: ir_config.zero_util_rate,
            hundred_util_rate: ir_config.hundred_util_rate,
            points: ir_config.points,
            _padding0: [0; 8],
            _padding1: [[0; 32]; 2],
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
            zero_util_rate: ir_config.zero_util_rate,
            hundred_util_rate: ir_config.hundred_util_rate,
            points: ir_config.points,
        }
    }
}
