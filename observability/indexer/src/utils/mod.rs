use marginfi::constants::{ASSET_TAG_DRIFT, DRIFT_SCALED_BALANCE_DECIMALS};
use marginfi::state::marginfi_group::Bank;
use solana_sdk::{account::Account, pubkey::Pubkey};

pub mod big_query;
pub mod crossbar;
pub mod errors;
pub mod marginfi_account_dup;
pub mod metrics;
pub mod protos;
pub mod snapshot;
pub mod swb_pull;
pub mod transactions_crawler;

pub fn get_balance_decimals(bank: &Bank) -> u8 {
    if bank.config.asset_tag == ASSET_TAG_DRIFT {
        DRIFT_SCALED_BALANCE_DECIMALS
    } else {
        bank.mint_decimals
    }
}

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
