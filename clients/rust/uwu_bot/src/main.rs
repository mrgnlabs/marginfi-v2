use anchor_client::{
    anchor_lang::{InstructionData, ToAccountMetas},
    Cluster, Program,
};
use anyhow::Result;
use marginfi::{constants::LIQUIDITY_VAULT_SEED, state::marginfi_group::Bank};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::RpcSendTransactionConfig,
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, instruction::Instruction, pubkey::Pubkey,
    signature::Keypair,
};
use std::{env, fs, rc::Rc, str::FromStr, time::Duration};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    let program_id = Pubkey::from_str(&env::var("MARGINFI_PROGRAM").unwrap()).unwrap();
    let group_id = Pubkey::from_str(&env::var("MARGINFI_GROUP").unwrap()).unwrap();
    let rpc = RpcClient::new(env::var("RPC_ENDPOINT").unwrap());
    let signer = fs::read(env::var("KEYPAIR_PATH").unwrap()).unwrap();

    let client = anchor_client::Client::new(
        Cluster::Custom(rpc.url(), "".to_owned()),
        Rc::new(Keypair::from_bytes(&signer)?),
    );
    let program = Rc::new(client.program(program_id));

    loop {
        let banks = load_all_banks_for_group(program.clone(), group_id)
            .await
            .unwrap();
        let mut request = program
            .state_request()
            .instruction(ComputeBudgetInstruction::set_compute_unit_limit(1_000_000));

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

        sleep(Duration::from_secs(1)).await;
    }
}

async fn load_all_banks_for_group(
    program: Rc<Program>,
    group_id: Pubkey,
) -> Result<Vec<(Pubkey, Bank)>> {
    Ok(
        program.accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            48,
            group_id.to_bytes().to_vec(),
        ))])?,
    )
}
