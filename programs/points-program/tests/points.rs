use marginfi::state::marginfi_account::MarginfiAccount;
use solana_program_test::tokio;
use solana_program_test::tokio::time::{self, Duration};
use fixtures::{
    test::TestFixture,
    points::PointsFixture,
    test::{TestBankSetting, TestSettings, BankMint},
};
use solana_sdk::signature::Keypair;
use points_program::AccountBalances;
use std::rc::Rc;

// In order to load the points account into memory you have to use RUST_MIN_STACK=1000000000
// Otherwise calling points_f.load() blows the stack
#[tokio::test]
async fn initialize_global_points() -> Result<(), anyhow::Error> {
    let test_f = TestFixture::new(None).await;
    let points_f = PointsFixture::new(Rc::clone(&test_f.context), Keypair::new());

    points_f.try_initialize_global_points().await?;

    Ok(())
}

#[tokio::test]
async fn initialize_points_account_single() -> Result<(), anyhow::Error> {
    let test_f = TestFixture::new(None).await;
    let points_f = PointsFixture::new(Rc::clone(&test_f.context), Keypair::new());

    points_f.try_initialize_global_points().await?;

    let mfi_account_f = test_f.create_marginfi_account().await;

    points_f.try_initialize_points_account(mfi_account_f.key).await?;

    let points_mapping: points_program::PointsMapping = points_f.load().await;
 
    assert!(points_mapping.points_accounts[0].is_some());
    assert!(points_mapping.first_free_index == 1_usize);

    Ok(())
}

#[tokio::test]
async fn accrue_points_single() -> Result<(), anyhow::Error> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::USDC,
            ..TestBankSetting::default()
        }],
        ..TestSettings::default()
    }))
    .await;
    let points_f = PointsFixture::new(Rc::clone(&test_f.context), Keypair::new());

    points_f.try_initialize_global_points().await?;

    let mfi_account_f = test_f.create_marginfi_account().await;

    points_f.try_initialize_points_account(mfi_account_f.key).await?;

    // Deposit some funds so there's actually points to accrue
    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100)
        .await;
    mfi_account_f
        .try_bank_deposit(token_account_usdc.key, usdc_bank_f, 100)
        .await?;

    let mfi_account: MarginfiAccount = mfi_account_f.load().await;

    let balances: [points_program::Balance; 16] = mfi_account.lending_account.balances
        .map(|balance| points_program::Balance::from(balance));

    let account_balances = AccountBalances { balances };

    let account_balance_datas = vec![(mfi_account_f.key, account_balances)];

    let price_data = vec![(usdc_bank_f.key, 1_i128)];

    points_f.try_accrue_points(
        account_balance_datas.clone(),
        price_data.clone(),
        0
    ).await?;

    let points_mapping: points_program::PointsMapping = points_f.load().await;
    
    println!("{}", points_mapping.points_accounts[0].unwrap().points.value);

    Ok(())
}