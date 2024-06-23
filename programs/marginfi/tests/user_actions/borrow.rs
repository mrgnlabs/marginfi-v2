use anchor_spl::token_2022::spl_token_2022::extension::{
    transfer_fee::TransferFeeConfig, BaseStateWithExtensions,
};
use fixed::types::I80F48;
use fixtures::{assert_custom_error, native, prelude::*, ui_to_native};
use marginfi::{
    assert_eq_with_tolerance,
    prelude::*,
    state::marginfi_group::{BankConfigOpt, BankVaultType},
};
use pretty_assertions::assert_eq;
use solana_program_test::*;
use test_case::test_case;

#[test_case(100., 9., BankMint::Usdc, BankMint::Sol)]
#[test_case(123456.0, 12345.599999999, BankMint::Usdc, BankMint::Sol)]
#[test_case(123456.0, 10000., BankMint::UsdcSwb, BankMint::Sol)]
#[test_case(1.0, 5.0, BankMint::Sol, BankMint::Usdc)]
#[test_case(128932.0, 9834.0, BankMint::PyUSD, BankMint::SolSwb)]
#[test_case(240., 0.092, BankMint::PyUSD, BankMint::T22WithFee)]
#[test_case(36., 1.7, BankMint::T22WithFee, BankMint::Sol)]
#[tokio::test]
async fn marginfi_account_borrow_success(
    deposit_amount: f64,
    borrow_amount: f64,
    collateral_mint: BankMint,
    debt_mint: BankMint,
) -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    // Fund LP

    let lp_wallet_balance = get_max_deposit_amount_pre_fee(2. * borrow_amount);
    let lp_mfi_account_f = test_f.create_marginfi_account().await;
    let lp_collateral_token_account = test_f
        .get_bank(&debt_mint)
        .mint
        .create_token_account_and_mint_to(lp_wallet_balance)
        .await;
    lp_mfi_account_f
        .try_bank_deposit(
            lp_collateral_token_account.key,
            test_f.get_bank(&debt_mint),
            2. * borrow_amount,
        )
        .await?;

    // Fund user

    let user_mfi_account_f = test_f.create_marginfi_account().await;
    let user_wallet_balance = get_max_deposit_amount_pre_fee(deposit_amount);
    let user_collateral_token_account_f = test_f
        .get_bank_mut(&collateral_mint)
        .mint
        .create_token_account_and_mint_to(user_wallet_balance)
        .await;
    let user_debt_token_account_f = test_f
        .get_bank(&debt_mint)
        .mint
        .create_token_account_and_mint_to(0)
        .await;
    let collateral_bank = test_f.get_bank(&collateral_mint);
    user_mfi_account_f
        .try_bank_deposit(
            user_collateral_token_account_f.key,
            collateral_bank,
            deposit_amount,
        )
        .await?;

    let debt_bank_f = test_f.get_bank(&debt_mint);

    // Borrow

    let pre_vault_balance = debt_bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;
    let pre_user_debt_accounted = I80F48::ZERO;

    let res = user_mfi_account_f
        .try_bank_borrow(user_debt_token_account_f.key, debt_bank_f, borrow_amount)
        .await;
    assert!(res.is_ok());

    let post_vault_balance = debt_bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;
    let marginfi_account = user_mfi_account_f.load().await;
    let balance = marginfi_account
        .lending_account
        .get_balance(&debt_bank_f.key)
        .unwrap();
    let post_user_debt_accounted = debt_bank_f
        .load()
        .await
        .get_asset_amount(balance.liability_shares.into())
        .unwrap();

    // Check state

    let borrow_amount_native = ui_to_native!(borrow_amount, debt_bank_f.mint.mint.decimals);
    let borrow_fee = debt_bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_inverse_epoch_fee(0, borrow_amount_native)
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let borrow_amount_pre_fee = borrow_amount_native + borrow_fee;

    let active_balance_count = marginfi_account
        .lending_account
        .get_active_balances_iter()
        .count();
    assert_eq!(2, active_balance_count);

    let expected_liquidity_vault_delta = borrow_amount_pre_fee;
    let actual_liquidity_vault_delta = pre_vault_balance - post_vault_balance;
    let accounted_liquidity_vault_delta = post_user_debt_accounted - pre_user_debt_accounted;

    assert_eq!(expected_liquidity_vault_delta, actual_liquidity_vault_delta);
    assert_eq_with_tolerance!(
        I80F48::from(expected_liquidity_vault_delta),
        accounted_liquidity_vault_delta,
        1
    );

    Ok(())
}

#[tokio::test]
async fn marginfi_account_borrow_failure_not_enough_collateral() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 101)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::RiskEngineInitRejected);

    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 100)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_account_borrow_failure_borrow_limit() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);

    usdc_bank
        .update_config(BankConfigOpt {
            borrow_limit: Some(native!(1000, "USDC")),
            deposit_limit: Some(native!(10001, "USDC")),
            ..Default::default()
        })
        .await?;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let _owner = test_f.payer();
    let depositor_usdc_account = usdc_bank
        .mint
        .create_token_account_and_mint_to(10_000)
        .await;

    marginfi_account_f
        .try_bank_deposit(depositor_usdc_account.key, usdc_bank, 10_000)
        .await
        .unwrap();

    let borrower = test_f.create_marginfi_account().await;

    let borrower_sol_account = sol_bank
        .mint
        .create_token_account_and_mint_to(100_000)
        .await;

    let borrower_usdc_account = usdc_bank.mint.create_token_account_and_mint_to(0).await;

    borrower
        .try_bank_deposit(borrower_sol_account.key, sol_bank, 1000)
        .await?;

    let res = borrower
        .try_bank_borrow(borrower_usdc_account.key, usdc_bank, 1001)
        .await;

    assert!(res.is_err());
    assert_custom_error!(
        res.unwrap_err(),
        MarginfiError::BankLiabilityCapacityExceeded
    );

    let res = borrower
        .try_bank_borrow(borrower_usdc_account.key, usdc_bank, 999)
        .await;

    assert!(res.is_ok());

    Ok(())
}
