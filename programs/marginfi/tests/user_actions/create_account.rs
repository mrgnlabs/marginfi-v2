use anchor_lang::{InstructionData, ToAccountMetas};
use fixtures::test::TestFixture;
use marginfi::state::marginfi_account::MarginfiAccount;
use solana_program_test::tokio;
use solana_sdk::{
    instruction::Instruction, signature::Keypair, signer::Signer, system_program,
    transaction::Transaction,
};

#[tokio::test]
async fn marginfi_account_create_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    let marginfi_account_key = Keypair::new();
    let accounts = marginfi::accounts::MarginfiAccountInitialize {
        marginfi_group: test_f.marginfi_group.key,
        marginfi_account: marginfi_account_key.pubkey(),
        authority: test_f.payer(),
        fee_payer: test_f.payer(),
        system_program: system_program::id(),
    };
    let init_marginfi_account_ix = Instruction {
        program_id: marginfi::id(),
        accounts: accounts.to_account_metas(Some(true)),
        data: marginfi::instruction::MarginfiAccountInitialize {}.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[init_marginfi_account_ix],
        Some(&test_f.payer()),
        &[&test_f.payer_keypair(), &marginfi_account_key],
        test_f.get_latest_blockhash().await,
    );

    let res = test_f
        .context
        .borrow_mut()
        .banks_client
        .process_transaction(tx)
        .await;

    assert!(res.is_ok());

    let marginfi_account: MarginfiAccount = test_f
        .load_and_deserialize(&marginfi_account_key.pubkey())
        .await;

    assert_eq!(marginfi_account.group, test_f.marginfi_group.key);
    assert_eq!(marginfi_account.authority, test_f.payer());
    assert!(marginfi_account
        .lending_account
        .balances
        .iter()
        .all(|bank| !bank.is_active()));

    Ok(())
}
