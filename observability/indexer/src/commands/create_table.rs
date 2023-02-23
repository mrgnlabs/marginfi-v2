use std::str::FromStr;
use anyhow::{anyhow, Result};
use gcp_bigquery_client::{error::BQError, model::{table::Table, time_partitioning::TimePartitioning}};
use log::{info, warn};
use yup_oauth2::parse_service_account_key;

use crate::utils::big_query::{ACCOUNT_SCHEMA, NOT_FOUND_CODE, TRANSACTION_SCHEMA};

#[derive(Debug)]
pub enum TableType  {
    Transaction,
    Account,
}

impl FromStr for TableType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "transaction" => Ok(Self::Transaction),
            "account" => Ok(Self::Account),
            _ => Err(anyhow!("Invalid table type")),
        }
    }}

pub async fn create_table(
    project_id: String,
    dataset_id: String,
    table_id: String,
    table_type: TableType,
    table_friendly_name: Option<String>,
            table_description: Option<String>,
        ) -> Result<()> {
        // Init BigQuery client
        let sa_key = parse_service_account_key(std::env::var("GOOGLE_APPLICATION_CREDENTIALS_JSON").unwrap()).unwrap();
        let client = gcp_bigquery_client::Client::from_service_account_key(sa_key, false)
        .await
        .unwrap();

        let schema = match table_type {
        TableType::Transaction => TRANSACTION_SCHEMA.to_owned(),
        TableType::Account => ACCOUNT_SCHEMA.to_owned(),
    };

    // Create a new table if needed
    match client
        .table()
        .get(
            &project_id,
            &dataset_id,
            &table_id,
            None,
        )
        .await
    {
        Ok(_) => info!("Table {} already exists", table_id),
        Err(error) => match error {
            BQError::ResponseError { error } if error.error.code == NOT_FOUND_CODE => {
                warn!("Table {} not found, creating", table_id);
                match client
                    .table()
                    .create(
                        Table::new(
                            &project_id,
                            &dataset_id,
                            &table_id,
                            schema,
                        )
                        .friendly_name(&table_friendly_name.unwrap_or_default())
                        .description(&table_description.unwrap_or_default())
                        .time_partitioning(TimePartitioning::per_day().field("timestamp")),
                    )
                    .await {
                    Ok(_) => info!("Table {} created", table_id),
                    Err(error) => panic!("Error creating table {}: {:#?}", table_id, error),
                };
            }
            _ => panic!("Error fetching table {}: {:#?}", table_id, error),
        },
    };

    Ok(())
}
