use solana_sdk::{account::Account, pubkey::Pubkey};

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
