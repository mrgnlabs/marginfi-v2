use fixed_macro::types::I80F48;
use fixtures::{
    assert_custom_error,
    test::{
        BankMint, TestBankSetting, TestFixture, TestSettings, DEFAULT_SOL_TEST_BANK_CONFIG,
        PYTH_SOL_EQUIVALENT_FEED, PYTH_SOL_FEED, PYTH_USDC_FEED,
    },
};
use marginfi::{
    prelude::MarginfiError,
    state::marginfi_group::{BankConfig, BankConfigOpt, BankVaultType, GroupConfig},
};
use solana_program_test::tokio;

#[tokio::test]
/// User deposits $5000 SOLE and $500 USDC, borrowing $990 SOL should fail due to stale oracle
async fn re_one_oracle_stale_failure() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let sol_eq_bank = test_f.get_bank(&BankMint::SolEquivalent);

    // Make SOLE feed stale
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
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 500)
        .await?;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_sol_eq.key, sol_eq_bank, 500)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow_with_nonce(borrower_token_account_f_sol.key, sol_bank, 99, 1)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::RiskEngineInitRejected);

    // Make SOLE feed not stale
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
        .try_bank_borrow_with_nonce(borrower_token_account_f_sol.key, sol_bank, 99, 2)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
/// User deposits $500 of SOLE and $500 of USDC, but SOLE oracle is stale
/// -> borrowing 51 SOL should not succeed ($500 USDC collateral < $510 SOL borrow), but borrowing 40 SOL should go through despite the stale SOLE oracle ($500 USDC collateral > $400 SOL borrow)
async fn re_one_oracle_stale_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let sol_eq_bank = test_f.get_bank(&BankMint::SolEquivalent);

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
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 500)
        .await?;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_sol_eq.key, sol_eq_bank, 500)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 51)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::RiskEngineInitRejected);

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 40)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
/// Borrowing from a bank with a stale oracle should fail
async fn re_one_oracle_stale_failure_2() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);

    test_f.set_time(0);

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
    let borrower_token_account_f_usdc =
        test_f.usdc_mint.create_token_account_and_mint_to(500).await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 500)
        .await?;

    // Make SOL oracle stale
    test_f.set_time(0);
    test_f.set_pyth_oracle_timestamp(PYTH_USDC_FEED, 120).await;
    test_f.set_pyth_oracle_timestamp(PYTH_SOL_FEED, 0).await;
    test_f.advance_time(120).await;

    // Attempt to borrow SOL with stale oracle
    let res = borrower_mfi_account_f
        .try_bank_borrow_with_nonce(borrower_token_account_f_sol.key, sol_bank, 40, 1)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::StaleOracle);

    // Make SOL oracle not stale
    test_f.set_pyth_oracle_timestamp(PYTH_SOL_FEED, 120).await;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow_with_nonce(borrower_token_account_f_sol.key, sol_bank, 40, 2)
        .await;
    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
/// Borrower borrows USDC against SOL, if SOL oracle is stale, the liquidation should fail.
///
/// Liquidator is using SOLE and USDC as collateral, if SOLE oracle is stale and USDC is live,
/// liquidation should succeed as the liquidator has enough USDC collateral.
async fn re_liquidaiton_fail() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                ..Default::default()
            },
            TestBankSetting {
                mint: BankMint::SolEquivalent,
                ..Default::default()
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    asset_weight_maint: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
        protocol_fees: false,
    }))
    .await;

    test_f.set_time(0);

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);
    let sole_bank_f = test_f.get_bank(&BankMint::SolEquivalent);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(2_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 2_000)
        .await?;
    let lender_token_account_sole = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(100)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sole.key, sole_bank_f, 100)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;

    // Borrower deposits 100 SOL worth $1000
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

    // Make borrower asset bank stale
    test_f.set_pyth_oracle_timestamp(PYTH_SOL_FEED, 0).await;
    test_f.set_pyth_oracle_timestamp(PYTH_USDC_FEED, 120).await;
    test_f
        .set_pyth_oracle_timestamp(PYTH_SOL_EQUIVALENT_FEED, 120)
        .await;

    test_f.advance_time(120).await;

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::StaleOracle);

    // Make borrower asset bank not stale
    test_f.set_pyth_oracle_timestamp(PYTH_SOL_FEED, 120).await;
    // Make part of liquidator deposts stale
    test_f
        .set_pyth_oracle_timestamp(PYTH_SOL_EQUIVALENT_FEED, 0)
        .await;

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 2, usdc_bank_f)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn re_bankruptcy_fail() -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings {
        group_config: Some(GroupConfig { admin: None }),
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        protocol_fees: false,
    }))
    .await;

    test_f.set_time(0);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(
            lender_token_account_usdc.key,
            test_f.get_bank(&BankMint::Usdc),
            100_000,
        )
        .await?;

    let borrower_account = test_f.create_marginfi_account().await;
    let borrower_deposit_account = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;

    borrower_account
        .try_bank_deposit(
            borrower_deposit_account.key,
            test_f.get_bank(&BankMint::Sol),
            1_001,
        )
        .await?;

    let borrower_borrow_account = test_f.usdc_mint.create_empty_token_account().await;

    borrower_account
        .try_bank_borrow(
            borrower_borrow_account.key,
            test_f.get_bank(&BankMint::Usdc),
            10_000,
        )
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    borrower_account.set_account(&borrower_mfi_account).await?;

    {
        let (insurance_vault, _) = test_f
            .get_bank(&BankMint::Usdc)
            .get_vault(BankVaultType::Insurance);
        test_f
            .get_bank_mut(&BankMint::Usdc)
            .mint
            .mint_to(&insurance_vault, 10_000)
            .await;
    }

    test_f.set_pyth_oracle_timestamp(PYTH_USDC_FEED, 0).await;
    test_f.advance_time(120).await;

    let res = test_f
        .marginfi_group
        .try_handle_bankruptcy_with_nonce(test_f.get_bank(&BankMint::Usdc), &borrower_account, 1)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::StaleOracle);

    // Make borrower liablity bank not stale
    test_f.set_pyth_oracle_timestamp(PYTH_USDC_FEED, 120).await;

    let res = test_f
        .marginfi_group
        .try_handle_bankruptcy_with_nonce(test_f.get_bank(&BankMint::Usdc), &borrower_account, 2)
        .await;

    assert!(res.is_ok());

    Ok(())
}
