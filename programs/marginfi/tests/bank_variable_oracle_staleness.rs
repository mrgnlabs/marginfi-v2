use fixtures::{
    assert_custom_error,
    test::{
        BankMint, TestFixture, TestSettings, PYTH_SOL_EQUIVALENT_FEED, PYTH_SOL_FEED,
        PYTH_USDC_FEED,
    },
};
use marginfi::{prelude::MarginfiError, state::marginfi_group::BankConfigOpt};
use solana_program_test::tokio;

#[tokio::test]
/// Borrowing with deposits to a non isolated stale bank should error
async fn bank_oracle_staleness_test() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);
    let sol_eq_bank = test_f.get_bank(&BankMint::SolEquivalent);

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
    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 500)
        .await?;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_sol_eq.key, sol_eq_bank, 500)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 99)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::BadAccountHealth);

    test_f.set_pyth_oracle_timestamp(PYTH_SOL_FEED, 200).await;
    test_f.set_pyth_oracle_timestamp(PYTH_USDC_FEED, 200).await;
    test_f
        .set_pyth_oracle_timestamp(PYTH_SOL_EQUIVALENT_FEED, 200)
        .await;
    test_f.advance_time(80).await;

    usdc_bank
        .update_config(BankConfigOpt {
            oracle_max_age: Some(200),
            ..Default::default()
        })
        .await?;

    sol_bank
        .update_config(BankConfigOpt {
            oracle_max_age: Some(200),
            ..Default::default()
        })
        .await?;

    sol_eq_bank
        .update_config(BankConfigOpt {
            oracle_max_age: Some(200),
            ..Default::default()
        })
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 98)
        .await;

    assert!(res.is_ok());

    Ok(())
}
