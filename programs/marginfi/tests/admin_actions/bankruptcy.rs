use anchor_spl::token_2022::spl_token_2022::extension::{
    transfer_fee::{TransferFeeConfig, MAX_FEE_BASIS_POINTS},
    BaseStateWithExtensions,
};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{assert_custom_error, assert_eq_noise, native, prelude::*};
use marginfi::{
    prelude::{GroupConfig, MarginfiError},
    state::marginfi_group::{BankConfig, BankVaultType},
};
use pretty_assertions::assert_eq;
use solana_program_test::*;
use test_case::test_case;

#[test_case(BankMint::Usdc, BankMint::Sol)]
#[test_case(BankMint::UsdcSwb, BankMint::Sol)]
#[test_case(BankMint::Sol, BankMint::Usdc)]
#[test_case(BankMint::PyUSD, BankMint::SolSwb)]
#[test_case(BankMint::PyUSD, BankMint::T22WithFee)]
#[test_case(BankMint::T22WithFee, BankMint::Sol)]
#[tokio::test]
async fn marginfi_group_handle_bankruptcy_failure_not_bankrupt(
    collateral_mint: BankMint,
    debt_mint: BankMint,
) -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let borrow_amount = 10_000.;

    // LP

    let lp_deposit_amount = 2. * borrow_amount;
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
    user_mfi_account_f
        .try_bank_borrow(
            user_debt_token_account_f.key,
            test_f.get_bank(&debt_mint),
            borrow_amount,
        )
        .await?;

    // -------------------------------------------------------------------------
    // Test
    // -------------------------------------------------------------------------

    let debt_bank_f = test_f.get_bank(&debt_mint);

    let res = test_f
        .marginfi_group
        .try_handle_bankruptcy(debt_bank_f, &user_mfi_account_f)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountNotBankrupt);

    Ok(())
}

