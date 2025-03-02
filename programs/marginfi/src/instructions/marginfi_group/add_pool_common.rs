use crate::state::marginfi_group::Bank;
use anchor_lang::prelude::*;

/// Echo the information used to create banks to the log output. Useful for at-a-glance debugging
/// bank creation txes in explorer. Note: costs a lot of CU
pub fn log_pool_info(bank: &Bank) {
    let conf = bank.config;
    let asset_weight_init = u128::from_le_bytes(conf.asset_weight_init.value);
    let asset_weight_maint = u128::from_le_bytes(bank.config.asset_weight_maint.value);
    msg!(
        "Asset weight init: {:?} maint: {:?}",
        asset_weight_init,
        asset_weight_maint
    );
    let liab_weight_init = u128::from_le_bytes(conf.liability_weight_init.value);
    let liab_weight_maint = u128::from_le_bytes(conf.liability_weight_maint.value);
    msg!(
        "Liab weight init: {:?} maint: {:?}",
        liab_weight_init,
        liab_weight_maint
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
