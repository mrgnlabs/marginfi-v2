use {
    crate::{config::Config, profile::Profile},
    anchor_client::anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas},
    anyhow::Result,
    marginfi::state::marginfi_account::MarginfiAccount,
    solana_client::rpc_filter::{Memcmp, RpcFilterType},
    solana_sdk::{
        instruction::Instruction, message::Message, pubkey::Pubkey, transaction::Transaction,
    },
};

const CHUNK_SIZE: usize = 22;

pub fn claim_all_emissions_for_bank(
    config: &Config,
    profile: &Profile,
    bank_pk: Pubkey,
) -> Result<()> {
    let rpc_client = config.mfi_program.rpc();

    let group = profile.marginfi_group.expect("group not set");

    let signing_keypairs = config.get_signers(false);

    let marginfi_accounts =
        config
            .mfi_program
            .accounts::<MarginfiAccount>(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                8,
                group.try_to_vec()?,
            ))])?;

    let ixs = marginfi_accounts
        .into_iter()
        .filter_map(|(address, account)| {
            if account
                .lending_account
                .balances
                .iter()
                .any(|balance| balance.active && balance.bank_pk == bank_pk)
            {
                Some(address)
            } else {
                None
            }
        })
        .map(|address| Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountSettleEmissions {
                marginfi_account: address,
                bank: bank_pk,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingAccountSettleEmissions {}.data(),
        })
        .collect::<Vec<_>>();

    println!("Found {} accounts", ixs.len());

    let ixs_batches = ixs.chunks(CHUNK_SIZE);
    let ixs_batches_count = ixs_batches.len();

    // Send txs and show progress to user [n/total]
    println!("Sending {} txs", ixs_batches_count);

    for (i, ixs) in ixs_batches.enumerate() {
        let blockhash = rpc_client.get_latest_blockhash()?;

        let message = Message::new(ixs, Some(&config.authority()));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.partial_sign(&signing_keypairs, blockhash);

        let sig = rpc_client.send_and_confirm_transaction_with_spinner(&transaction)?;

        println!("Sent [{}/{}] {}", i + 1, ixs_batches_count, sig);
    }

    println!("Done!");

    Ok(())
}
