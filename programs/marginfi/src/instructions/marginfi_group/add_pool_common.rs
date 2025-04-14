use crate::state::marginfi_group::Bank;
use anchor_lang::prelude::*;
use fixed::types::I80F48;

/// Echo the information used to create banks to the log output. Useful for at-a-glance debugging
/// bank creation txes in explorer. Note: costs a lot of CU
pub fn log_pool_info(bank: &Bank) {
    let conf = bank.config;
    let asset_weight_init: I80F48 = conf.asset_weight_init.into();
    let asset_weight_init_f64: f64 = asset_weight_init.to_num();
    let asset_weight_maint: I80F48 = conf.asset_weight_maint.into();
    let asset_weight_maint_f64: f64 = asset_weight_maint.to_num();
    msg!(
        "Asset weight init: {:?} maint: {:?}",
        asset_weight_init_f64,
        asset_weight_maint_f64
    );
    let liab_weight_init: I80F48 = conf.liability_weight_init.into();
    let liab_weight_init_f64: f64 = liab_weight_init.to_num();
    let liab_weight_maint: I80F48 = conf.liability_weight_maint.into();
    let liab_weight_maint_f64: f64 = liab_weight_maint.to_num();
    msg!(
        "Liab weight init: {:?} maint: {:?}",
        liab_weight_init_f64,
        liab_weight_maint_f64
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
        "oracle age: {:?} flags: {:?}",
        conf.oracle_max_age as u8,
        bank.flags as u8
    );
}
