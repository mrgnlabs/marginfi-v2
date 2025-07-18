use fixtures::test::TestFixture;
use marginfi_type_crate::types::{MarginfiAccount, ACCOUNT_DISABLED};
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer};

// Test transfer account authority.
// No transfer flag set -- no longer matters, tx should succeed.
// RUST_BACKTRACE=1 cargo test-bpf marginfi_account_authority_transfer_no_flag_set -- --exact
#[tokio::test]
async fn marginfi_account_transfer_happy_path() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    // Default account with no flags set
    let marginfi_account = test_f.create_marginfi_account().await;
    let new_authority = Keypair::new();
    let new_account = Keypair::new();

    let res = marginfi_account
        .try_transfer_account(
            new_account.pubkey(),
            new_authority.pubkey(),
            None,
            &new_account,
            test_f.marginfi_group.fee_wallet,
        )
        .await;
    assert!(res.is_ok());

    // Old account still has the old authority, but is now inactive
    let account_old = marginfi_account.load().await;
    assert_eq!(account_old.authority, test_f.payer());
    assert_eq!(account_old.account_flags, ACCOUNT_DISABLED);

    // The new account has the new authority
    let account_new: MarginfiAccount = test_f.load_and_deserialize(&new_account.pubkey()).await;
    assert_eq!(account_new.authority, new_authority.pubkey());
    // Old account is recorded as the migration source
    assert_eq!(account_new.migrated_from, marginfi_account.key);

    Ok(())
}

#[tokio::test]
async fn marginfi_account_transfer_not_account_owner() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    let marginfi_account = test_f.create_marginfi_account().await;
    let new_authority = Keypair::new();
    let new_account = Keypair::new();
    let signer = Keypair::new();

    let tx = marginfi_account
        .get_tx_transfer_account(
            new_account.pubkey(),
            new_authority.pubkey(),
            Some(signer),
            &new_account,
            test_f.marginfi_group.fee_wallet,
        )
        .await;

    // Note: Sending this tx takes a very long time (longer than all the other tests combined)
    // because for some reason it takes longer for a signature verification fail to return than it
    // does for other errors. We simulate instead here for testing SPEEEEEED
    let ctx = test_f.context.borrow_mut();
    let res = ctx.banks_client.simulate_transaction(tx).await;
    let is_err = res.unwrap().result.unwrap().is_err();

    // Assert the response is an error due to fact that a non-owner of the
    // acount attempted to initialize this account transfer
    assert!(is_err);

    Ok(())
}
