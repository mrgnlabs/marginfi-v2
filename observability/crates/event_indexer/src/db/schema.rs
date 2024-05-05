// @generated automatically by Diesel CLI.

diesel::table! {
    accounts (id) {
        id -> Int4,
        address -> Varchar,
        user_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    bank_operational_state (id) {
        id -> Int4,
        name -> Varchar,
    }
}

diesel::table! {
    banks (id) {
        id -> Int4,
        address -> Varchar,
        mint_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    borrow_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        account_id -> Int4,
        authority_id -> Int4,
        bank_id -> Int4,
        amount -> Numeric,
        price -> Nullable<Numeric>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    configure_bank_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        bank_id -> Int4,
        asset_weight_init -> Nullable<Numeric>,
        asset_weight_maint -> Nullable<Numeric>,
        liability_weight_init -> Nullable<Numeric>,
        liability_weight_maint -> Nullable<Numeric>,
        deposit_limit -> Nullable<Numeric>,
        borrow_limit -> Nullable<Numeric>,
        operational_state_id -> Nullable<Int4>,
        oracle_setup_id -> Nullable<Int4>,
        oracle_keys -> Nullable<Varchar>,
        optimal_utilization_rate -> Nullable<Numeric>,
        plateau_interest_rate -> Nullable<Numeric>,
        max_interest_rate -> Nullable<Numeric>,
        insurance_fee_fixed_apr -> Nullable<Numeric>,
        insurance_ir_fee -> Nullable<Numeric>,
        protocol_fixed_fee_apr -> Nullable<Numeric>,
        protocol_ir_fee -> Nullable<Numeric>,
        risk_tier_id -> Nullable<Int4>,
        total_asset_value_init_limit -> Nullable<Numeric>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        oracle_max_age -> Int4,
    }
}

diesel::table! {
    create_account_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        account_id -> Int4,
        authority_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    create_bank_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        bank_id -> Int4,
        asset_weight_init -> Numeric,
        asset_weight_maint -> Numeric,
        liability_weight_init -> Numeric,
        liability_weight_maint -> Numeric,
        deposit_limit -> Numeric,
        optimal_utilization_rate -> Numeric,
        plateau_interest_rate -> Numeric,
        max_interest_rate -> Numeric,
        insurance_fee_fixed_apr -> Numeric,
        insurance_ir_fee -> Numeric,
        protocol_fixed_fee_apr -> Numeric,
        protocol_ir_fee -> Numeric,
        operational_state_id -> Int4,
        oracle_setup_id -> Int4,
        oracle_keys -> Varchar,
        borrow_limit -> Numeric,
        risk_tier_id -> Int4,
        total_asset_value_init_limit -> Numeric,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        oracle_max_age -> Int4,
    }
}

diesel::table! {
    deposit_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        account_id -> Int4,
        authority_id -> Int4,
        bank_id -> Int4,
        amount -> Numeric,
        price -> Nullable<Numeric>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    liquidate_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        liquidator_account_id -> Int4,
        liquidatee_account_id -> Int4,
        liquidator_user_id -> Int4,
        asset_bank_id -> Int4,
        liability_bank_id -> Int4,
        asset_amount -> Numeric,
        asset_price -> Nullable<Numeric>,
        liability_price -> Nullable<Numeric>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    mints (id) {
        id -> Int4,
        address -> Varchar,
        symbol -> Varchar,
        decimals -> Int2,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    oracle_setup (id) {
        id -> Int4,
        name -> Varchar,
    }
}

diesel::table! {
    repay_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        account_id -> Int4,
        authority_id -> Int4,
        bank_id -> Int4,
        amount -> Numeric,
        price -> Nullable<Numeric>,
        all -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    risk_tier (id) {
        id -> Int4,
        name -> Varchar,
    }
}

diesel::table! {
    transfer_account_authority_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        account_id -> Int4,
        old_authority_id -> Int4,
        new_authority_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    unknown_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        address -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    withdraw_emissions_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        account_id -> Int4,
        authority_id -> Int4,
        bank_id -> Int4,
        emission_mint_id -> Int4,
        amount -> Numeric,
        price -> Nullable<Numeric>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    withdraw_events (id) {
        id -> Int4,
        timestamp -> Timestamp,
        slot -> Numeric,
        tx_sig -> Varchar,
        in_flashloan -> Bool,
        call_stack -> Varchar,
        account_id -> Int4,
        authority_id -> Int4,
        bank_id -> Int4,
        amount -> Numeric,
        price -> Nullable<Numeric>,
        all -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(accounts -> users (user_id));
diesel::joinable!(banks -> mints (mint_id));
diesel::joinable!(borrow_events -> accounts (account_id));
diesel::joinable!(borrow_events -> banks (bank_id));
diesel::joinable!(borrow_events -> users (authority_id));
diesel::joinable!(configure_bank_events -> bank_operational_state (operational_state_id));
diesel::joinable!(configure_bank_events -> banks (bank_id));
diesel::joinable!(configure_bank_events -> oracle_setup (oracle_setup_id));
diesel::joinable!(configure_bank_events -> risk_tier (risk_tier_id));
diesel::joinable!(create_account_events -> accounts (account_id));
diesel::joinable!(create_account_events -> users (authority_id));
diesel::joinable!(create_bank_events -> bank_operational_state (operational_state_id));
diesel::joinable!(create_bank_events -> banks (bank_id));
diesel::joinable!(create_bank_events -> oracle_setup (oracle_setup_id));
diesel::joinable!(create_bank_events -> risk_tier (risk_tier_id));
diesel::joinable!(deposit_events -> accounts (account_id));
diesel::joinable!(deposit_events -> banks (bank_id));
diesel::joinable!(deposit_events -> users (authority_id));
diesel::joinable!(liquidate_events -> users (liquidator_user_id));
diesel::joinable!(repay_events -> accounts (account_id));
diesel::joinable!(repay_events -> banks (bank_id));
diesel::joinable!(repay_events -> users (authority_id));
diesel::joinable!(transfer_account_authority_events -> accounts (account_id));
diesel::joinable!(withdraw_emissions_events -> accounts (account_id));
diesel::joinable!(withdraw_emissions_events -> banks (bank_id));
diesel::joinable!(withdraw_emissions_events -> mints (emission_mint_id));
diesel::joinable!(withdraw_emissions_events -> users (authority_id));
diesel::joinable!(withdraw_events -> accounts (account_id));
diesel::joinable!(withdraw_events -> banks (bank_id));
diesel::joinable!(withdraw_events -> users (authority_id));

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    bank_operational_state,
    banks,
    borrow_events,
    configure_bank_events,
    create_account_events,
    create_bank_events,
    deposit_events,
    liquidate_events,
    mints,
    oracle_setup,
    repay_events,
    risk_tier,
    transfer_account_authority_events,
    unknown_events,
    users,
    withdraw_emissions_events,
    withdraw_events,
);
