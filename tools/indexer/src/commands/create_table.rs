use anyhow::Result;
use envconfig::Envconfig;
use gcp_bigquery_client::{
    error::BQError,
    model::{table::Table, time_partitioning::TimePartitioning},
};
use log::{info, warn};
use yup_oauth2::parse_service_account_key;

use crate::utils::big_query::{NOT_FOUND_CODE, TRANSACTION_SCHEMA};

#[derive(Envconfig, Debug, Clone)]
pub struct CreateTableConfig {
    pub project_id: String,
    pub dataset_id: String,
    pub table_id: String,
    pub table_friendly_name: Option<String>,
    pub table_description: Option<String>,
    pub gcp_sa_key: String,
}

pub async fn create_table(config: CreateTableConfig) -> Result<()> {
    // Init BigQuery client
    let sa_key = parse_service_account_key(config.gcp_sa_key).unwrap();
    let client = gcp_bigquery_client::Client::from_service_account_key(sa_key, false)
        .await
        .unwrap();

    // Create a new table if needed
    match client
        .table()
        .get(
            &config.project_id,
            &config.dataset_id,
            &config.table_id,
            None,
        )
        .await
    {
        Ok(_) => info!("Table {} already exists", config.table_id),
        Err(error) => match error {
            BQError::ResponseError { error } if error.error.code == NOT_FOUND_CODE => {
                warn!("Table {} not found, creating", config.table_id);
                client
                    .table()
                    .create(
                        Table::new(
                            &config.project_id,
                            &config.dataset_id,
                            &config.table_id,
                            TRANSACTION_SCHEMA.to_owned(),
                        )
                        .friendly_name(&config.table_friendly_name.unwrap_or_default())
                        .description(&config.table_description.unwrap_or_default())
                        .time_partitioning(TimePartitioning::per_day().field("timestamp")),
                    )
                    .await
                    .unwrap();
                info!("Table {} created", config.table_id);
            }
            _ => panic!("Error fetching table {}: {}", config.table_id, error),
        },
    };

    Ok(())
}
