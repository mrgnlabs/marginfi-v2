use fixtures::{
    assert_custom_error,
    test::{BankMint, TestFixture, TestSettings},
};
use marginfi::errors::MarginfiError;
use pretty_assertions::assert_eq;
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer};

#[tokio::test]
async fn bank_delegate_admin_success() -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_settings = TestSettings::all_banks_payer_not_admin();
    if let Some(group_config) = &mut test_settings.group_config {
        group_config.admin = Some(solana_sdk::pubkey::Pubkey::default());
    }
    let mut test_f = TestFixture::new(Some(test_settings)).await;
    let delegate = Keypair::new();
    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);

    // -------------------------------------------------------------------------
    // Test: Set delegate admin
    // -------------------------------------------------------------------------

    usdc_bank_f.try_configure_delegate(delegate.pubkey()).await?;
    let bank = usdc_bank_f.load().await;
    assert_eq!(bank.delegate_admin, delegate.pubkey());

    // -------------------------------------------------------------------------
    // Test: Increase deposit limit
    // -------------------------------------------------------------------------

    let initial_limit = bank.config.deposit_limit;
    let new_limit = initial_limit + 1000;
    usdc_bank_f.try_increase_deposit_limit(&delegate, new_limit).await?;

    let bank = usdc_bank_f.load().await;
    assert_eq!(bank.config.deposit_limit, new_limit);

    // -------------------------------------------------------------------------
    // Test: Increase emissions rate
    // -------------------------------------------------------------------------

    let initial_rate = bank.emissions_rate;
    let new_rate = initial_rate + 100;
    usdc_bank_f.try_increase_emissions_rate(&delegate, new_rate).await?;

    let bank = usdc_bank_f.load().await;
    assert_eq!(bank.emissions_rate, new_rate);

    Ok(())
}

#[tokio::test]
async fn bank_delegate_admin_failure_decrease_limit() -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_settings = TestSettings::all_banks_payer_not_admin();
    if let Some(group_config) = &mut test_settings.group_config {
        group_config.admin = Some(solana_sdk::pubkey::Pubkey::default());
    }
    let mut test_f = TestFixture::new(Some(test_settings)).await;
    let delegate = Keypair::new();
    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);

    // Set up delegate
    usdc_bank_f.try_configure_delegate(delegate.pubkey()).await?;
    let bank = usdc_bank_f.load().await;
    let initial_limit = bank.config.deposit_limit;

    // -------------------------------------------------------------------------
    // Test: Try to decrease deposit limit (should fail)
    // -------------------------------------------------------------------------

    let lower_limit = initial_limit - 1000;
    let res = usdc_bank_f.try_increase_deposit_limit(&delegate, lower_limit).await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::InvalidDelegateOperation);

    Ok(())
}

#[tokio::test]
async fn bank_delegate_admin_failure_wrong_signer() -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    let mut test_settings = TestSettings::all_banks_payer_not_admin();
    if let Some(group_config) = &mut test_settings.group_config {
        group_config.admin = Some(solana_sdk::pubkey::Pubkey::default());
    }
    let mut test_f = TestFixture::new(Some(test_settings)).await;
    let delegate = Keypair::new();
    let wrong_signer = Keypair::new();
    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);

    // Set up delegate
    usdc_bank_f.try_configure_delegate(delegate.pubkey()).await?;
    let bank = usdc_bank_f.load().await;

    // -------------------------------------------------------------------------
    // Test: Try operations with wrong signer
    // -------------------------------------------------------------------------

    // Try to increase deposit limit
    let new_limit = bank.config.deposit_limit + 1000;
    let res = usdc_bank_f.try_increase_deposit_limit(&wrong_signer, new_limit).await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::InvalidDelegateOperation);

    // Try to increase emissions rate
    let new_rate = bank.emissions_rate + 100;
    let res = usdc_bank_f.try_increase_emissions_rate(&wrong_signer, new_rate).await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::InvalidDelegateOperation);

    Ok(())
}