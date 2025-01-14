use anchor_spl::token_2022::spl_token_2022::extension::{
    transfer_fee::TransferFeeConfig, BaseStateWithExtensions,
};
use fixed::types::I80F48;
use fixtures::{assert_custom_error, native, prelude::*, ui_to_native};
use marginfi::{
    assert_eq_with_tolerance,
    prelude::*,
    state::{bank::BankConfigOpt, marginfi_group::BankVaultType},
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
#[test_case(200., 1.1, BankMint::Usdc, BankMint::SolSwbOrigFee)] // Sol @ ~ $153
#[tokio::test]
async fn marginfi_account_borrow_success(
    deposit_amount: f64,
    borrow_amount: f64,
    collateral_mint: BankMint,
    debt_mint: BankMint,
) -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    // LP

    let lp_deposit_amount = 2. * borrow_amount;
    let lp_wallet_balance = get_max_deposit_amount_pre_fee(lp_deposit_amount);
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
            lp_deposit_amount,
        )
        .await?;

    // User

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
        .create_empty_token_account()
        .await;
    let collateral_bank = test_f.get_bank(&collateral_mint);
    user_mfi_account_f
        .try_bank_deposit(
            user_collateral_token_account_f.key,
            collateral_bank,
            deposit_amount,
        )
        .await?;

    // -------------------------------------------------------------------------
    // Test
    // -------------------------------------------------------------------------

    let debt_bank_f = test_f.get_bank(&debt_mint);
    let bank_before = debt_bank_f.load().await;

    let pre_vault_balance = debt_bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;
    let pre_user_debt_accounted = I80F48::ZERO;
    let pre_fee_group_fees: I80F48 = bank_before.collected_group_fees_outstanding.into();
    let pre_fee_program_fees: I80F48 = bank_before.collected_program_fees_outstanding.into();

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
    let post_user_debt_accounted = bank_before
        .get_asset_amount(balance.liability_shares.into())
        .unwrap();

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
    let origination_fee_rate: I80F48 = bank_before
        .config
        .interest_rate_config
        .protocol_origination_fee
        .into();
    let program_fee_rate: I80F48 = test_f
        .marginfi_group
        .load()
        .await
        .fee_state_cache
        .program_fee_rate
        .into();
    let origination_fee: I80F48 = I80F48::from_num(borrow_amount_native)
        .checked_mul(origination_fee_rate)
        .unwrap();
    let program_origination_fee: I80F48 = origination_fee.checked_mul(program_fee_rate).unwrap();
    let group_origination_fee: I80F48 = origination_fee.saturating_sub(program_origination_fee);

    let active_balance_count = marginfi_account
        .lending_account
        .get_active_balances_iter()
        .count();
    assert_eq!(2, active_balance_count);

    let expected_liquidity_vault_delta = -(borrow_amount_pre_fee as i64);
    let actual_liquidity_vault_delta = post_vault_balance as i64 - pre_vault_balance as i64;
    let accounted_user_balance_delta = post_user_debt_accounted - pre_user_debt_accounted;

    // The liquidity vault paid out just the pre-origination fee amount (e.g. what the user borrowed
    // before accounting for the fee)
    assert_eq!(expected_liquidity_vault_delta, actual_liquidity_vault_delta);
    assert_eq_with_tolerance!(
        // Note: the user still gains debt which includes the origination fee
        I80F48::from(expected_liquidity_vault_delta) - origination_fee,
        -accounted_user_balance_delta,
        1
    );

    // The outstanding origination fee is recorded
    let bank_after = debt_bank_f.load().await;
    let post_fee_program_fees: I80F48 = bank_after.collected_program_fees_outstanding.into();
    assert_eq!(
        pre_fee_program_fees + program_origination_fee,
        post_fee_program_fees
    );

    let post_fee_group_fees: I80F48 = bank_after.collected_group_fees_outstanding.into();
    assert_eq!(
        pre_fee_group_fees + group_origination_fee,
        post_fee_group_fees
    );

    Ok(())
}

