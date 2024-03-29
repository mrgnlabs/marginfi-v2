use std::time::Duration;

use futures::{stream, StreamExt};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{account::Account, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use backoff::{future::retry, ExponentialBackoffBuilder};

pub mod big_query;
pub mod errors;
pub mod marginfi_account_dup;
pub mod metrics;
pub mod snapshot;
pub mod transactions_crawler;

pub fn convert_account(
    account_update: yellowstone_grpc_proto::geyser::SubscribeUpdateAccountInfo,
) -> Result<Account, String> {
    Ok(Account {
        lamports: account_update.lamports,
        data: account_update.data,
        owner: Pubkey::try_from(account_update.owner).unwrap(),
        executable: account_update.executable,
        rent_epoch: account_update.rent_epoch,
    })
  }
  
pub async fn get_multiple_transactions(
  rpc_client: &RpcClient,
  signatures: &[Signature],
) -> Result<Vec<(Signature, EncodedConfirmedTransactionWithStatusMeta)>, String>
{
  Ok(stream::iter(signatures)
      .map(|signature| async move {
          (
              *signature,
              retry(
                  ExponentialBackoffBuilder::new()
                      .with_max_interval(Duration::from_secs(5))
                      .build(),
                  || async {
                      Ok(rpc_client
                          .get_transaction_with_config(
                              signature,
                              RpcTransactionConfig {
                                  max_supported_transaction_version: Some(0),
                                  encoding: Some(UiTransactionEncoding::Base64),
                                  ..Default::default()
                              },
                          )
                          .await?)
                  },
              )
              .await
              .map_err(|_| "Failed to get transaction".to_string())
              .unwrap(),
          )
      })
      .buffered(10) // Higher ingest if unordered, but no way to order txs in same slot a posteriori in that case
      .collect::<Vec<_>>()
      .await)
}
