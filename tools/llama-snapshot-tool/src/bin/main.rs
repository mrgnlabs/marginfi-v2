use std::{collections::HashMap, env, rc::Rc};

use anchor_client::Client;
use anyhow::Result;
use fixed::types::I80F48;
use futures::future::join_all;
use lazy_static::lazy_static;
use marginfi::{
    constants::{EMISSIONS_FLAG_BORROW_ACTIVE, EMISSIONS_FLAG_LENDING_ACTIVE, SECONDS_PER_YEAR},
    state::marginfi_group::Bank,
};
use reqwest::header::CONTENT_TYPE;
use s3::{creds::Credentials, Bucket, Region};
use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signature::Keypair};

lazy_static! {
    static ref TOKEN_LIST: HashMap<String, String> = {
        let token_list_path = env::var("TOKEN_LIST_PATH").expect("TOKEN_LIST_PATH not set");
        println!("Loading token list from {}", token_list_path);
        let tokens_file = std::fs::read_to_string(token_list_path).unwrap();
        let tokens = serde_json::from_str::<Value>(&tokens_file).unwrap();

        HashMap::from_iter(
            tokens
                .get("tokens")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|token| {
                    (
                        token.get("address").unwrap().as_str().unwrap().to_owned(),
                        token.get("symbol").unwrap().as_str().unwrap().to_owned(),
                    )
                })
                .collect::<Vec<(String, String)>>(),
        )
    };
}

const BIRDEYE_API: &str = "https://public-api.birdeye.so";
const CHAIN: &str = "solana";
const PROJECT: &str = "marginfi";
const S3_BUCKET: &str = "marignfi-pools-snapshot";
const AWS_S3_OBJ_PATH: &str = "snapshot.json";

#[tokio::main]
async fn main() -> Result<()> {
    let dummy_key = Keypair::new();
    let rpc_url = env::var("RPC_ENDPOINT").expect("RPC_ENDPOINT not set");
    let client = Client::new(
        anchor_client::Cluster::Custom(rpc_url.to_string(), "".to_string()),
        Rc::new(dummy_key),
    );

    let program = client.program(marginfi::id()).unwrap();
    let rpc = program.rpc();

    let banks = program.accounts::<Bank>(vec![])?;

    println!("Found {} banks", banks.len());

    let snapshot = join_all(
        banks
            .iter()
            .map(|(bank_pk, bank)| DefiLammaPoolInfo::from_bank(bank, bank_pk, &rpc))
            .collect::<Vec<_>>(),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>>>()?;

    let snapshot_json = serde_json::to_string(&snapshot).unwrap();

    println!("Banks: {:#?}", snapshot);

    store_on_s3(&snapshot_json).await?;

    Ok(())
}

async fn store_on_s3(snapshot: &str) -> anyhow::Result<()> {
    println!("Storing snapshot on to {}", AWS_S3_OBJ_PATH);
    let credentials = Credentials::new(
        Some(&env::var("AWS_ACCESS_KEY").expect("AWS_ACCESS_KEY not set")),
        Some(&env::var("AWS_SECRET_KEY").expect("AWS_SECRET_KEY not set")),
        None,
        None,
        None,
    )?;

    let bucket = Bucket::new(S3_BUCKET, Region::UsEast1, credentials)?;

    bucket
        .put_object(AWS_S3_OBJ_PATH, snapshot.as_bytes())
        .await?;

    println!("Done!");

    Ok(())
}

#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DefiLammaPoolInfo {
    pool: String,
    chain: String,
    project: String,
    symbol: String,
    tvl_usd: f64,
    total_supply_usd: f64,
    total_borrow_usd: f64,
    ltv: f64,
    apy_base: f64,
    apy_reward: Option<f64>,
    apy_base_borrow: f64,
    apy_reward_borrow: Option<f64>,
    reward_tokens: Vec<String>,
    underlying_tokens: Vec<String>,
}

