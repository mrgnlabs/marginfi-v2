use anchor_lang::prelude::Clock;
use fixtures::{assert_custom_error, native, prelude::*};
use marginfi::{prelude::*, state::fee_state::PanicState};
use pretty_assertions::assert_eq;
use solana_program_test::*;

#[tokio::test]
async fn test_deposit_when_panic_mode_not_set() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let marginfi_account_f = test_f.create_marginfi_account().await;
    let token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    let marginfi_group = test_f.marginfi_group.load().await;
    assert!(!marginfi_group.panic_state_cache.is_paused());

    marginfi_account_f
        .try_bank_deposit(token_account.key, usdc_bank_f, 500, None)
        .await?;

    let marginfi_account = marginfi_account_f.load().await;
    let bank = usdc_bank_f.load().await;

    assert!(marginfi_account.lending_account.balances[0].is_active());
    let asset_amount = bank.get_asset_amount(
        marginfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;
    assert_eq!(asset_amount.to_num::<u64>(), native!(500, "USDC"));

    Ok(())
}

#[tokio::test]
async fn test_deposit_when_panic_mode_set() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let marginfi_account_f = test_f.create_marginfi_account().await;
    let token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    // Set panic mode
    test_f.marginfi_group.try_panic_pause().await?;

    // Propagate panic state to group cache
    test_f.marginfi_group.try_propagate_fee_state().await?;

    // Verify panic mode is now set in the cache
    let marginfi_group = test_f.marginfi_group.load().await;
    assert!(marginfi_group.panic_state_cache.is_paused());

    // Deposit should fail when panic mode is active
    let result = marginfi_account_f
        .try_bank_deposit(token_account.key, usdc_bank_f, 500, None)
        .await;

    assert_custom_error!(result.unwrap_err(), MarginfiError::ProtocolPaused);

    Ok(())
}

#[tokio::test]
async fn test_deposit_when_panic_mode_expires_via_propagate_fee() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let marginfi_account_f = test_f.create_marginfi_account().await;
    let token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    test_f.marginfi_group.try_panic_pause().await?;

    let start_timestamp = {
        let ctx = test_f.context.borrow_mut();
        let clock: Clock = ctx.banks_client.get_sysvar().await?;
        clock.unix_timestamp
    };

    test_f.marginfi_group.try_propagate_fee_state().await?;

    let marginfi_group = test_f.marginfi_group.load().await;
    assert!(marginfi_group.panic_state_cache.is_paused());

    {
        let ctx = test_f.context.borrow_mut();
        let mut clock: Clock = ctx.banks_client.get_sysvar().await?;
        clock.unix_timestamp = start_timestamp + PanicState::PAUSE_DURATION_SECONDS + 60;
        ctx.set_sysvar(&clock);
    }

    // Verify panic mode is now not active (expired and auto-unpaused)
    let marginfi_group = test_f.marginfi_group.load().await;
    let current_timestamp = {
        let ctx = test_f.context.borrow_mut();
        let clock: Clock = ctx.banks_client.get_sysvar().await?;
        clock.unix_timestamp
    };
    assert!(marginfi_group
        .panic_state_cache
        .is_expired(current_timestamp));

    marginfi_account_f
        .try_bank_deposit(token_account.key, usdc_bank_f, 500, None)
        .await?;

    let marginfi_account = marginfi_account_f.load().await;
    let bank = usdc_bank_f.load().await;

    assert!(marginfi_account.lending_account.balances[0].is_active());
    let asset_amount = bank.get_asset_amount(
        marginfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;
    assert_eq!(asset_amount.to_num::<u64>(), native!(500, "USDC"));

    Ok(())
}

