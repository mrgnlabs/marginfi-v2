use fixtures::{
    assert_custom_error, native,
    test::{BankMint, TestFixture, TestSettings},
};
use marginfi::errors::MarginfiError;
use solana_program_test::tokio;

#[tokio::test]
async fn real_oracle_marginfi_account_borrow_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::real_oracle_data())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    test_f.set_time(1720094628);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 9)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::RiskEngineInitRejected);

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 7)
        .await;

    assert!(res.is_ok());

    // Check token balances are correct
    assert_eq!(
        borrower_token_account_f_usdc.balance().await,
        native!(0, "USDC")
    );

    assert_eq!(
        borrower_token_account_f_sol.balance().await,
        native!(7, "SOL")
    );

    // TODO: check health is sane

    Ok(())
}
