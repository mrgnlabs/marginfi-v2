#[cfg(feature = "admin")]
const CHUNK_SIZE: usize = 22;
#[cfg(feature = "admin")]
pub fn claim_all_emissions_for_bank(
    config: &Config,
    profile: &Profile,
    bank_pk: Pubkey,
) -> Result<()> {
    let group = profile.marginfi_group.expect("group not set");

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
    let blockhash = config.mfi_program.rpc().get_latest_blockhash()?;

    for (i, ixs) in ixs_batches.enumerate() {
        let tx = Transaction::new_signed_with_payer(
            ixs,
            Some(&config.payer.pubkey()),
            &[&config.payer],
            blockhash,
        );

        let sig = config
            .mfi_program
            .rpc()
            .send_and_confirm_transaction_with_spinner(&tx)?;

        println!("Sent [{}/{}] {}", i + 1, ixs_batches_count, sig);
    }

    println!("Done!");

    Ok(())
}
