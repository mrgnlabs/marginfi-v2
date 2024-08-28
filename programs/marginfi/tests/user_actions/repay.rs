use anchor_spl::token_2022::spl_token_2022::extension::{
    transfer_fee::TransferFeeConfig, BaseStateWithExtensions,
};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{assert_custom_error, native, prelude::*};
use marginfi::{assert_eq_with_tolerance, prelude::*, state::marginfi_group::BankVaultType};
use pretty_assertions::assert_eq;
use solana_program_test::*;
use test_case::test_case;

#[test_case(100., 9., BankMint::Usdc, BankMint::Sol)]
#[test_case(123456., 12345.599999999, BankMint::Usdc, BankMint::Sol)]
#[test_case(123456., 10000., BankMint::UsdcSwb, BankMint::Sol)]
#[test_case(1., 1., BankMint::Sol, BankMint::Usdc)]
#[test_case(128932., 9834., BankMint::PyUSD, BankMint::SolSwb)]
#[test_case(240., 0.092, BankMint::PyUSD, BankMint::T22WithFee)]
#[test_case(36., 20., BankMint::T22WithFee, BankMint::Sol)]
#[test_case(200., 1.1, BankMint::Usdc, BankMint::SolSwbOrigFee)] // Sol @ ~ $153
#[tokio::test]
async fn marginfi_account_repay_success(
    borrow_amount: f64,
    repay_amount: f64,
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
    let sufficient_collateral_amount = test_f
        .get_sufficient_collateral_for_outflow(borrow_amount, &collateral_mint, &debt_mint)
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

    let debt_bank = test_f.get_bank(&debt_mint);

    user_mfi_account_f
        .try_bank_borrow(user_debt_token_account_f.key, debt_bank, borrow_amount)
        .await?;

    let pre_vault_balance = debt_bank
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;
    let marginfi_account = user_mfi_account_f.load().await;
    let balance = marginfi_account
        .lending_account
        .get_balance(&debt_bank.key)
        .unwrap();
    let pre_accounted = debt_bank
        .load()
        .await
        .get_asset_amount(balance.liability_shares.into())
        .unwrap();

    let res = user_mfi_account_f
        .try_bank_repay(user_debt_token_account_f.key, debt_bank, repay_amount, None)
        .await;
    assert!(res.is_ok());

    let post_vault_balance = debt_bank
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;

    let marginfi_account = user_mfi_account_f.load().await;
    let balance = marginfi_account
        .lending_account
        .get_balance(&debt_bank.key)
        .unwrap();
    let post_accounted = debt_bank
        .load()
        .await
        .get_asset_amount(balance.liability_shares.into())
        .unwrap();

    let expected_liquidity_vault_delta =
        I80F48::from(native!(repay_amount, debt_bank.mint.mint.decimals, f64));
    let actual_liquidity_vault_delta = I80F48::from(post_vault_balance - pre_vault_balance);
    let accounted_user_balance_delta = post_accounted - pre_accounted;

    assert_eq!(expected_liquidity_vault_delta, actual_liquidity_vault_delta);
    assert_eq_with_tolerance!(
        expected_liquidity_vault_delta,
        -accounted_user_balance_delta,
        1
    );

    Ok(())
}

