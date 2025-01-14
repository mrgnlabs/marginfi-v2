use std::{
    collections::HashMap,
    mem::size_of,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use log::{error, info, warn};
use marginfi::{
    constants::{PYTH_PUSH_MARGINFI_SPONSORED_SHARD_ID, PYTH_PUSH_PYTH_SPONSORED_SHARD_ID},
    state::{bank::Bank, price::OracleSetup},
};
use pagerduty_rs::{
    eventsv2sync::EventsV2,
    types::{AlertResolve, AlertTrigger, AlertTriggerPayload, Event},
};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use serde::Deserialize;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::pubkey::Pubkey;
use structopt::StructOpt;
use switchboard_on_demand::PullFeedAccountData;
use switchboard_solana::{AggregatorAccountData, AnchorDeserialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, Deserialize)]
pub struct MarginfiAlerterConfig {
    rpc_url: String,
    pd_integration_key: String,
    marginfi_program_id: String,
    marginfi_groups: Vec<MarginfiGroupAlertingConfig>,
    balance_alert: Vec<BalanceAlertingConfig>,
}

impl MarginfiAlerterConfig {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let config_str = std::io::read_to_string(reader)?;
        let config = toml::from_str(&config_str)?;
        Ok(config)
    }
}

#[derive(Clone, Debug, Deserialize)]
struct MarginfiGroupAlertingConfig {
    address: String,
    max_age_secs: i64,
}

#[derive(Clone, Debug, Deserialize)]
struct BalanceAlertingConfig {
    address: String,
    min_balance: f64,
    label: Option<String>,
}

#[derive(Clone, Debug, StructOpt)]
pub struct MarginfiAlerter {
    config_path: String,
}

struct AlertingContext {
    rpc_client: RpcClient,
    config: MarginfiAlerterConfig,
    group_config_map: HashMap<Pubkey, MarginfiGroupAlertingConfig>,
    pd: EventsV2,
}

fn main() {
    env_logger::init();
    let args = MarginfiAlerter::from_args();
    let config = MarginfiAlerterConfig::load_from_file(&args.config_path).unwrap();
    let rpc_client = RpcClient::new(config.rpc_url.clone());

    println!(
        "Starting marginfi alerter, evaluating {} groups and {} balance alerts",
        config.marginfi_groups.len(),
        config.balance_alert.len()
    );

    let group_config_map = config
        .marginfi_groups
        .iter()
        .map(|group| (Pubkey::from_str(&group.address).unwrap(), group.clone()))
        .collect();

    let context = AlertingContext {
        rpc_client,
        config: config.clone(),
        group_config_map,
        pd: EventsV2::new(config.pd_integration_key.clone(), None).unwrap(),
    };

    match check_all(&context) {
        Ok(_) => {
            clear_error_alert(&context).unwrap();
        }
        Err(e) => {
            error!("Error running marginfi alerter: {:?}", e);
            send_error_alert(&context, &format!("Error running marginfi alerter: {}", e)).unwrap();
            eprintln!("{:#?}", e);
            std::process::exit(1);
        }
    }
}

fn check_all(context: &AlertingContext) -> anyhow::Result<()> {
    check_marginfi_groups(context)?;
    check_balance_alert(context)?;
    Ok(())
}