#[tokio::test]
async fn test_borrow_when_panic_mode_not_set() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

    // Setup liquidity and collateral
    let lender_account_f = test_f.create_marginfi_account().await;
    let sol_token_account = test_f.sol_mint.create_token_account_and_mint_to(10).await;

    lender_account_f
        .try_bank_deposit(sol_token_account.key, sol_bank_f, 10, None)
        .await?;

    let borrower_account_f = test_f.create_marginfi_account().await;
    let usdc_token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    borrower_account_f
        .try_bank_deposit(usdc_token_account.key, usdc_bank_f, 1000, None)
        .await?;

    let borrow_token_account = test_f.sol_mint.create_empty_token_account().await;

    let marginfi_group = test_f.marginfi_group.load().await;
    assert!(!marginfi_group.panic_state_cache.is_paused());

    borrower_account_f
        .try_bank_borrow(borrow_token_account.key, sol_bank_f, 1)
        .await?;

    let marginfi_account = borrower_account_f.load().await;

    let sol_balance = marginfi_account
        .lending_account
        .get_balance(&sol_bank_f.key)
        .unwrap();
    assert!(fixed::types::I80F48::from(sol_balance.liability_shares) > fixed::types::I80F48::ZERO);

    Ok(())
}

#[tokio::test]
async fn test_borrow_when_panic_mode_set() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

    let lender_account_f = test_f.create_marginfi_account().await;
    let sol_token_account = test_f.sol_mint.create_token_account_and_mint_to(10).await;

    lender_account_f
        .try_bank_deposit(sol_token_account.key, sol_bank_f, 10, None)
        .await?;

    let borrower_account_f = test_f.create_marginfi_account().await;
    let usdc_token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    borrower_account_f
        .try_bank_deposit(usdc_token_account.key, usdc_bank_f, 1000, None)
        .await?;

    let borrow_token_account = test_f.sol_mint.create_empty_token_account().await;

    test_f.marginfi_group.try_panic_pause().await?;

    test_f.marginfi_group.try_propagate_fee_state().await?;

    let marginfi_group = test_f.marginfi_group.load().await;
    assert!(marginfi_group.panic_state_cache.is_paused());

    let result = borrower_account_f
        .try_bank_borrow(borrow_token_account.key, sol_bank_f, 1)
        .await;

    assert_custom_error!(result.unwrap_err(), MarginfiError::ProtocolPaused);

    Ok(())
}

#[tokio::test]
async fn test_withdraw_when_panic_mode_not_set() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let marginfi_account_f = test_f.create_marginfi_account().await;
    let token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    marginfi_account_f
        .try_bank_deposit(token_account.key, usdc_bank_f, 500, None)
        .await?;

    let marginfi_group = test_f.marginfi_group.load().await;
    assert!(!marginfi_group.panic_state_cache.is_paused());

    marginfi_account_f
        .try_bank_withdraw(token_account.key, usdc_bank_f, 100, None)
        .await?;

    let marginfi_account = marginfi_account_f.load().await;
    let bank = usdc_bank_f.load().await;

    let remaining_amount = bank.get_asset_amount(
        marginfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;
    assert_eq!(remaining_amount.to_num::<u64>(), native!(400, "USDC"));

    Ok(())
}

#[tokio::test]
async fn test_withdraw_when_panic_mode_set() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let marginfi_account_f = test_f.create_marginfi_account().await;
    let token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    marginfi_account_f
        .try_bank_deposit(token_account.key, usdc_bank_f, 500, None)
        .await?;

    test_f.marginfi_group.try_panic_pause().await?;

    test_f.marginfi_group.try_propagate_fee_state().await?;

    let marginfi_group = test_f.marginfi_group.load().await;
    assert!(marginfi_group.panic_state_cache.is_paused());

    // Withdraw should fail when panic mode is active
    let result = marginfi_account_f
        .try_bank_withdraw(token_account.key, usdc_bank_f, 100, None)
        .await;

    assert_custom_error!(result.unwrap_err(), MarginfiError::ProtocolPaused);

    Ok(())
}

#[tokio::test]
async fn test_propagate_fee_updates_both_fee_and_panic_caches() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    test_f.marginfi_group.try_panic_pause().await?;

    test_f.marginfi_group.try_propagate_fee_state().await?;

    let start_timestamp = {
        let ctx = test_f.context.borrow_mut();
        let clock: Clock = ctx.banks_client.get_sysvar().await?;
        clock.unix_timestamp
    };

    let marginfi_group = test_f.marginfi_group.load().await;

    assert!(marginfi_group.panic_state_cache.is_paused());
    assert!(marginfi_group.panic_state_cache.last_cache_update >= start_timestamp);

    Ok(())
}
