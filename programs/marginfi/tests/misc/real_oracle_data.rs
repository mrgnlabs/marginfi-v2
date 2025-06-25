use fixtures::{
    assert_custom_error, native,
    test::{
        BankMint, TestBankSetting, TestFixture, TestSettings,
        DEFAULT_PYTH_PUSH_SOL_TEST_REAL_BANK_CONFIG, DEFAULT_USDC_TEST_REAL_BANK_CONFIG,
    },
};
use marginfi::errors::MarginfiError;
use solana_program_test::tokio;
#[tokio::test]
async fn real_oracle_pyth_push_marginfi_account_borrow_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                config: Some(*DEFAULT_USDC_TEST_REAL_BANK_CONFIG),
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(*DEFAULT_PYTH_PUSH_SOL_TEST_REAL_BANK_CONFIG),
            },
        ],
        protocol_fees: false,
    }))
    .await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);

    test_f.set_time(1720094628);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank, 1_000, None)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000, None)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 7)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::RiskEngineInitRejected);

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 6)
        .await;

    assert!(res.is_ok());

    // Check token balances are correct
    assert_eq!(
        borrower_token_account_f_usdc.balance().await,
        native!(0, "USDC")
    );

    assert_eq!(
        borrower_token_account_f_sol.balance().await,
        native!(6, "SOL")
    );

    Ok(())
}
