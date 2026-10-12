use fixed_macro::types::I80F48;
use fixtures::prelude::*;
use marginfi::errors::MarginfiError;

/// `lending_account_close_balance` refuses integration (Kamino/Drift/Solend/JupLend) banks via
/// `is_marginfi_asset_tag`, so a venue balance emptied down to dust could never be closed and
/// permanently consumed a balance slot. `lending_account_close_integration_balance` releases it.
///
/// A real position can be left holding asset dust below `ZERO_AMOUNT_THRESHOLD` (interest
/// rounding, a partial redeem). At that size the venue's own `withdraw_all` rejects it
/// (`NoAssetFound`) and there is no liability to repay, so the slot is otherwise stuck. As in
/// `close_balance.rs`, seed the dust state directly to retain coverage of the close path.
#[tokio::test]
async fn close_integration_balance_frees_dust_venue_slot() -> anyhow::Result<()> {
    let setup = TestFixture::setup_kamino_bank(None).await;
    let (user, user_token) = setup.create_user_with_liquidity(10.0).await;

    setup
        .test_f
        .run_kamino_deposit(&setup.bank_f, &user, user_token.key, 1_000_000)
        .await?;

    let dust_asset_shares = I80F48!(0.00005);
    let mut account = user.load().await;
    let kamino_balance = account
        .lending_account
        .balances
        .iter_mut()
        .find(|balance| balance.is_active() && balance.bank_pk == setup.bank_f.key)
        .expect("kamino balance should be active after deposit");
    kamino_balance.asset_shares = dust_asset_shares.into();
    user.set_account(&account).await?;

    let active_before = user
        .load()
        .await
        .lending_account
        .balances
        .iter()
        .filter(|b| b.is_active())
        .count();
    assert_eq!(active_before, 1, "dust balance should be active");

    // The standard instruction must still reject an integration bank.
    let res = user.try_balance_close(&setup.bank_f).await;
    assert!(res.is_err());
    assert_custom_error!(
        res.unwrap_err(),
        MarginfiError::WrongAssetTagForStandardInstructions
    );

    // The new instruction closes it.
    user.try_close_integration_balance(&setup.bank_f).await?;

    let account = user.load().await;
    let active_after = account
        .lending_account
        .balances
        .iter()
        .filter(|b| b.is_active())
        .count();
    assert_eq!(active_after, 0, "integration balance should be closed");

    Ok(())
}

/// A venue that still holds a real (non-dust) position must not be closable here — the caller has
/// to withdraw it through the venue's own withdraw path first.
#[tokio::test]
async fn close_integration_balance_rejects_non_empty_position() -> anyhow::Result<()> {
    let setup = TestFixture::setup_kamino_bank(None).await;
    let (user, user_token) = setup.create_user_with_liquidity(10.0).await;

    setup
        .test_f
        .run_kamino_deposit(&setup.bank_f, &user, user_token.key, 1_000_000)
        .await?;

    let res = user.try_close_integration_balance(&setup.bank_f).await;
    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalBalanceState);

    let balance_still_active = user
        .load()
        .await
        .lending_account
        .balances
        .iter()
        .any(|b| b.is_active() && b.bank_pk == setup.bank_f.key);
    assert!(
        balance_still_active,
        "position must survive the failed close"
    );

    Ok(())
}

/// The new instruction must refuse standard (non-integration) banks; that path stays owned by
/// `lending_account_close_balance`.
#[tokio::test]
async fn close_integration_balance_rejects_standard_bank() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let usdc_bank = test_f.get_bank(&BankMint::Usdc);

    let user = test_f.create_marginfi_account().await;
    let token = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    user.try_bank_deposit(token.key, usdc_bank, 1_000, None)
        .await?;

    let res = user.try_close_integration_balance(&usdc_bank).await;
    assert!(res.is_err());
    assert_custom_error!(
        res.unwrap_err(),
        MarginfiError::WrongAssetTagForIntegrationInstructions
    );

    Ok(())
}
