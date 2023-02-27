use gcp_bigquery_client::model::{table_field_schema::TableFieldSchema, table_schema::TableSchema};
use lazy_static::lazy_static;

pub const NOT_FOUND_CODE: i64 = 404;

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
        TableFieldSchema::big_numeric("lamports"),
        TableFieldSchema::bool("executable"),
        TableFieldSchema::big_numeric("rent_epoch"),
        TableFieldSchema::string("data"),
    ]);
}

pub const DATE_FORMAT_STR: &str = "%Y-%m-%d %H:%M:%S";
