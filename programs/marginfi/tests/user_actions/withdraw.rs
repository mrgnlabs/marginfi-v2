use anchor_spl::token_2022::spl_token_2022::extension::{
    transfer_fee::TransferFeeConfig, BaseStateWithExtensions,
};
use fixed::types::I80F48;
use fixtures::{assert_custom_error, prelude::*, ui_to_native};
use marginfi::{assert_eq_with_tolerance, prelude::*, state::marginfi_group::BankVaultType};
use pretty_assertions::assert_eq;
use solana_program_test::*;
use test_case::test_case;

#[test_case(0.03, 0.012, BankMint::Usdc)]
#[test_case(100.0, 100.0, BankMint::UsdcSwb)]
#[test_case(100.0, 100.0, BankMint::SolSwb)]
#[test_case(128932.0, 9834.0, BankMint::PyUSD)]
#[test_case(0.1, 0.092, BankMint::T22WithFee)]
#[test_case(100.0, 92.0, BankMint::T22WithFee)]
#[tokio::test]
async fn marginfi_account_withdraw_success(
    deposit_amount: f64,
    withdraw_amount: f64,
    bank_mint: BankMint,
) -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    // User

    let marginfi_account_f = test_f.create_marginfi_account().await;
    let user_wallet_balance = get_max_deposit_amount_pre_fee(deposit_amount);
    let token_account_f = TokenAccountFixture::new(
        test_f.context.clone(),
        &test_f.get_bank(&bank_mint).mint,
        &test_f.payer(),
    )
    .await;
    test_f
        .get_bank_mut(&bank_mint)
        .mint
        .mint_to(&token_account_f.key, user_wallet_balance)
        .await;
    let bank_f = test_f.get_bank(&bank_mint);
    marginfi_account_f
        .try_bank_deposit(token_account_f.key, bank_f, deposit_amount)
        .await
        .unwrap();

    // -------------------------------------------------------------------------
    // Test
    // -------------------------------------------------------------------------

    let marginfi_account = marginfi_account_f.load().await;
    let pre_vault_balance = bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;
    let balance = marginfi_account
        .lending_account
        .get_balance(&bank_f.key)
        .unwrap();
    let pre_accounted = bank_f
        .load()
        .await
        .get_asset_amount(balance.asset_shares.into())
        .unwrap();

    let deposit_amount_native = ui_to_native!(deposit_amount, bank_f.mint.mint.decimals);
    let withdraw_amount_native = ui_to_native!(withdraw_amount, bank_f.mint.mint.decimals);
    let withdraw_fee_to_use;
    let (withdraw_fee, withdraw_fee_if_excessive) = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            (
                // withdraw <= available case
                tf.calculate_inverse_epoch_fee(0, withdraw_amount_native)
                    .unwrap_or(0),
                // withdraw all case, if withdraw > available
                tf.calculate_epoch_fee(0, deposit_amount_native)
                    .unwrap_or(0),
            )
        })
        .unwrap_or((0, 0));

    // If exceeds available, clamp to available.
    // If it does not, use specified withdraw amount
    let adjusted_withdraw_amount = if withdraw_amount_native + withdraw_fee > deposit_amount_native
    {
        // Clamp to deposit amount minus fee if excessive
        withdraw_fee_to_use = withdraw_fee_if_excessive;
        deposit_amount
            - withdraw_fee_if_excessive as f64 / 10_f64.powi(bank_f.mint.mint.decimals as i32)
    } else {
        // Use specified withdraw amount
        withdraw_fee_to_use = withdraw_fee;
        withdraw_amount
    };

    let res = marginfi_account_f
        .try_bank_withdraw(token_account_f.key, bank_f, adjusted_withdraw_amount, None)
        .await;
    assert!(res.is_ok());

    let post_vault_balance = bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;
    let marginfi_account = marginfi_account_f.load().await;
    let balance = marginfi_account
        .lending_account
        .get_balance(&bank_f.key)
        .unwrap();
    let post_accounted = bank_f
        .load()
        .await
        .get_asset_amount(balance.asset_shares.into())
        .unwrap();

    let active_balance_count = marginfi_account
        .lending_account
        .get_active_balances_iter()
        .count();
    assert_eq!(1, active_balance_count);

    let expected_liquidity_vault_delta = -I80F48::from(
        ui_to_native!(adjusted_withdraw_amount, bank_f.mint.mint.decimals) + withdraw_fee_to_use,
    );
    let actual_liquidity_vault_delta =
        I80F48::from(post_vault_balance) - I80F48::from(pre_vault_balance);

    let accounted_user_balance_delta = post_accounted - pre_accounted;

    assert_eq!(expected_liquidity_vault_delta, actual_liquidity_vault_delta);
    assert_eq_with_tolerance!(
        expected_liquidity_vault_delta,
        accounted_user_balance_delta,
        1
    );

    Ok(())
}

