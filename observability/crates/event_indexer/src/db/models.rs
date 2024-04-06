use crate::db::schema::*;
use diesel::prelude::*;
use rust_decimal::Decimal;

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = mints)]
pub struct Mints {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub address: String,
    pub symbol: String,
    pub decimals: i16,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(belongs_to(Mints, foreign_key = mint_id))]
#[diesel(table_name = banks)]
pub struct Banks {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub address: String,
    pub mint_id: i32,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = users)]
pub struct Users {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub address: String,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable, Associations)]
#[diesel(table_name = accounts)]
#[diesel(belongs_to(Users, foreign_key = user_id))]
pub struct Accounts {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub address: String,
    pub user_id: i32,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = unknown_events)]
pub struct UnknownEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable, Associations)]
#[diesel(table_name = create_account_events)]
#[diesel(belongs_to(Accounts, foreign_key = account_id), belongs_to(Users, foreign_key = authority_id))]
pub struct CreateAccountEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = transfer_account_authority_events)]
pub struct TransferAccountAuthorityEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub old_authority_id: i32,
    pub new_authority_id: i32,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = deposit_events)]
pub struct DepositEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub amount: Decimal,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = borrow_events)]
pub struct BorrowEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub amount: Decimal,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = repay_events)]
pub struct RepayEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub amount: Decimal,
    pub all: bool,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = withdraw_events)]
pub struct WithdrawEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub amount: Decimal,
    pub all: bool,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = withdraw_emissions_events)]
pub struct WithdrawEmissionsEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub emission_mint_id: i32,
    pub amount: Decimal,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = liquidate_events)]
pub struct LiquidateEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub liquidator_account_id: i32,
    pub liquidatee_account_id: i32,
    pub liquidator_user_id: i32,
    pub asset_bank_id: i32,
    pub liability_bank_id: i32,
    pub asset_amount: Decimal,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = create_bank_events)]
pub struct CreateBankEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub bank_id: i32,
    pub asset_weight_init: Decimal,
    pub asset_weight_maint: Decimal,
    pub liability_weight_init: Decimal,
    pub liability_weight_maint: Decimal,
    pub deposit_limit: Decimal,
    pub optimal_utilization_rate: Decimal,
    pub plateau_interest_rate: Decimal,
    pub max_interest_rate: Decimal,
    pub insurance_fee_fixed_apr: Decimal,
    pub insurance_ir_fee: Decimal,
    pub protocol_fixed_fee_apr: Decimal,
    pub protocol_ir_fee: Decimal,
    pub operational_state_id: i32,
    pub oracle_setup_id: i32,
    pub oracle_keys: String,
    pub borrow_limit: Decimal,
    pub risk_tier_id: i32,
    pub total_asset_value_init_limit: Decimal,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = configure_bank_events)]
pub struct ConfigureBankEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub bank_id: i32,
    pub asset_weight_init: Option<Decimal>,
    pub asset_weight_maint: Option<Decimal>,
    pub liability_weight_init: Option<Decimal>,
    pub liability_weight_maint: Option<Decimal>,
    pub deposit_limit: Option<Decimal>,
    pub optimal_utilization_rate: Option<Decimal>,
    pub plateau_interest_rate: Option<Decimal>,
    pub max_interest_rate: Option<Decimal>,
    pub insurance_fee_fixed_apr: Option<Decimal>,
    pub insurance_ir_fee: Option<Decimal>,
    pub protocol_fixed_fee_apr: Option<Decimal>,
    pub protocol_ir_fee: Option<Decimal>,
    pub operational_state_id: Option<i32>,
    pub oracle_setup_id: Option<i32>,
    pub oracle_keys: Option<String>,
    pub borrow_limit: Option<Decimal>,
    pub risk_tier_id: Option<i32>,
    pub total_asset_value_init_limit: Option<Decimal>,
}
