use fixtures::{
    assert_custom_error,
    test::{
        BankMint, TestFixture, TestSettings, PYTH_SOL_EQUIVALENT_FEED, PYTH_SOL_FEED,
        PYTH_USDC_FEED,
    },
};
use marginfi::prelude::MarginfiError;
use solana_program_test::tokio;

#[tokio::test]
/// Borrowing with deposits in two banks (1 stale, 1 non-stale) should error with bad health
/// Account has enough total collateral to borrow, but one bank is stale and the non-stale collateral left
/// is not sufficient, so the borrow fails with bad health
async fn stale_bank_should_error() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let sol_eq_bank = test_f.get_bank(&BankMint::SolEquivalent);

    // Make SOLE feed stale
    test_f.set_time(0);
    test_f.set_pyth_oracle_timestamp(PYTH_USDC_FEED, 120).await;
    test_f
        .set_pyth_oracle_timestamp(PYTH_SOL_EQUIVALENT_FEED, 0)
        .await;
    test_f.set_pyth_oracle_timestamp(PYTH_SOL_FEED, 120).await;
    test_f.advance_time(120).await;

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
    let borrower_token_account_f_sol_eq = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 500)
        .await?;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_sol_eq.key, sol_eq_bank, 100)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 99)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::RiskEngineInitRejected);

    Ok(())
}

#[tokio::test]
/// Borrowing with deposits in two banks (1 stale) should not error if the non-stale collateral is sufficient
async fn non_stale_bank_should_error() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_eq_bank = test_f.get_bank(&BankMint::SolEquivalent);
    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Make USDC feed stale
    test_f.set_time(0);
    test_f.set_pyth_oracle_timestamp(PYTH_USDC_FEED, 0).await;
    test_f.set_pyth_oracle_timestamp(PYTH_SOL_FEED, 120).await;
    test_f
        .set_pyth_oracle_timestamp(PYTH_SOL_EQUIVALENT_FEED, 120)
        .await;
    test_f.advance_time(120).await;

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
    let borrower_token_account_f_sol_eq = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 15)
        .await?;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_sol_eq.key, sol_eq_bank, 100)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 99)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
/// Borrowing with deposits to a non isolated stale bank should error
async fn isolated_stale_should_not_error() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let sol_eq_iso_bank = test_f.get_bank(&BankMint::SolEqIsolated);

    test_f.set_time(0);
    test_f.set_pyth_oracle_timestamp(PYTH_SOL_FEED, 120).await;
    test_f.set_pyth_oracle_timestamp(PYTH_USDC_FEED, 120).await;
    test_f.advance_time(120).await;

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
    let borrower_token_account_f_sol_eq = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_sol_eq.key, sol_eq_iso_bank, 1_000)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 99)
        .await;

    assert!(res.is_ok());

    Ok(())
}
