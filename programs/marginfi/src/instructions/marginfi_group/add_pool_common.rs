use crate::utils::wrapped_i80f48_to_f64;
use anchor_lang::prelude::*;
use marginfi_type_crate::types::Bank;

/// Echo the information used to create banks to the log output. Useful for at-a-glance debugging
/// bank creation txes in explorer. Note: costs a lot of CU
pub fn log_pool_info(bank: &Bank) {
    let conf = bank.config;
    msg!(
        "Asset weight init: {:?} maint: {:?}",
        wrapped_i80f48_to_f64(conf.asset_weight_init),
        wrapped_i80f48_to_f64(conf.asset_weight_maint)
    );
    msg!(
        "Liab weight init: {:?} maint: {:?}",
        wrapped_i80f48_to_f64(conf.liability_weight_init),
        wrapped_i80f48_to_f64(conf.liability_weight_maint)
    );
    msg!(
        "deposit limit: {:?} borrow limit: {:?} init val limit: {:?}",
        conf.deposit_limit,
        conf.borrow_limit,
        conf.total_asset_value_init_limit
    );
    msg!(
        "op state: {:?} risk tier: {:?} asset tag: {:?}",
        conf.operational_state as u8,
        conf.risk_tier as u8,
        conf.asset_tag
    );
    msg!(
        "oracle conf {:?} age: {:?} flags: {:?}",
        conf.oracle_max_confidence,
        conf.oracle_max_age as u8,
        bank.flags as u8
    );
    let interest = conf.interest_rate_config;
    msg!(
        "Insurance fixed: {:?} ir: {:?}",
        wrapped_i80f48_to_f64(interest.insurance_fee_fixed_apr),
        wrapped_i80f48_to_f64(interest.insurance_ir_fee)
    );
    msg!(
        "Group fixed: {:?} ir: {:?} origination: {:?}",
        wrapped_i80f48_to_f64(interest.protocol_fixed_fee_apr),
        wrapped_i80f48_to_f64(interest.protocol_ir_fee),
        wrapped_i80f48_to_f64(interest.protocol_origination_fee)
    );
    msg!(
        "Init: {:?} points: {:?}: max: {:?}",
        interest.zero_util_rate,
        // TODO validate this pretty-prints in a readable way or add a debug impl
        interest.points,
        interest.max_interest_rate
    );
}