#[test_case(0.03, BankMint::Usdc)]
#[test_case(100.0, BankMint::Usdc)]
#[test_case(100.0, BankMint::Sol)]
#[test_case(128932.0, BankMint::PyUSD)]
#[test_case(0.1, BankMint::T22WithFee)]
#[test_case(100.0, BankMint::T22WithFee)]
#[tokio::test]
async fn marginfi_account_withdraw_all_success(
    deposit_amount: f64,
    bank_mint: BankMint,
) -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    // User

    let marginfi_account_f = test_f.create_marginfi_account().await;
    let user_wallet_balance = get_max_deposit_amount_pre_fee(deposit_amount);
    let token_account_f = TokenAccountFixture::new(
        test_f.context.clone(),
        &test_f.get_bank(&bank_mint).mint,
        &test_f.payer(),
    )
    .await;
    test_f
        .get_bank_mut(&bank_mint)
        .mint
        .mint_to(&token_account_f.key, user_wallet_balance)
        .await;

    // -------------------------------------------------------------------------
    // Test
    // -------------------------------------------------------------------------

    let bank_f = test_f.get_bank(&bank_mint);

    marginfi_account_f
        .try_bank_deposit(token_account_f.key, bank_f, deposit_amount)
        .await
        .unwrap();

    let marginfi_account = marginfi_account_f.load().await;
    let pre_vault_balance = bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;
    let balance = marginfi_account
        .lending_account
        .get_balance(&bank_f.key)
        .unwrap();
    let pre_accounted = bank_f
        .load()
        .await
        .get_asset_amount(balance.asset_shares.into())
        .unwrap();

    let res = marginfi_account_f
        .try_bank_withdraw(token_account_f.key, bank_f, 0, Some(true))
        .await;
    assert!(res.is_ok());

    let marginfi_account = marginfi_account_f.load().await;

    let active_balance_count = marginfi_account
        .lending_account
        .get_active_balances_iter()
        .count();
    assert_eq!(0, active_balance_count);

    let post_vault_balance = bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;
    assert!(marginfi_account
        .lending_account
        .get_balance(&bank_f.key)
        .is_none());
    let post_accounted = I80F48::ZERO;

    let deposit_amount_native = ui_to_native!(deposit_amount, bank_f.mint.mint.decimals);

    let expected_liquidity_vault_delta = -I80F48::from(deposit_amount_native);
    let actual_liquidity_vault_delta =
        I80F48::from(post_vault_balance) - I80F48::from(pre_vault_balance);
    let accounted_user_balance_delta = post_accounted - pre_accounted;

    assert_eq!(expected_liquidity_vault_delta, actual_liquidity_vault_delta);
    assert_eq_with_tolerance!(
        expected_liquidity_vault_delta,
        accounted_user_balance_delta,
        1
    );

    Ok(())
}

#[test_case(0.03, 0.030001, BankMint::Usdc)]
#[test_case(100., 101., BankMint::UsdcSwb)]
#[test_case(100., 102., BankMint::Sol)]
#[test_case(100., 102., BankMint::SolSwb)]
#[test_case(109247394., 109247394.000001, BankMint::PyUSD)]
#[test_case(16., 16., BankMint::T22WithFee)]
#[test_case(100., 98., BankMint::T22WithFee)]
#[tokio::test]
async fn marginfi_account_withdraw_failure_withdrawing_too_much(
    deposit_amount: f64,
    withdraw_amount: f64,
    bank_mint: BankMint,
) -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    // User

    let marginfi_account_f = test_f.create_marginfi_account().await;
    let user_wallet_balance = get_max_deposit_amount_pre_fee(deposit_amount);
    let token_account_f = TokenAccountFixture::new(
        test_f.context.clone(),
        &test_f.get_bank(&bank_mint).mint,
        &test_f.payer(),
    )
    .await;
    test_f
        .get_bank_mut(&bank_mint)
        .mint
        .mint_to(&token_account_f.key, user_wallet_balance)
        .await;

    // -------------------------------------------------------------------------
    // Test
    // -------------------------------------------------------------------------

    let bank_f = test_f.get_bank(&bank_mint);

    marginfi_account_f
        .try_bank_deposit(token_account_f.key, bank_f, deposit_amount)
        .await?;

    let res = marginfi_account_f
        .try_bank_withdraw(token_account_f.key, bank_f, withdraw_amount, None)
        .await;
    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::OperationWithdrawOnly);

    Ok(())
}
