use anchor_client::{
    anchor_lang::{InstructionData, ToAccountMetas},
    Cluster, Program,
};
use anyhow::Result;
use clap::Parser;
use log::info;
use marginfi::{
    bank_authority_seed, bank_seed,
    constants::LIQUIDITY_VAULT_SEED,
    state::marginfi_group::{Bank, BankVaultType},
};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{
        EncodingConfig, RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcSendTransactionConfig,
    },
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
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

const DEFAULT_PROGRAM_ID: &str = "mf2iDQbVTAE3tT4tgAZBhBAmKUW56GsXX7H3oeH4atr";
const DEFAULT_GROUP_ID: &str = "EqmdxNhQS2Snwzh5nsBB3JkWGaaBqefcC5Lgj2Zjiv78";
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
                liquidity_vault_authority: find_bank_vault_authority_pda(
                    bank_pk,
                    BankVaultType::Liquidity,
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

        // request
        //     .send_with_spinner_and_config(RpcSendTransactionConfig::default())
        //     .unwrap();
        request.send().unwrap();

        sleep(Duration::from_secs(interval_sec)).await;
    }
}

async fn load_all_banks_for_group(
    program: Rc<Program>,
    group_id: Pubkey,
) -> Result<Vec<(Pubkey, Bank)>> {
    Ok(program.accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp {
        offset: 8 + size_of::<Pubkey>() + size_of::<u8>(),
        bytes: MemcmpEncodedBytes::Bytes(group_id.to_bytes().to_vec()),
        encoding: None,
    })])?)
}

pub fn find_bank_vault_pda(
    bank_pk: &Pubkey,
    vault_type: BankVaultType,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(bank_seed!(vault_type, bank_pk), program_id)
}

pub fn find_bank_vault_authority_pda(
    bank_pk: &Pubkey,
    vault_type: BankVaultType,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(bank_authority_seed!(vault_type, bank_pk), program_id)
}
