use gcp_bigquery_client::model::{table_field_schema::TableFieldSchema, table_schema::TableSchema};
use lazy_static::lazy_static;
use serde::Serialize;

pub const NOT_FOUND_CODE: i64 = 404;

lazy_static! {
    pub static ref TRANSACTION_SCHEMA: TableSchema = TableSchema::new(vec![
        TableFieldSchema::string("id"),
        TableFieldSchema::timestamp("created_at"),
        TableFieldSchema::timestamp("timestamp"),
        TableFieldSchema::string("signature"),
        TableFieldSchema::big_numeric("slot"),
        TableFieldSchema::string("signer"),
        TableFieldSchema::bool("success"),
        TableFieldSchema::string("version"),
        TableFieldSchema::big_numeric("fee"),
        TableFieldSchema::string("meta"),
        TableFieldSchema::string("message"),
    ]);
}

pub const DATE_FORMAT_STR: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug, Serialize)]
pub struct TxRow {
    pub id: String,
    pub created_at: String,
    pub timestamp: String,
    pub signature: String,
    pub slot: u64,
    pub signer: String,
    pub success: bool,
    pub version: Option<String>,
    pub fee: Option<u64>,
    pub meta: Option<String>,
    pub message: String,
}
