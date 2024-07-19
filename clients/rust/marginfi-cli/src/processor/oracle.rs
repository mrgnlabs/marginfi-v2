use pyth_solana_receiver_sdk::price_update::FeedId;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::account_info::IntoAccountInfo;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn find_pyth_push_oracles_for_feed_id(
    rpc_client: &RpcClient,
    feed_id: FeedId,
) -> anyhow::Result<()> {
    let mut res = rpc_client.get_program_accounts_with_config(
        &pyth_solana_receiver_sdk::ID,
        RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                8 + 32 + 1,
                feed_id.to_vec(),
            ))]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..Default::default()
            },
            ..Default::default()
        },
    )?;

    println!("Found {} price feeds", res.len());

    for (ref address, account) in res.iter_mut() {
        let ai = (address, account).into_account_info();
        let price_update_v2 = marginfi::state::price::load_price_update_v2_checked(&ai)?;

        let feed_id = &price_update_v2.price_message.feed_id;
        let current_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let age_secs = current_timestamp - price_update_v2.price_message.publish_time;
        let verification_level = price_update_v2.verification_level;

        let feed_id_hex = hex::encode(feed_id);
        println!(
            "Found pyth account {}, feed_id: 0x{}, min_verification_level: {:?}, age: {}s",
            address, feed_id_hex, verification_level, age_secs
        );
    }

    Ok(())
}
