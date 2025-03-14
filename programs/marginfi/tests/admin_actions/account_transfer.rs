use fixtures::{assert_custom_error, test::TestFixture};
use marginfi::{
    errors::MarginfiError, state::marginfi_account::ACCOUNT_TRANSFER_AUTHORITY_ALLOWED,
};
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer};

// Test transfer account authority.
// No transfer flag set -- tx should fail.
// Set the flag and try again -- tx should succeed.
// RUST_BACKTRACE=1 cargo test-bpf marginfi_account_authority_transfer_no_flag_set -- --exact
#[tokio::test]
async fn marginfi_account_authority_transfer_no_flag_set() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    // Default account with no flags set
    let marginfi_account = test_f.create_marginfi_account().await;
    let new_authority = Keypair::new().pubkey();

    let res = marginfi_account
        .try_transfer_account_authority(new_authority, None)
        .await;

    // Check transfer authority is unchanged
    let account = marginfi_account.load().await;
    assert_eq!(account.authority, test_f.payer());

    // Assert the response is an error due to the lack of the correct flag
    assert!(res.is_err());
    assert_custom_error!(
        res.unwrap_err(),
        MarginfiError::IllegalAccountAuthorityTransfer
    );

    // set the flag on the account
    marginfi_account
        .try_set_flag(ACCOUNT_TRANSFER_AUTHORITY_ALLOWED)
        .await
        .unwrap();

    // Check transfer authority flag
    let account = marginfi_account.load().await;
    assert!(account.get_flag(ACCOUNT_TRANSFER_AUTHORITY_ALLOWED));

    let new_authority_2 = Keypair::new().pubkey();
    let res = marginfi_account
        .try_transfer_account_authority(new_authority_2, None)
        .await;

    assert!(res.is_ok());

    // Check transfer authority
    let account = marginfi_account.load().await;
    assert_eq!(account.authority, new_authority_2);

    Ok(())
}

#[tokio::test]
async fn marginfi_account_authority_transfer_not_account_owner() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    let marginfi_account = test_f.create_marginfi_account().await;
    let new_authority = Keypair::new().pubkey();
    let signer = Keypair::new();

    let tx = marginfi_account
        .get_tx_transfer_account_authority(new_authority, Some(signer))
        .await;

    // Note: Sending this tx takes a very long time (longer than all the other tests combined)
    // because for some reason it takes longer for a signature verification fail to return than it
    // does for other errors. We simulate instead here for testing SPEEEEEED
    let mut ctx = test_f.context.borrow_mut();
    let res = ctx.banks_client.simulate_transaction(tx).await;
    let is_err = res.unwrap().result.unwrap().is_err();

    // Assert the response is an error due to fact that a non-owner of the
    // acount attempted to initialize this account transfer
    assert!(is_err);

    Ok(())
}
