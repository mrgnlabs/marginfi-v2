use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{
    assert_custom_error, assert_eq_noise, native,
    test::{
        BankMint, TestBankSetting, TestFixture, TestSettings,
        DEFAULT_SOL_TEST_PYTH_PUSH_FULLV_BANK_CONFIG, DEFAULT_SOL_TEST_PYTH_PUSH_PARTV_BANK_CONFIG,
        DEFAULT_USDC_TEST_BANK_CONFIG,
    },
};
use marginfi::{
    errors::MarginfiError,
    state::marginfi_group::{Bank, BankConfig, BankConfigOpt, BankVaultType, GroupConfig},
};
use solana_program_test::tokio;

#[tokio::test]
async fn pyth_push_fullv_borrow() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                config: Some(*DEFAULT_USDC_TEST_BANK_CONFIG),
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(*DEFAULT_SOL_TEST_PYTH_PUSH_FULLV_BANK_CONFIG),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
        protocol_fees: false,
    }))
    .await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);

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
async fn pyth_push_partv_borrow() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                config: Some(*DEFAULT_USDC_TEST_BANK_CONFIG),
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(*DEFAULT_SOL_TEST_PYTH_PUSH_PARTV_BANK_CONFIG),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
        protocol_fees: false,
    }))
    .await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);

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

    // TODO: Bad error, need to improve the flexible risk engine logic to correctly pass the
    // unerlying error.
    assert_custom_error!(res.unwrap_err(), MarginfiError::StaleOracle);

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 100)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::StaleOracle);

    Ok(())
}

#[tokio::test]
async fn pyth_push_fullv_liquidate() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                ..TestBankSetting::default()
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    asset_weight_maint: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_PYTH_PUSH_FULLV_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
        protocol_fees: false,
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(2_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 2_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;

    // Borrower deposits 100 SOL worth of $1000
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 100)
        .await?;

    // Borrower borrows $999
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 999)
        .await?;

    // Synthetically bring down the borrower account health by reducing the asset weights of the SOL bank
    sol_bank_f
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.25).into()),
            asset_weight_maint: Some(I80F48!(0.5).into()),
            ..Default::default()
        })
        .await?;

    lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await?;

    // Checks
    let sol_bank: Bank = sol_bank_f.load().await;
    let usdc_bank: Bank = usdc_bank_f.load().await;

    let depositor_ma = lender_mfi_account_f.load().await;
    let borrower_ma = borrower_mfi_account_f.load().await;

    // Depositors should have 1 SOL
    assert_eq!(
        sol_bank
            .get_asset_amount(depositor_ma.lending_account.balances[1].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(1, "SOL"))
    );

    // Depositors should have 1990.25 USDC
    assert_eq_noise!(
        usdc_bank
            .get_asset_amount(depositor_ma.lending_account.balances[0].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(1990.25, "USDC", f64)),
        native!(0.00001, "USDC", f64)
    );

    // Borrower should have 99 SOL
    assert_eq!(
        sol_bank
            .get_asset_amount(borrower_ma.lending_account.balances[0].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(99, "SOL"))
    );

    // Borrower should have 989.50 USDC
    assert_eq_noise!(
        usdc_bank
            .get_liability_amount(
                borrower_ma.lending_account.balances[1]
                    .liability_shares
                    .into()
            )
            .unwrap(),
        I80F48::from(native!(989.50, "USDC", f64)),
        native!(0.00001, "USDC", f64)
    );

    // Check insurance fund fee
    let insurance_fund_usdc = usdc_bank_f
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq_noise!(
        insurance_fund_usdc.balance().await as i64,
        native!(0.25, "USDC", f64) as i64,
        1
    );

    Ok(())
}
