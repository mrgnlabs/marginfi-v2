use anyhow::Result;
use fixtures::{
    assert_custom_error, native,
    spl::balance_of,
    test::{TestFixture, DEFAULT_USDC_TEST_BANK_CONFIG},
    time,
};
use liquidity_incentive_program::LIPError;

use solana_program_test::tokio;
use solana_sdk::signer::Signer;

#[tokio::test]
async fn lip_create_campaign() -> Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&test_f.usdc_mint, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;

    let _marginfi_account_f = test_f.create_marginfi_account().await;

    let _owner = test_f.context.borrow().payer.pubkey();

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
        .try_deposit(deposit_funding_account.key, native!(1001, "USDC"))
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), LIPError::DepositAmountTooLarge);

    let res = campaign_f
        .try_deposit(deposit_funding_account.key, native!(1000, "USDC"))
        .await;

    assert!(res.is_ok());

    let deposit_key = res.unwrap();

    let campaign = campaign_f.load().await;
    let deposit = campaign_f.load_deposit(deposit_key).await;

    assert_eq!(deposit.amount, native!(1000, "USDC"));
    assert_eq!(campaign.max_deposits, native!(1000, "USDC"));
    assert_eq!(campaign.outstanding_deposits, 0);

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