#[test_case(100., BankMint::Usdc, BankMint::Sol)]
#[test_case(123456., BankMint::Usdc, BankMint::Sol)]
#[test_case(123456., BankMint::UsdcSwb, BankMint::Sol)]
#[test_case(1., BankMint::Sol, BankMint::Usdc)]
#[test_case(128932., BankMint::PyUSD, BankMint::SolSwb)]
#[test_case(240., BankMint::PyUSD, BankMint::T22WithFee)]
#[test_case(36., BankMint::T22WithFee, BankMint::Sol)]
#[test_case(200., BankMint::Usdc, BankMint::SolSwbOrigFee)] // Sol @ ~ $153
#[tokio::test]
async fn marginfi_account_repay_all_success(
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
    let sufficient_collateral_amount = test_f
        .get_sufficient_collateral_for_outflow(borrow_amount, &collateral_mint, &debt_mint)
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
        .create_token_account_and_mint_to(2. * borrow_amount + 1.) // to ensure user has enough to repay given interest and fees taken during borrow and repay
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

    let debt_bank = test_f.get_bank(&debt_mint);

    user_mfi_account_f
        .try_bank_borrow(user_debt_token_account_f.key, debt_bank, borrow_amount)
        .await
        .unwrap();

    let (pre_vault_balance, pre_accounted_vault_balance) = {
        let pre_vault_balance = debt_bank
            .get_vault_token_account(BankVaultType::Liquidity)
            .await
            .balance()
            .await;
        let marginfi_account = user_mfi_account_f.load().await;
        let balance = marginfi_account
            .lending_account
            .get_balance(&debt_bank.key)
            .unwrap();
        let pre_accounted_vault_balance = debt_bank
            .load()
            .await
            .get_asset_amount(balance.liability_shares.into())
            .unwrap();

        (pre_vault_balance, pre_accounted_vault_balance)
    };

    let res = user_mfi_account_f
        .try_bank_repay(user_debt_token_account_f.key, debt_bank, 0, Some(true))
        .await;
    assert!(res.is_ok());

    let (post_vault_balance, post_accounted_vault_balance) = {
        let post_vault_balance = debt_bank
            .get_vault_token_account(BankVaultType::Liquidity)
            .await
            .balance()
            .await;
        let marginfi_account = user_mfi_account_f.load().await;
        let balance = marginfi_account.lending_account.get_balance(&debt_bank.key);
        assert!(balance.is_none());
        let post_accounted_vault_balance = I80F48!(0);

        (post_vault_balance, post_accounted_vault_balance)
    };

    let borrow_fee = debt_bank
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_inverse_epoch_fee(
                0,
                native!(borrow_amount, debt_bank.mint.mint.decimals, f64),
            )
            .unwrap_or(0)
        })
        .unwrap_or(0);

    let origination_fee_rate: I80F48 = debt_bank
        .load()
        .await
        .config
        .interest_rate_config
        .protocol_origination_fee
        .into();
    let origination_fee: I80F48 =
        I80F48::from_num(native!(borrow_amount, debt_bank.mint.mint.decimals, f64))
            .checked_mul(origination_fee_rate)
            .unwrap()
            .ceil(); // Round up when repaying
    let origination_fee_u64: u64 = origination_fee.checked_to_num().expect("out of bounds");

    let expected_liquidity_delta = I80F48::from(
        native!(borrow_amount, debt_bank.mint.mint.decimals, f64)
            + borrow_fee
            + origination_fee_u64,
    );
    let actual_liquidity_delta = I80F48::from(post_vault_balance) - I80F48::from(pre_vault_balance);
    let accounted_liquidity_delta = post_accounted_vault_balance - pre_accounted_vault_balance;

    assert_eq!(expected_liquidity_delta, actual_liquidity_delta);
    assert_eq_with_tolerance!(expected_liquidity_delta, -accounted_liquidity_delta, 1);

    Ok(())
}

#[test_case(100., 110., BankMint::Usdc, BankMint::Sol)]
#[test_case(123456., 123457., BankMint::Usdc, BankMint::Sol)]
#[test_case(3000., 10000., BankMint::UsdcSwb, BankMint::Sol)]
#[test_case(1., 1.000002, BankMint::Sol, BankMint::Usdc)]
#[test_case(9834., 234749., BankMint::PyUSD, BankMint::SolSwb)]
#[test_case(0.092, 240., BankMint::PyUSD, BankMint::T22WithFee)]
#[test_case(1.7, 36., BankMint::T22WithFee, BankMint::Sol)]
#[tokio::test]
async fn marginfi_account_repay_failure_repaying_too_much(
    borrow_amount: f64,
    repay_amount: f64,
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
    let sufficient_collateral_amount = test_f
        .get_sufficient_collateral_for_outflow(borrow_amount, &collateral_mint, &debt_mint)
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
        .create_token_account_and_mint_to(2. * borrow_amount + 1.) // to ensure user has enough to repay given interest and fees taken during borrow and repay
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

    let debt_bank = test_f.get_bank(&debt_mint);

    user_mfi_account_f
        .try_bank_borrow(user_debt_token_account_f.key, debt_bank, borrow_amount)
        .await?;

    let res = user_mfi_account_f
        .try_bank_repay(user_debt_token_account_f.key, debt_bank, repay_amount, None)
        .await;
    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::OperationRepayOnly);

    Ok(())
}