#[test_case(100., 9., 10.000000001, BankMint::Usdc, BankMint::Sol)]
#[test_case(123_456., 12_345.6, 12_345.9, BankMint::Usdc, BankMint::Sol)]
#[test_case(123_456., 10_000., 15_000., BankMint::UsdcSwb, BankMint::Sol)]
#[test_case(1., 5., 11.98224, BankMint::Sol, BankMint::Usdc)]
#[test_case(128_932., 10_000., 15_000.0, BankMint::PyUSD, BankMint::SolSwb)]
#[test_case(240., 0.092, 500., BankMint::PyUSD, BankMint::T22WithFee)]
#[test_case(36., 1.7, 1.9, BankMint::T22WithFee, BankMint::Sol)]
#[test_case(1., 100., 155.1, BankMint::SolSwbPull, BankMint::Usdc)] // Sol @ ~ $153
#[tokio::test]
async fn marginfi_account_borrow_failure_not_enough_collateral(
    deposit_amount: f64,
    borrow_amount_ok: f64,
    borrow_amount_failed: f64,
    collateral_mint: BankMint,
    debt_mint: BankMint,
) -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    // LP

    let lp_deposit_amount = 2. * borrow_amount_failed;
    let lp_wallet_balance = get_max_deposit_amount_pre_fee(lp_deposit_amount);
    let lp_mfi_account_f = test_f.create_marginfi_account().await;
    let lp_token_account_f_sol = test_f
        .get_bank(&debt_mint)
        .mint
        .create_token_account_and_mint_to(lp_wallet_balance)
        .await;
    lp_mfi_account_f
        .try_bank_deposit(
            lp_token_account_f_sol.key,
            test_f.get_bank(&debt_mint),
            lp_deposit_amount,
        )
        .await?;

    // User

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let user_wallet_balance = get_max_deposit_amount_pre_fee(deposit_amount);
    let borrower_debt_token_account_f = test_f
        .get_bank_mut(&debt_mint)
        .mint
        .create_empty_token_account()
        .await;
    let borrower_collateral_token_account_f = test_f
        .get_bank_mut(&collateral_mint)
        .mint
        .create_token_account_and_mint_to(user_wallet_balance)
        .await;
    let collateral_bank = test_f.get_bank(&collateral_mint);
    borrower_mfi_account_f
        .try_bank_deposit(
            borrower_collateral_token_account_f.key,
            collateral_bank,
            deposit_amount,
        )
        .await?;

    // -------------------------------------------------------------------------
    // Test
    // -------------------------------------------------------------------------

    let debt_bank_f = test_f.get_bank(&debt_mint);

    let res = borrower_mfi_account_f
        .try_bank_borrow(
            borrower_debt_token_account_f.key,
            debt_bank_f,
            borrow_amount_failed,
        )
        .await;
    assert_custom_error!(res.unwrap_err(), MarginfiError::RiskEngineInitRejected);

    let res = borrower_mfi_account_f
        .try_bank_borrow(
            borrower_debt_token_account_f.key,
            debt_bank_f,
            borrow_amount_ok,
        )
        .await;
    assert!(res.is_ok());

    Ok(())
}

