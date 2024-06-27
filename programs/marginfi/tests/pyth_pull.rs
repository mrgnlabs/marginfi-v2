use fixtures::{
    assert_custom_error, native,
    test::{BankMint, TestFixture, TestSettings},
};
use marginfi::errors::MarginfiError;
use solana_program_test::tokio;

#[tokio::test]
async fn pyth_pull_fullv_borrow() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(
        TestSettings::all_banks_pyth_pull_fullv_payer_not_admin(),
    ))
    .await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

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

    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 101)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::RiskEngineInitRejected);

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 100)
        .await;

    assert!(res.is_ok());

    // Check token balances are correct
    assert_eq!(
        borrower_token_account_f_usdc.balance().await,
        native!(0, "USDC")
    );

    assert_eq!(
        borrower_token_account_f_sol.balance().await,
        native!(100, "SOL")
    );

    Ok(())
}

#[tokio::test]
async fn pyth_pull_partv_borrow() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(
        TestSettings::all_banks_pyth_pull_partv_payer_not_admin(),
    ))
    .await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

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

    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 101)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::StaleOracle);

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 100)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::StaleOracle);

    Ok(())
}
