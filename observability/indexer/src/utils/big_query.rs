use gcp_bigquery_client::model::{table_field_schema::TableFieldSchema, table_schema::TableSchema};
use lazy_static::lazy_static;

pub const NOT_FOUND_CODE: i64 = 404;
pub const DATE_FORMAT_STR: &str = "%Y-%m-%d %H:%M:%S";

lazy_static! {
    pub static ref TRANSACTION_SCHEMA: TableSchema = TableSchema::new(vec![
        TableFieldSchema::string("id"),
        TableFieldSchema::timestamp("created_at"),
        TableFieldSchema::timestamp("timestamp"),
        TableFieldSchema::string("signature"),
        TableFieldSchema::string("indexing_address"),
        TableFieldSchema::big_numeric("slot"),
        TableFieldSchema::string("signer"),
        TableFieldSchema::bool("success"),
        TableFieldSchema::string("version"),
        TableFieldSchema::big_numeric("fee"),
        TableFieldSchema::string("meta"),
        TableFieldSchema::string("message"),
    ]);
    pub static ref ACCOUNT_SCHEMA: TableSchema = TableSchema::new(vec![
        TableFieldSchema::string("id"),
        TableFieldSchema::timestamp("created_at"),
        TableFieldSchema::timestamp("timestamp"),
        TableFieldSchema::string("owner"),
        TableFieldSchema::big_numeric("slot"),
        TableFieldSchema::string("pubkey"),
        TableFieldSchema::string("txn_signature"),
        TableFieldSchema::big_numeric("write_version"),
        TableFieldSchema::big_numeric("lamports"),
        TableFieldSchema::bool("executable"),
        TableFieldSchema::big_numeric("rent_epoch"),
        TableFieldSchema::string("data"),
    ]);
}

lazy_static! {
    pub static ref METRIC_MARGINFI_GROUP_SCHEMA: TableSchema = TableSchema::new(vec![
        TableFieldSchema::string("id"),
        TableFieldSchema::timestamp("created_at"),
        TableFieldSchema::timestamp("timestamp"),
        TableFieldSchema::string("pubkey"),
        TableFieldSchema::integer("marginfi_accounts_count"),
        TableFieldSchema::integer("banks_count"),
        TableFieldSchema::integer("mints_count"),
        TableFieldSchema::float("total_assets_in_usd"),
        TableFieldSchema::float("total_liabilities_in_usd"),
    ]);
    pub static ref METRIC_LENDING_POOL_BANK_SCHEMA: TableSchema = TableSchema::new(vec![
        TableFieldSchema::string("id"),
        TableFieldSchema::timestamp("created_at"),
        TableFieldSchema::timestamp("timestamp"),
        TableFieldSchema::string("pubkey"),
        TableFieldSchema::string("marginfi_group"),
        TableFieldSchema::string("mint"),
        TableFieldSchema::float("usd_price"),
        TableFieldSchema::string("operational_state"),
        TableFieldSchema::float("asset_weight_maintenance"),
        TableFieldSchema::float("liability_weight_maintenance"),
        TableFieldSchema::float("asset_weight_initial"),
        TableFieldSchema::float("liability_weight_initial"),
        TableFieldSchema::float("deposit_limit_in_tokens"),
        TableFieldSchema::float("borrow_limit_in_tokens"),
        TableFieldSchema::float("deposit_limit_in_usd"),
        TableFieldSchema::float("borrow_limit_in_usd"),
        TableFieldSchema::integer("lenders_count"),
        TableFieldSchema::integer("borrowers_count"),
        TableFieldSchema::float("deposit_rate"),
        TableFieldSchema::float("borrow_rate"),
        TableFieldSchema::float("group_fee"),
        TableFieldSchema::float("insurance_fee"),
        TableFieldSchema::float("total_assets_in_tokens"),
        TableFieldSchema::float("total_liabilities_in_tokens"),
        TableFieldSchema::float("total_assets_in_usd"),
        TableFieldSchema::float("total_liabilities_in_usd"),
        TableFieldSchema::float("liquidity_vault_balance"),
        TableFieldSchema::float("insurance_vault_balance"),
        TableFieldSchema::float("fee_vault_balance"),
    ]);
    pub static ref METRIC_MARGINFI_ACCOUNT_SCHEMA: TableSchema = TableSchema::new(vec![
        TableFieldSchema::string("id"),
        TableFieldSchema::timestamp("created_at"),
        TableFieldSchema::timestamp("timestamp"),
        TableFieldSchema::string("pubkey"),
        TableFieldSchema::string("marginfi_group"),
        TableFieldSchema::string("owner"),
        TableFieldSchema::float("total_assets_in_usd"),
        TableFieldSchema::float("total_liabilities_in_usd"),
        TableFieldSchema::float("total_assets_in_usd_maintenance"),
        TableFieldSchema::float("total_liabilities_in_usd_maintenance"),
        TableFieldSchema::float("total_assets_in_usd_initial"),
        TableFieldSchema::float("total_liabilities_in_usd_initial"),
        TableFieldSchema::string("positions"),
    ]);
}
