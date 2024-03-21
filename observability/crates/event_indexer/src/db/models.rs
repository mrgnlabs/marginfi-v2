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

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = accounts)]
pub struct Accounts {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub address: String,
    pub user_id: i32,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable, Associations)]
#[diesel(belongs_to(Accounts, foreign_key = account_id), belongs_to(Users, foreign_key = authority_id))]
#[diesel(table_name = create_account_events)]
pub struct CreateAccountEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
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
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,
    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub emission_mint_id: i32,
    pub amount: Decimal,
}