impl DefiLammaPoolInfo {
    pub async fn from_bank(bank: &Bank, bank_pk: &Pubkey, rpc_client: &RpcClient) -> Result<Self> {
        let ltv = I80F48::ONE / I80F48::from(bank.config.liability_weight_init);
        let reward_tokens = if bank.emissions_mint != Pubkey::default() {
            vec![bank.emissions_mint.to_string()]
        } else {
            vec![]
        };

        let token_price = fetch_price_from_birdeye(&bank.mint).await?;
        let scale = I80F48::from_num(10_i32.pow(bank.mint_decimals as u32));

        let total_deposits = bank.get_asset_amount(bank.total_asset_shares.into())? / scale;
        let total_borrows = bank.get_liability_amount(bank.total_liability_shares.into())? / scale;

        let net_supply = total_deposits - total_borrows;

        let tvl_usd = token_price * net_supply;

        let total_supply_usd = token_price * total_deposits;
        let total_borrow_usd = token_price * total_borrows;

        let token_mint = bank.mint.to_string();

        let ur = if total_deposits > 0 {
            total_borrows / total_deposits
        } else {
            I80F48::ZERO
        };

        let (lending_rate, borrowing_rate, _, _) = bank
            .config
            .interest_rate_config
            .calc_interest_rate(ur)
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to calculate interest rate for bank {}", bank_pk)
            })?;

        let (apr_reward, apr_reward_borrow) = if bank.emissions_mint.ne(&Pubkey::default()) {
            let emissions_token_price = fetch_price_from_birdeye(&bank.emissions_mint).await?;
            let mint = rpc_client.get_account(&bank.emissions_mint)?;
            let mint = spl_token::state::Mint::unpack_from_slice(&mint.data)?;

            // rate / 10 ^ decimals
            let reward_rate_per_token =
                bank.emissions_rate as f64 / 10i32.pow(mint.decimals as u32) as f64;
            let relative_emissions_value = (emissions_token_price
                * I80F48::from_num(reward_rate_per_token))
                / I80F48::from_num(token_price);

            (
                if bank.get_emissions_flag(EMISSIONS_FLAG_LENDING_ACTIVE) {
                    Some(relative_emissions_value)
                } else {
                    None
                },
                if bank.get_emissions_flag(EMISSIONS_FLAG_BORROW_ACTIVE) {
                    Some(relative_emissions_value)
                } else {
                    None
                },
            )
        } else {
            (None, None)
        };

        Ok(Self {
            pool: bank_pk.to_string(),
            chain: CHAIN.to_string(),
            project: PROJECT.to_string(),
            symbol: TOKEN_LIST
                .get(token_mint.as_str())
                .unwrap_or(&"Unknown Token".to_string())
                .to_string(),
            tvl_usd: tvl_usd.to_num(),
            total_supply_usd: total_supply_usd.to_num(),
            total_borrow_usd: total_borrow_usd.to_num(),
            ltv: ltv.to_num(),
            reward_tokens,
            apy_base: dec_to_percentage(apr_to_apy(
                lending_rate.to_num(),
                SECONDS_PER_YEAR.to_num(),
            )),
            apy_reward: apr_reward.map(|a| {
                dec_to_percentage(apr_to_apy(
                    (lending_rate + a).to_num(),
                    SECONDS_PER_YEAR.to_num(),
                ))
            }),
            apy_base_borrow: dec_to_percentage(apr_to_apy(
                borrowing_rate.to_num(),
                SECONDS_PER_YEAR.to_num(),
            )),
            apy_reward_borrow: apr_reward_borrow.map(|a| {
                dec_to_percentage(apr_to_apy(
                    (borrowing_rate + a).to_num(),
                    SECONDS_PER_YEAR.to_num(),
                ))
            }),
            underlying_tokens: vec![bank.mint.to_string()],
        })
    }
}

async fn fetch_price_from_birdeye(token: &Pubkey) -> Result<I80F48> {
    println!("Fetching price for {}", token);
    let url = format!("{}/public/price?address={}", BIRDEYE_API, token);
    let client = reqwest::Client::new();

    let res = client
        .get(&url)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await?;

    let body = res.json::<serde_json::Value>().await?;

    let price = body
        .as_object()
        .unwrap()
        .get("data")
        .unwrap()
        .get("value")
        .unwrap()
        .as_f64();

    Ok(I80F48::from_num(price.unwrap()))
}

fn apr_to_apy(apr: f64, m: f64) -> f64 {
    (1. + (apr / m)).powf(m) - 1.
}

fn dec_to_percentage(dec: f64) -> f64 {
    dec * 100.
}
