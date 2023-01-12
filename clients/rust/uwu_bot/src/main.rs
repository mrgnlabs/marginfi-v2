use anchor_client::{
    anchor_lang::{InstructionData, ToAccountMetas},
    Cluster, Program,
};
use anyhow::Result;
use clap::Parser;
use log::info;
use marginfi::{constants::LIQUIDITY_VAULT_SEED, state::marginfi_group::Bank};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{
        EncodingConfig, RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcSendTransactionConfig,
    },
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
};
use std::{env, fs, mem::size_of, rc::Rc, str::FromStr, time::Duration};
use tokio::time::sleep;

const DEFAULT_PROGRAM_ID: &str = "EPsDwX4sRNRkiykuqeyExF5LsHV9XBPMZM6gHj7QQbkY";
const DEFAULT_GROUP_ID: &str = "2y5NtJQVpaDPynjHFSEAcPJ6ZFWeReJaK2sCYFbRaERC";
const DEFAULT_RPC_URL: &str = "https://devnet.genesysgo.net";

#[derive(Parser)]
struct Ops {
    #[clap(subcommand)]
    cmd: Command,
    #[clap(flatten)]
    global: GlobalOptions,
}
#[derive(Parser)]
struct GlobalOptions {
    #[clap(
        global = true,
        short = 'p',
        long = "program",
        default_value = DEFAULT_PROGRAM_ID
    )]
    program_id: String,
    #[clap(
        global = true,
        short = 'g',
        long = "group",
        default_value = DEFAULT_GROUP_ID
    )]
    group_id: String,
    #[clap(
        global = true,
        short = 'u',
        long = "url",
        default_value = DEFAULT_RPC_URL
    )]
    rpc_url: String,
    #[clap(
        global = true,
        short = 'k',
        long = "keypair",
        default_value = "~/.config/solana/id.json"
    )]
    wallet_path: String,
}

#[derive(Parser)]
enum Command {
    #[clap(name = "crank")]
    Crank(CrankOptions),
}

#[derive(Parser)]
struct CrankOptions {
    #[clap(short = 'i', long = "interval", default_value = "1")]
    interval_sec: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    info!("Starting interest cranker (* ^ ω ^)");

    let Ops { cmd, global } = Ops::parse();

    let program_id = Pubkey::from_str(&global.program_id).unwrap();
    let group_id = Pubkey::from_str(&global.group_id).unwrap();
    let rpc = RpcClient::new(global.rpc_url);
    let signer = read_keypair_file(shellexpand::tilde(&global.wallet_path).to_string()).unwrap();

    let client =
        anchor_client::Client::new(Cluster::Custom(rpc.url(), "".to_owned()), Rc::new(signer));
    let program = Rc::new(client.program(program_id));

    let Command::Crank(CrankOptions { interval_sec }) = cmd;

    loop {
        let banks = load_all_banks_for_group(program.clone(), group_id)
            .await
            .unwrap();
        let mut request = program
            .state_request()
            .instruction(ComputeBudgetInstruction::set_compute_unit_limit(1_000_000))
            .options(CommitmentConfig::confirmed());

        info!("Cranking {} banks ヽ(*・ω・)ﾉ", banks.len());

        let ixs = banks.iter().map(|(bank_pk, bank)| Instruction {
            program_id,
            accounts: marginfi::accounts::LendingPoolBankAccrueInterest {
                marginfi_group: group_id,
                bank: *bank_pk,
                liquidity_vault_authority: Pubkey::find_program_address(
                    &[LIQUIDITY_VAULT_SEED, bank_pk.as_ref()],
                    &program_id,
                )
                .0,
                liquidity_vault: bank.liquidity_vault,
                insurance_vault: bank.insurance_vault,
                fee_vault: bank.fee_vault,
                token_program: anchor_spl::token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::BankAccrueInterest.data(),
        });

        for ix in ixs {
            request = request.instruction(ix);
        }

        request
            .send_with_spinner_and_config(RpcSendTransactionConfig::default())
            .unwrap();

        sleep(Duration::from_secs(interval_sec)).await;
    }
}

async fn load_all_banks_for_group(
    program: Rc<Program>,
    group_id: Pubkey,
) -> Result<Vec<(Pubkey, Bank)>> {
    Ok(program
        .rpc()
        .get_program_accounts_with_config(
            &program.id(),
            RpcProgramAccountsConfig {
                filters: Some(vec![RpcFilterType::DataSize(472)]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                },
                ..Default::default()
            },
        )?
        .iter()
        .map(|(key, account)| (*key, *bytemuck::from_bytes::<Bank>(&account.data[8..])))
        .collect::<Vec<_>>())
}