#[test_case(505., 500., 505.0000000001, BankMint::Usdc, BankMint::Sol)]
#[test_case(12_345.6, 12_345.5, 12_345.9, BankMint::Usdc, BankMint::Sol)]
#[test_case(11_000., 10_000., 15_000., BankMint::UsdcSwb, BankMint::Sol)]
#[test_case(0.91, 0.1, 0.98, BankMint::Sol, BankMint::Usdc)]
#[test_case(11_000., 10_000., 15_000., BankMint::PyUSD, BankMint::SolSwb)]
#[test_case(505., 0.092, 500., BankMint::PyUSD, BankMint::T22WithFee)]
#[test_case(1.8, 1.7, 1.9, BankMint::T22WithFee, BankMint::Sol)]
#[test_case(1.5, 1.4, 1.6, BankMint::SolSwbPull, BankMint::Usdc)]
#[tokio::test]
async fn marginfi_account_borrow_failure_borrow_limit(
    borrow_cap: f64,
    borrow_amount_ok: f64,
    borrow_amount_failed: f64,
    collateral_mint: BankMint,
    debt_mint: BankMint,
) -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    // LP

    let lp_deposit_amount = 2. * borrow_amount_failed;
    let lp_wallet_balance = get_max_deposit_amount_pre_fee(lp_deposit_amount);
    let lp_mfi_account_f = test_f.create_marginfi_account().await;
    let lp_collateral_token_account = test_f
        .get_bank_mut(&debt_mint)
        .mint
        .create_token_account_and_mint_to(lp_wallet_balance)
        .await;
    lp_mfi_account_f
        .try_bank_deposit(
            lp_collateral_token_account.key,
            test_f.get_bank(&debt_mint),
            lp_deposit_amount,
        )
        .await
        .unwrap();

    // User

    let user_mfi_account_f = test_f.create_marginfi_account().await;
    let sufficient_collateral_amount = test_f
        .get_sufficient_collateral_for_outflow(borrow_amount_failed, &collateral_mint, &debt_mint)
        .await;
    let user_wallet_balance = get_max_deposit_amount_pre_fee(sufficient_collateral_amount);
    let user_collateral_token_account_f = test_f
        .get_bank_mut(&collateral_mint)
        .mint
        .create_token_account_and_mint_to(user_wallet_balance)
        .await;
    let user_debt_token_account_f = test_f
        .get_bank_mut(&debt_mint)
        .mint
        .create_empty_token_account()
        .await;
    user_mfi_account_f
        .try_bank_deposit(
            user_collateral_token_account_f.key,
            test_f.get_bank(&collateral_mint),
            sufficient_collateral_amount,
        )
        .await?;

    // -------------------------------------------------------------------------
    // Test
    // -------------------------------------------------------------------------

    let debt_mint_decimals = test_f.get_bank(&debt_mint).mint.mint.decimals;
    test_f
        .get_bank_mut(&debt_mint)
        .update_config(BankConfigOpt {
            borrow_limit: Some(native!(borrow_cap, debt_mint_decimals, f64)),
            ..Default::default()
        })
        .await?;

    let debt_bank_f = test_f.get_bank(&debt_mint);

    let res = user_mfi_account_f
        .try_bank_borrow(
            user_debt_token_account_f.key,
            debt_bank_f,
            borrow_amount_failed,
        )
        .await;
    assert!(res.is_err());
    assert_custom_error!(
        res.unwrap_err(),
        MarginfiError::BankLiabilityCapacityExceeded
    );

    let res = user_mfi_account_f
        .try_bank_borrow(user_debt_token_account_f.key, debt_bank_f, borrow_amount_ok)
        .await;
    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn isolated_borrows() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_eq_iso_bank = test_f.get_bank(&BankMint::SolEqIsolated);
    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_eq_iso_bank, 1_000)
        .await?;

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
    let borrower_token_account_f_sol = test_f
        .sol_equivalent_mint
        .create_empty_token_account()
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL EQ
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_eq_iso_bank, 10)
        .await;

    assert!(res.is_ok());

    // Repay isolated SOL EQ borrow and borrow SOL successfully,
    let borrower_sol_account = test_f.sol_mint.create_empty_token_account().await;
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_sol_account.key, sol_bank, 10)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::IsolatedAccountIllegalState);

    borrower_mfi_account_f
        .try_bank_repay(
            borrower_token_account_f_sol.key,
            sol_eq_iso_bank,
            0,
            Some(true),
        )
        .await?;

    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_sol_account.key, sol_bank, 10)
        .await;

    assert!(res.is_ok());

    // Borrowing SOL EQ again fails
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_eq_iso_bank, 10)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::IsolatedAccountIllegalState);

    Ok(())
}
