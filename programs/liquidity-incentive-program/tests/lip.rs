use anyhow::Result;
use fixed::types::I80F48;
use fixtures::{
    assert_custom_error, native,
    spl::{balance_of, TokenAccountFixture},
    test::{TestFixture, DEFAULT_SOL_TEST_BANK_CONFIG, DEFAULT_USDC_TEST_BANK_CONFIG},
    time,
    utils::lip::get_reward_vault_address,
};
use liquidity_incentive_program::errors::LIPError;
use marginfi::assert_eq_with_tolerance;
use solana_program_test::tokio;

#[tokio::test]
async fn campaign_no_yield() -> Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&test_f.usdc_mint, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;

    let campaign_reward_funding_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    let campaign_res = usdc_bank
        .try_create_campaign(
            time!(1, "s"),
            native!(1000, "USDC"),
            native!(1000, "USDC"),
            campaign_reward_funding_account.key,
        )
        .await;

    assert!(campaign_res.is_ok());

    let campaign_f = campaign_res.unwrap();

    let deposit_funding_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1001)
        .await;

    let res = campaign_f
        .try_create_deposit(deposit_funding_account.key, native!(1001, "USDC"))
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), LIPError::DepositAmountTooLarge);

    let res = campaign_f
        .try_create_deposit(deposit_funding_account.key, native!(1000, "USDC"))
        .await;

    assert!(res.is_ok());

    let deposit_key = res.unwrap();

    let campaign = campaign_f.load().await;
    let deposit = campaign_f.load_deposit(deposit_key).await;

    assert_eq!(deposit.amount, native!(1000, "USDC"));
    assert_eq!(campaign.max_deposits, native!(1000, "USDC"));
    assert_eq!(campaign.remaining_capacity, 0);

    let destination_account = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    let res = campaign_f
        .try_end_deposit(deposit_key, destination_account.key)
        .await;

    assert!(res.is_err());

    test_f.advance_time(time!(1, "s")).await;

    let destination_account = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    let res = campaign_f
        .try_end_deposit(deposit_key, destination_account.key)
        .await;

    assert!(res.is_ok());

    let deposit = test_f
        .context
        .borrow_mut()
        .banks_client
        .get_account(deposit_key)
        .await?;

    assert!(deposit.is_none());

    assert_eq!(
        balance_of(test_f.context.clone(), destination_account.key).await,
        native!(2000, "USDC")
    );

    Ok(())
}

#[tokio::test]
async fn campaign_mixed_yield() -> Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&test_f.usdc_mint, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;

    let sol_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&test_f.sol_mint, *DEFAULT_SOL_TEST_BANK_CONFIG)
        .await?;

    let campaign_reward_funding_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    let campaign_res = usdc_bank
        .try_create_campaign(
            time!(1, "s"),
            native!(1000, "USDC"),
            native!(1000, "USDC"),
            campaign_reward_funding_account.key,
        )
        .await;

    assert!(campaign_res.is_ok());

    let campaign_f = campaign_res.unwrap();

    let deposit_funding_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1001)
        .await;

    let deposit_key = campaign_f
        .try_create_deposit(deposit_funding_account.key, native!(1000, "USDC"))
        .await?;

    let borrower = test_f.create_marginfi_account().await;

    let sol_funding_account = test_f.sol_mint.create_token_account_and_mint_to(1000).await;

    borrower
        .try_bank_deposit(sol_funding_account.key, &sol_bank, 1000)
        .await?;

    let usdc_borrowing_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(10000)
        .await;

    borrower
        .try_bank_borrow(usdc_borrowing_account.key, &usdc_bank, 500)
        .await?;

    test_f.advance_time(time!(1, "y")).await;

    borrower
        .try_bank_repay(usdc_borrowing_account.key, &usdc_bank, 500, Some(true))
        .await?;

    let destination_account = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    campaign_f
        .try_end_deposit(deposit_key, destination_account.key)
        .await?;

    assert_eq!(
        balance_of(test_f.context.clone(), destination_account.key).await,
        native!(2000, "USDC")
    );

    let reward_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        get_reward_vault_address(campaign_f.key).0,
    )
    .await;

    assert_eq_with_tolerance!(
        reward_vault.balance().await as i64,
        native!(300, "USDC") as i64,
        native!(1, "USDC") as i64
    );

    Ok(())
}

#[tokio::test]
async fn campaign_max_yield() -> Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&test_f.usdc_mint, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;

    let sol_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&test_f.sol_mint, *DEFAULT_SOL_TEST_BANK_CONFIG)
        .await?;

    let campaign_reward_funding_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    let campaign_res = usdc_bank
        .try_create_campaign(
            time!(1, "s"),
            native!(1000, "USDC"),
            native!(1000, "USDC"),
            campaign_reward_funding_account.key,
        )
        .await;

    assert!(campaign_res.is_ok());

    let campaign_f = campaign_res.unwrap();

    let deposit_funding_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1001)
        .await;

    let deposit_key = campaign_f
        .try_create_deposit(deposit_funding_account.key, native!(1000, "USDC"))
        .await?;

    let borrower = test_f.create_marginfi_account().await;

    let sol_funding_account = test_f.sol_mint.create_token_account_and_mint_to(1000).await;

    borrower
        .try_bank_deposit(sol_funding_account.key, &sol_bank, 1000)
        .await?;

    let usdc_borrowing_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(10000)
        .await;

    borrower
        .try_bank_borrow(usdc_borrowing_account.key, &usdc_bank, 500)
        .await?;

    test_f.advance_time(time!(10, "y")).await;

    borrower
        .try_bank_repay(usdc_borrowing_account.key, &usdc_bank, 500, Some(true))
        .await?;

    let destination_account = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    campaign_f
        .try_end_deposit(deposit_key, destination_account.key)
        .await?;

    assert_eq_with_tolerance!(
        balance_of(test_f.context.clone(), destination_account.key).await as i64,
        native!(4000, "USDC") as i64,
        native!(1, "USDC") as i64
    );

    let reward_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        get_reward_vault_address(campaign_f.key).0,
    )
    .await;

    assert_eq!(reward_vault.balance().await, native!(1000, "USDC"));

    Ok(())
}

#[tokio::test]
async fn campaign_neg_yield() -> Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&test_f.usdc_mint, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;

    let campaign_reward_funding_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1000)
        .await;

    let campaign_f = usdc_bank
        .try_create_campaign(
            time!(1, "y"),
            native!(1000, "USDC"),
            native!(1000, "USDC"),
            campaign_reward_funding_account.key,
        )
        .await?;

    let deposit_funding_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1001)
        .await;

    let deposit_key = campaign_f
        .try_create_deposit(deposit_funding_account.key, native!(1000, "USDC"))
        .await?;

    test_f.advance_time(time!(1, "y")).await;

    usdc_bank
        .set_asset_share_value(I80F48::from(usdc_bank.load().await.asset_share_value) / 2)
        .await;

    let destination_account = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    campaign_f
        .try_end_deposit(deposit_key, destination_account.key)
        .await?;

    assert_eq!(
        balance_of(test_f.context.clone(), destination_account.key).await,
        native!(1500, "USDC")
    );

    let reward_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        get_reward_vault_address(campaign_f.key).0,
    )
    .await;

    assert_eq!(reward_vault.balance().await, 0);

    Ok(())
}
