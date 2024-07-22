use solana_sdk::{account::Account, pubkey::Pubkey};
use yellowstone_grpc_client::{GeyserGrpcBuilder, GeyserGrpcClient, Interceptor};

pub mod big_query;
pub mod errors;
pub mod marginfi_account_dup;
pub mod metrics;
pub mod protos;
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

pub async fn create_geyser_client(
    rpc_url: &str,
    rpc_token: &str,
) -> anyhow::Result<GeyserGrpcClient<impl Interceptor>> {
    let geyser_client_connection_result = GeyserGrpcBuilder::from_shared(rpc_url.to_string())?
        .x_token(Some(rpc_token.to_string()))?
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(10))
        .connect()
        .await?;

    Ok(geyser_client_connection_result)
}
