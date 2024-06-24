use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{assert_custom_error, assert_eq_noise, native, prelude::*};
use marginfi::{
    prelude::*,
    state::marginfi_group::{Bank, BankConfig, BankConfigOpt, BankVaultType},
};
use pretty_assertions::assert_eq;
use solana_program_test::*;

#[tokio::test]
async fn marginfi_account_liquidation_success() -> anyhow::Result<()> {
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
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
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
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;

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

#[tokio::test]
async fn marginfi_account_liquidation_success_many_balances() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::many_banks_10())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);
    let sol_eq_bank_f = test_f.get_bank(&BankMint::SolEquivalent);
    let sol_eq1_bank_f = test_f.get_bank(&BankMint::SolEquivalent1);
    let sol_eq2_bank_f = test_f.get_bank(&BankMint::SolEquivalent2);
    let sol_eq3_bank_f = test_f.get_bank(&BankMint::SolEquivalent3);
    let sol_eq4_bank_f = test_f.get_bank(&BankMint::SolEquivalent4);
    let sol_eq5_bank_f = test_f.get_bank(&BankMint::SolEquivalent5);
    let sol_eq6_bank_f = test_f.get_bank(&BankMint::SolEquivalent6);
    let sol_eq7_bank_f = test_f.get_bank(&BankMint::SolEquivalent7);

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
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    let borrower_token_account_sol_eq = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(5000)
        .await;

    // Borrower deposits 100 SOL worth of $1000
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 100)
        .await?;

    // Borrower borrows $999
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 999)
        .await?;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq1_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq2_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq3_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq4_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq5_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq6_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq7_bank_f, 0)
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

#[tokio::test]
async fn marginfi_account_liquidation_success_swb() -> anyhow::Result<()> {
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
                    ..*DEFAULT_SOL_TEST_SW_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
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
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;

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
        native!(0.01, "USDC", f64)
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
        native!(0.01, "USDC", f64)
    );

    // Check insurance fund fee
    let insurance_fund_usdc = usdc_bank_f
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq_noise!(
        insurance_fund_usdc.balance().await as i64,
        native!(0.25, "USDC", f64) as i64,
        native!(0.001, "USDC", f64) as i64
    );

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_failure_liquidatee_not_unhealthy() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                config: Some(BankConfig {
                    asset_weight_maint: I80F48!(1).into(),
                    ..*DEFAULT_USDC_TEST_BANK_CONFIG
                }),
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
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 200)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 100)
        .await?;

    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 100)
        .await?;

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await;

    assert!(res.is_err());

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_failure_liquidation_too_severe() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 200)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(10).await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 10)
        .await?;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 61)
        .await?;

    sol_bank_f
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.25).into()),
            asset_weight_maint: Some(I80F48!(0.5).into()),
            ..Default::default()
        })
        .await?;

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 10, usdc_bank_f)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_failure_liquidator_no_collateral() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                config: Some(BankConfig {
                    liability_weight_init: I80F48!(1.2).into(),
                    liability_weight_maint: I80F48!(1.1).into(),
                    ..*DEFAULT_USDC_TEST_BANK_CONFIG
                }),
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::SolEquivalent,
                config: None,
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);
    let sol_eq_bank_f = test_f.get_bank(&BankMint::SolEquivalent);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 200)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_sol_eq = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(100)
        .await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 10)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq_bank_f, 1)
        .await?;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 60)
        .await?;

    sol_bank_f
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.25).into()),
            asset_weight_maint: Some(I80F48!(0.3).into()),
            ..Default::default()
        })
        .await?;

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_eq_bank_f, 2, usdc_bank_f)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_eq_bank_f, 1, usdc_bank_f)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_failure_bank_not_liquidatable() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);
    let sol_eq_bank_f = test_f.get_bank(&BankMint::SolEquivalent);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 200)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_sol_eq = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(100)
        .await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 10)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq_bank_f, 1)
        .await?;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 60)
        .await?;

    sol_bank_f
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.25).into()),
            asset_weight_maint: Some(I80F48!(0.4).into()),
            ..Default::default()
        })
        .await?;

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_eq_bank_f, 1, sol_bank_f)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await;

    assert!(res.is_ok());

    Ok(())
}