fn send_error_alert(context: &AlertingContext, message: &str) -> anyhow::Result<()> {
    let event = Event::AlertTrigger::<String>(AlertTrigger {
        payload: AlertTriggerPayload {
            summary: message.to_string(),
            source: "marginfi-alerter".to_string(),
            severity: pagerduty_rs::types::Severity::Critical,
            custom_details: None,
            component: None,
            group: None,
            class: None,
            timestamp: None,
        },
        dedup_key: Some("marginfi-alerter-error".to_string()),
        images: None,
        links: None,
        client: None,
        client_url: None,
    });

    context.pd.event(event).map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

fn clear_error_alert(context: &AlertingContext) -> anyhow::Result<()> {
    let event = Event::AlertResolve::<String>(AlertResolve {
        dedup_key: "marginfi-alerter-error".to_string(),
    });

    context.pd.event(event).map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

fn check_marginfi_groups(context: &AlertingContext) -> anyhow::Result<()> {
    for group in context.config.marginfi_groups.iter() {
        check_marginfi_group(context, group)?;
    }

    Ok(())
}

fn check_balance_alert(context: &AlertingContext) -> anyhow::Result<()> {
    for balance in context.config.balance_alert.iter() {
        check_balance(context, balance)?;
    }

    Ok(())
}

fn check_balance(
    context: &AlertingContext,
    balance_config: &BalanceAlertingConfig,
) -> anyhow::Result<()> {
    let address = Pubkey::from_str(&balance_config.address)?;
    let balance = context.rpc_client.get_balance(&address)?;

    let min_balance_lamports = (balance_config.min_balance * 10u64.pow(9) as f64).floor() as u64;

    info!(
        "Balance for account {} is {}, min balance is {}",
        address,
        balance as f64 / 10u64.pow(9) as f64,
        balance_config.min_balance
    );

    if balance < min_balance_lamports {
        send_balance_alert(context, balance_config, balance)?;
    } else {
        clear_balance_alert(context, &address)?;
    }

    Ok(())
}

fn check_marginfi_group(
    context: &AlertingContext,
    group: &MarginfiGroupAlertingConfig,
) -> anyhow::Result<()> {
    let marginfi_program_id = Pubkey::from_str(&context.config.marginfi_program_id)?;
    let group_address = Pubkey::from_str(&group.address)?;
    let bank_accounts = context.rpc_client.get_program_accounts_with_config(
        &marginfi_program_id,
        RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                8 + size_of::<Pubkey>() + size_of::<u8>(),
                group_address.to_bytes().to_vec(),
            ))]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..Default::default()
            },
            with_context: None,
        },
    )?;

    let banks = bank_accounts
        .into_iter()
        .map(|(address, account)| {
            let data = account.data.as_slice();
            bytemuck::try_from_bytes::<Bank>(&data[8..])
                .cloned()
                .map(|b| (address, b))
        })
        .collect::<Result<Vec<(Pubkey, Bank)>, _>>()
        .map_err(|e| anyhow::anyhow!(e))?;

    info!("Found {} banks in group", banks.len());

    let switchboard_v2_oracles = banks
        .iter()
        .filter(|(_, bank)| bank.config.oracle_setup == OracleSetup::SwitchboardV2)
        .collect::<Vec<_>>();
    // Pyth legacy is deprecated
    let _pyth_oracles = banks
        .iter()
        .filter(|(_, bank)| bank.config.oracle_setup == OracleSetup::PythLegacy)
        .collect::<Vec<_>>();
    let pyth_push_oracles = banks
        .iter()
        .filter(|(_, bank)| bank.config.oracle_setup == OracleSetup::PythPushOracle)
        .collect::<Vec<_>>();
    let switchboard_pull_oracles = banks
        .iter()
        .filter(|(_, bank)| bank.config.oracle_setup == OracleSetup::SwitchboardPull)
        .collect::<Vec<_>>();

    check_switchboard_v2_oracles(context, &switchboard_v2_oracles)?;
    check_pyth_push_oracles(context, &pyth_push_oracles)?;
    check_switchboard_pull_oracles(context, &switchboard_pull_oracles)?;

    Ok(())
}

fn check_switchboard_v2_oracles(
    context: &AlertingContext,
    banks: &[&(Pubkey, Bank)],
) -> anyhow::Result<()> {
    info!("Checking {} switchboard v2 oracles", banks.len());
    for (address, bank) in banks {
        check_switchboard_v2_oracle(context, address, bank)?;
    }

    Ok(())
}