#[test_case(BankMint::Usdc, BankMint::Sol)]
#[test_case(BankMint::UsdcSwb, BankMint::Sol)]
#[test_case(BankMint::Sol, BankMint::Usdc)]
#[test_case(BankMint::PyUSD, BankMint::SolSwb)]
#[test_case(BankMint::PyUSD, BankMint::T22WithFee)]
#[test_case(BankMint::T22WithFee, BankMint::Sol)]
#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_no_debt(
    collateral_mint: BankMint,
    debt_mint: BankMint,
) -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let borrow_amount = 10_000.;

    // LP

    let lp_deposit_amount = 2. * borrow_amount;
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
    user_mfi_account_f
        .try_bank_borrow(
            user_debt_token_account_f.key,
            test_f.get_bank(&debt_mint),
            borrow_amount,
        )
        .await?;

    // -------------------------------------------------------------------------
    // Test
    // -------------------------------------------------------------------------

    let debt_bank_f = test_f.get_bank(&debt_mint);

    // Artificially nullify the collateral to place the account in a bankrupt state
    let mut user_mfi_account = user_mfi_account_f.load().await;
    user_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    user_mfi_account_f.set_account(&user_mfi_account).await?;

    let res = test_f
        .marginfi_group
        .try_handle_bankruptcy(debt_bank_f, &user_mfi_account_f)
        .await;
    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_no_debt_old() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
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
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 1_001)
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 10_000)
        .await?;

    let mut borrower_mfi_account = borrower_mfi_account_f.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    borrower_mfi_account_f
        .set_account(&borrower_mfi_account)
        .await?;

    let res = test_f
        .marginfi_group
        .try_handle_bankruptcy(sol_bank_f, &borrower_mfi_account_f)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::BalanceNotBadDebt);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_fully_insured() -> anyhow::Result<()> {
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
    }))
    .await;

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

    test_f
        .marginfi_group
        .try_handle_bankruptcy(test_f.get_bank(&BankMint::Usdc), &borrower_account)
        .await?;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let usdc_bank = test_f.get_bank(&BankMint::Usdc).load().await;

    let lender_usdc_value = usdc_bank.get_asset_amount(
        lender_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    assert_eq_noise!(
        lender_usdc_value,
        I80F48::from(native!(100_000, "USDC")),
        I80F48::ONE
    );

    let insurance_amount = test_f
        .get_bank(&BankMint::Usdc)
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq!(insurance_amount.balance().await, 0);

    // Test account is disabled

    // Deposit 1 SOL
    let res = borrower_account
        .try_bank_deposit(
            borrower_deposit_account.key,
            test_f.get_bank(&BankMint::Sol),
            1,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Withdraw 1 SOL
    let res = borrower_account
        .try_bank_withdraw(
            borrower_deposit_account.key,
            test_f.get_bank(&BankMint::Sol),
            1,
            None,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Borrow 1 USDC
    let res = borrower_account
        .try_bank_borrow(
            borrower_borrow_account.key,
            test_f.get_bank(&BankMint::Usdc),
            1,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Repay 1 USDC
    let res = borrower_account
        .try_bank_repay(
            borrower_borrow_account.key,
            test_f.get_bank(&BankMint::Usdc),
            1,
            None,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_fully_insured_t22_with_fee() -> anyhow::Result<()>
{
    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let collateral_mint = BankMint::Sol;
    let debt_mint = BankMint::T22WithFee;

    let user_balance = 100_000.0 * (1f64 + MAX_FEE_BASIS_POINTS as f64 / 10_000f64);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_debt = test_f
        .get_bank_mut(&debt_mint)
        .mint
        .create_token_account_and_mint_to(user_balance)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(
            lender_token_account_debt.key,
            test_f.get_bank(&debt_mint),
            100_000,
        )
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_collateral_account = test_f
        .get_bank_mut(&collateral_mint)
        .mint
        .create_token_account_and_mint_to(1_001)
        .await;

    borrower_mfi_account_f
        .try_bank_deposit(
            borrower_collateral_account.key,
            test_f.get_bank_mut(&collateral_mint),
            1_001,
        )
        .await?;

    let borrower_debt_account = test_f
        .get_bank_mut(&debt_mint)
        .mint
        .create_empty_token_account()
        .await;

    let borrow_amount = 10_000;

    borrower_mfi_account_f
        .try_bank_borrow(
            borrower_debt_account.key,
            test_f.get_bank_mut(&debt_mint),
            borrow_amount as f64,
        )
        .await?;
    assert_eq!(
        borrower_debt_account.balance().await,
        native!(borrow_amount, "USDC")
    );

    let mut borrower_mfi_account = borrower_mfi_account_f.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    borrower_mfi_account_f
        .set_account(&borrower_mfi_account)
        .await?;

    {
        let (insurance_vault, _) = test_f
            .get_bank(&debt_mint)
            .get_vault(BankVaultType::Insurance);
        let max_amount_to_cover_bad_debt =
            borrow_amount as f64 * (1f64 + MAX_FEE_BASIS_POINTS as f64 / 10_000f64);
        test_f
            .get_bank_mut(&debt_mint)
            .mint
            .mint_to(&insurance_vault, max_amount_to_cover_bad_debt)
            .await;
    }

    let debt_bank = test_f.get_bank(&debt_mint);

    let (pre_liquidity_vault_balance, pre_insurance_vault_balance) = (
        debt_bank
            .get_vault_token_account(BankVaultType::Liquidity)
            .await
            .balance()
            .await,
        debt_bank
            .get_vault_token_account(BankVaultType::Insurance)
            .await
            .balance()
            .await,
    );

    test_f
        .marginfi_group
        .try_handle_bankruptcy(test_f.get_bank(&debt_mint), &borrower_mfi_account_f)
        .await?;

    let (post_liquidity_vault_balance, post_insurance_vault_balance) = (
        debt_bank
            .get_vault_token_account(BankVaultType::Liquidity)
            .await
            .balance()
            .await,
        debt_bank
            .get_vault_token_account(BankVaultType::Insurance)
            .await
            .balance()
            .await,
    );

    let borrower_mfi_account = borrower_mfi_account_f.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let debt_bank = test_f.get_bank(&debt_mint).load().await;

    let lender_collateral_value = debt_bank.get_asset_amount(
        lender_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    // Check that no loss was socialized
    assert_eq_noise!(
        lender_collateral_value,
        I80F48::from(native!(
            100_000,
            test_f.get_bank(&debt_mint).mint.mint.decimals
        )),
        I80F48::ONE
    );

    let debt_bank_mint_state = test_f.get_bank(&debt_mint).mint.load_state().await;
    // borrow + fee(borrow)
    let expected_liquidity_vault_delta = native!(
        borrow_amount,
        test_f.get_bank(&debt_mint).mint.mint.decimals
    ) + debt_bank_mint_state
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_inverse_epoch_fee(
                0,
                native!(
                    borrow_amount,
                    test_f.get_bank(&debt_mint).mint.mint.decimals
                ),
            )
            .unwrap_or(0)
        })
        .unwrap_or(0);
    let expected_liquidity_vault_delta = I80F48::from(expected_liquidity_vault_delta);
    let actual_liquidity_vault_delta = post_liquidity_vault_balance - pre_liquidity_vault_balance;
    let actual_insurance_vault_delta = pre_insurance_vault_balance - post_insurance_vault_balance;

    assert_eq!(expected_liquidity_vault_delta, actual_liquidity_vault_delta);
    assert!(actual_insurance_vault_delta > expected_liquidity_vault_delta);

    // Test account is disabled

    // Deposit 1 SOL
    let res = borrower_mfi_account_f
        .try_bank_deposit(
            borrower_collateral_account.key,
            test_f.get_bank(&collateral_mint),
            1,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Withdraw 1 SOL
    let res = borrower_mfi_account_f
        .try_bank_withdraw(
            borrower_collateral_account.key,
            test_f.get_bank(&collateral_mint),
            1,
            None,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Borrow 1 USDC
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_debt_account.key, test_f.get_bank(&debt_mint), 1)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Repay 1 USDC
    let res = borrower_mfi_account_f
        .try_bank_repay(
            borrower_debt_account.key,
            test_f.get_bank(&debt_mint),
            1,
            None,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_partially_insured() -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings {
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
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

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
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_account
        .try_bank_deposit(
            borrower_token_account_sol.key,
            test_f.get_bank(&BankMint::Sol),
            1_001,
        )
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_account
        .try_bank_borrow(
            borrower_token_account_usdc.key,
            test_f.get_bank(&BankMint::Usdc),
            10_000,
        )
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    borrower_account.set_account(&borrower_mfi_account).await?;

    test_f
        .usdc_mint
        .mint_to(
            &test_f
                .get_bank(&BankMint::Usdc)
                .load()
                .await
                .insurance_vault,
            5_000,
        )
        .await;

    test_f
        .marginfi_group
        .try_handle_bankruptcy(test_f.get_bank(&BankMint::Usdc), &borrower_account)
        .await?;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let usdc_bank = test_f.get_bank(&BankMint::Usdc).load().await;

    let lender_usdc_value = usdc_bank.get_asset_amount(
        lender_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    assert_eq_noise!(
        lender_usdc_value,
        I80F48::from(native!(95_000, "USDC")),
        I80F48::ONE
    );

    let insurance_amount = test_f
        .get_bank(&BankMint::Usdc)
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq!(insurance_amount.balance().await, 0);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_partially_insured_t22_with_fee(
) -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let collateral_mint = BankMint::Sol;
    let debt_mint = BankMint::T22WithFee;

    let pre_lender_collateral_amount = 100_000.0;
    let user_balance =
        pre_lender_collateral_amount * (1f64 + MAX_FEE_BASIS_POINTS as f64 / 10_000f64);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_collateral_token_account = test_f
        .get_bank(&debt_mint)
        .mint
        .create_token_account_and_mint_to(user_balance)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(
            lender_collateral_token_account.key,
            test_f.get_bank(&debt_mint),
            pre_lender_collateral_amount,
        )
        .await?;

    let borrower_account = test_f.create_marginfi_account().await;
    let borrower_collateral_token_account = test_f
        .get_bank(&collateral_mint)
        .mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_account
        .try_bank_deposit(
            borrower_collateral_token_account.key,
            test_f.get_bank(&collateral_mint),
            1_001,
        )
        .await?;

    let borrow_amount = 10_000.0;

    let borrower_debt_token_account = test_f
        .get_bank(&debt_mint)
        .mint
        .create_empty_token_account()
        .await;
    borrower_account
        .try_bank_borrow(
            borrower_debt_token_account.key,
            test_f.get_bank(&debt_mint),
            borrow_amount,
        )
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    borrower_account.set_account(&borrower_mfi_account).await?;

    let initial_insurance_vault_balance = 5_000.0;
    let insurance_vault = test_f.get_bank(&debt_mint).load().await.insurance_vault;
    test_f
        .get_bank_mut(&debt_mint)
        .mint
        .mint_to(&insurance_vault, initial_insurance_vault_balance)
        .await;

    let debt_bank_f = test_f.get_bank(&debt_mint);

    let pre_borrower_debt_balance = borrower_account.load().await.lending_account.balances[1];

    let pre_liquidity_vault_balance = debt_bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;

    test_f
        .marginfi_group
        .try_handle_bankruptcy(test_f.get_bank(&debt_mint), &borrower_account)
        .await?;

    let post_liquidity_vault_balance = debt_bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_debt_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_debt_balance.liability_shares),
        I80F48::ZERO
    );

    let debt_bank = test_f.get_bank(&debt_mint).load().await;
    let pre_borrower_debt_amount =
        debt_bank.get_liability_amount(pre_borrower_debt_balance.liability_shares.into())?;

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let debt_bank = test_f.get_bank(&debt_mint).load().await;

    let post_lender_collateral_amount = debt_bank.get_asset_amount(
        lender_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    let debt_bank_mint_state = test_f.get_bank(&debt_mint).mint.load_state().await;

    let available_insurance_amount = native!(
        initial_insurance_vault_balance,
        test_f.get_bank(&debt_mint).mint.mint.decimals,
        f64
    );
    let if_fee_amount = debt_bank_mint_state
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_epoch_fee(0, available_insurance_amount)
                .unwrap_or(0)
        })
        .unwrap_or(0);

    let available_insurance_amount = available_insurance_amount - if_fee_amount;

    let amount_not_covered = pre_borrower_debt_amount - I80F48::from(available_insurance_amount);

    let pre_lender_collateral_amount_native = I80F48::from(native!(
        pre_lender_collateral_amount,
        debt_bank.mint_decimals,
        f64
    ));

    let expected_post_lender_collateral_amount =
        pre_lender_collateral_amount_native - amount_not_covered;
    assert_eq_noise!(
        expected_post_lender_collateral_amount,
        post_lender_collateral_amount,
        I80F48::ONE
    );

    let insurance_amount = test_f
        .get_bank(&debt_mint)
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq!(insurance_amount.balance().await, 0);

    let actual_liquidity_vault_delta = post_liquidity_vault_balance - pre_liquidity_vault_balance;
    assert_eq!(available_insurance_amount, actual_liquidity_vault_delta);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_not_insured() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
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
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000)
        .await?;

    let borrower_account = test_f.create_marginfi_account().await;
    let borrower_deposit_account = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_account
        .try_bank_deposit(borrower_deposit_account.key, sol_bank_f, 1_001)
        .await?;
    let borrower_borrow_account = test_f.usdc_mint.create_empty_token_account().await;
    borrower_account
        .try_bank_borrow(borrower_borrow_account.key, usdc_bank_f, 10_000)
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();

    borrower_account.set_account(&borrower_mfi_account).await?;

    test_f
        .marginfi_group
        .try_handle_bankruptcy(usdc_bank_f, &borrower_account)
        .await?;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let usdc_bank = usdc_bank_f.load().await;

    let lender_usdc_value = usdc_bank.get_asset_amount(
        lender_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    assert_eq_noise!(
        lender_usdc_value,
        I80F48::from(native!(90_000, "USDC")),
        I80F48::ONE
    );

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_not_insured_3_depositors() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
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
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

    let lender_1_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_1_token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_1_mfi_account_f
        .try_bank_deposit(lender_1_token_account.key, usdc_bank_f, 100_000)
        .await?;

    let lender_2_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_2_token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_2_mfi_account_f
        .try_bank_deposit(lender_2_token_account.key, usdc_bank_f, 100_000)
        .await?;

    let lender_3_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_3_token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_3_mfi_account_f
        .try_bank_deposit(lender_3_token_account.key, usdc_bank_f, 100_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 1_001)
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 10_000)
        .await?;

    let mut borrower_mfi_account = borrower_mfi_account_f.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();

    borrower_mfi_account_f
        .set_account(&borrower_mfi_account)
        .await?;

    test_f
        .marginfi_group
        .try_handle_bankruptcy(usdc_bank_f, &borrower_mfi_account_f)
        .await?;

    let borrower_mfi_account = borrower_mfi_account_f.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_1_mfi_account = lender_1_mfi_account_f.load().await;
    let usdc_bank = usdc_bank_f.load().await;

    let lender_usdc_value = usdc_bank.get_asset_amount(
        lender_1_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    assert_eq_noise!(
        lender_usdc_value,
        I80F48::from(native!(96_666, "USDC")),
        I80F48::from(native!(1, "USDC"))
    );

    Ok(())
}