fn check_switchboard_v2_oracle(
    context: &AlertingContext,
    address: &Pubkey,
    bank: &Bank,
) -> anyhow::Result<()> {
    let oracle_address = bank.config.oracle_keys.first().unwrap();
    let oracle_account = context.rpc_client.get_account(oracle_address)?;
    let oracle = bytemuck::try_from_bytes::<AggregatorAccountData>(&oracle_account.data[8..])
        .map_err(|e| anyhow::anyhow!(e))?;
    let group_config = context.group_config_map.get(&bank.group).unwrap();
    let max_age = group_config.max_age_secs;
    let last_update = oracle.latest_confirmed_round.round_open_timestamp;
    let current_time = get_current_unix_timestamp_secs();
    let oracle_age = current_time - last_update;

    info!(
        "Switchboard V2 oracle for bank {} is {} seconds old",
        address, oracle_age
    );

    if oracle_age > max_age {
        send_stale_oracle_alert(context, address, oracle_age)?;
    } else {
        clear_stale_oracle_alert(context, address)?;
    }

    Ok(())
}

fn check_pyth_push_oracles(
    context: &AlertingContext,
    banks: &[&(Pubkey, Bank)],
) -> anyhow::Result<()> {
    for (address, bank) in banks {
        check_pyth_push_oracle(context, address, bank)?;
    }

    Ok(())
}

fn check_pyth_push_oracle(
    context: &AlertingContext,
    address: &Pubkey,
    bank: &Bank,
) -> anyhow::Result<()> {
    let oracle_address = bank.config.oracle_keys.first().unwrap();
    let oracle_account = {
        let (marginfi_sponsored_oracle_address, _) =
            marginfi::state::price::PythPushOraclePriceFeed::find_oracle_address(
                PYTH_PUSH_MARGINFI_SPONSORED_SHARD_ID,
                oracle_address.as_ref().try_into().unwrap(),
            );
        let (pyth_sponsered_oracle_address, _) =
            marginfi::state::price::PythPushOraclePriceFeed::find_oracle_address(
                PYTH_PUSH_PYTH_SPONSORED_SHARD_ID,
                oracle_address.as_ref().try_into().unwrap(),
            );

        let accounts = context.rpc_client.get_multiple_accounts(&[
            marginfi_sponsored_oracle_address,
            pyth_sponsered_oracle_address,
        ])?;

        match (accounts.first().cloned(), accounts.get(1).cloned()) {
            (Some(Some(account)), _) => account,
            (_, Some(Some(account))) => account,
            _ => anyhow::bail!("Oracle account for bank {} not found", address),
        }
    };

    let price_update = PriceUpdateV2::deserialize(&mut &oracle_account.data[8..])?;
    let group_config = context.group_config_map.get(&bank.group).unwrap();
    let publish_time = price_update.price_message.publish_time;
    let current_time = get_current_unix_timestamp_secs();
    let max_age = group_config.max_age_secs;
    let oracle_age = current_time - publish_time;

    info!(
        "Pyth push oracle for bank {} is {} seconds old",
        address, oracle_age
    );

    if oracle_age > max_age {
        send_stale_oracle_alert(context, address, oracle_age)?;
    } else {
        clear_stale_oracle_alert(context, address)?;
    }

    Ok(())
}

fn check_switchboard_pull_oracles(
    context: &AlertingContext,
    banks: &[&(Pubkey, Bank)],
) -> anyhow::Result<()> {
    for (address, bank) in banks {
        check_switchboard_pull_oracle(context, address, bank)?;
    }

    Ok(())
}

fn check_switchboard_pull_oracle(
    context: &AlertingContext,
    address: &Pubkey,
    bank: &Bank,
) -> anyhow::Result<()> {
    let oracle_address = bank.config.oracle_keys.first().unwrap();
    let oracle_account = context.rpc_client.get_account(oracle_address)?;
    let pull_feed = bytemuck::try_from_bytes::<PullFeedAccountData>(&oracle_account.data[8..])
        .map_err(|e| anyhow::anyhow!(e))?;
    let group_config = context.group_config_map.get(&bank.group).unwrap();
    let max_age = group_config.max_age_secs;
    let current_time = get_current_unix_timestamp_secs();
    let last_update = pull_feed.last_update_timestamp;
    let oracle_age = current_time - last_update;

    info!(
        "Switchboard pull oracle for bank {} is {} seconds old",
        address, oracle_age
    );

    if oracle_age > max_age {
        send_stale_oracle_alert(context, address, oracle_age)?;
    } else {
        clear_stale_oracle_alert(context, address)?;
    }

    Ok(())
}

fn send_balance_alert(
    context: &AlertingContext,
    balance_config: &BalanceAlertingConfig,
    balance: u64,
) -> anyhow::Result<()> {
    let balance_ui = balance as f64 / 10u64.pow(9) as f64;
    warn!(
        "Account {} ({}) has balance of {} below minimum {}",
        balance_config.address,
        balance_config
            .label
            .clone()
            .unwrap_or(balance_config.address.to_string()),
        balance_ui,
        balance_config.min_balance
    );

    context
        .pd
        .event(pagerduty_rs::types::Event::AlertTrigger::<String>(
            AlertTrigger {
                payload: AlertTriggerPayload {
                    severity: pagerduty_rs::types::Severity::Critical,
                    summary: format!(
                        "Account {} ({}) has balance of {} below minimum {}",
                        balance_config.address,
                        balance_config
                            .label
                            .clone()
                            .unwrap_or(balance_config.address.to_string()),
                        balance_ui,
                        balance_config.min_balance
                    ),
                    source: "marginfi-alerter".to_string(),
                    timestamp: Some(OffsetDateTime::now_utc()),
                    component: None,
                    group: None,
                    class: None,
                    custom_details: None,
                },
                dedup_key: Some(format!("balance-{}", balance_config.address)),
                images: None,
                links: None,
                client: None,
                client_url: None,
            },
        ))?;

    Ok(())
}

fn clear_balance_alert(context: &AlertingContext, address: &Pubkey) -> anyhow::Result<()> {
    context
        .pd
        .event(Event::AlertResolve::<String>(AlertResolve {
            dedup_key: format!("balance-{}", address),
        }))?;

    Ok(())
}

fn send_stale_oracle_alert(
    context: &AlertingContext,
    address: &Pubkey,
    oralce_age_secs: i64,
) -> anyhow::Result<()> {
    warn!(
        "Oracle for bank {} is stale by {} seconds, sending alert",
        address, oralce_age_secs
    );

    context
        .pd
        .event(pagerduty_rs::types::Event::AlertTrigger::<String>(
            AlertTrigger {
                payload: AlertTriggerPayload {
                    severity: pagerduty_rs::types::Severity::Critical,
                    summary: format!(
                        "Oracle for bank {} is stale by {} seconds",
                        address, oralce_age_secs
                    ),
                    source: "marginfi-alerter".to_string(),
                    timestamp: Some(OffsetDateTime::now_utc()),
                    component: None,
                    group: None,
                    class: None,
                    custom_details: None,
                },
                dedup_key: Some(get_oracle_dedup_key(address)),
                images: None,
                links: None,
                client: None,
                client_url: None,
            },
        ))?;

    Ok(())
}

fn clear_stale_oracle_alert(context: &AlertingContext, address: &Pubkey) -> anyhow::Result<()> {
    context
        .pd
        .event(Event::AlertResolve::<String>(AlertResolve {
            dedup_key: get_oracle_dedup_key(address),
        }))?;

    Ok(())
}

fn get_oracle_dedup_key(address: &Pubkey) -> String {
    format!("stale-oracle-{}", address)
}

fn get_current_unix_timestamp_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
}
