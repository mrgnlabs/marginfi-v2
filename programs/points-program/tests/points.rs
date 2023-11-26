use marginfi::state::marginfi_account::MarginfiAccount;
use solana_program_test::tokio;
use solana_program_test::tokio::time::{self, Duration};
use fixtures::{
    test::TestFixture,
    points::PointsFixture,
};
use solana_sdk::signature::Keypair;
use points_program::AccountBalances;

#[tokio::test]
async fn initialize_global_points() -> Result<(), anyhow::Error> {
    let test_f = TestFixture::new(None).await;
    let points_f = PointsFixture::new(std::rc::Rc::clone(&test_f.context), Keypair::new());

    points_f.try_initialize_global_points().await?;

    Ok(())
}

#[tokio::test]
async fn initialize_points_account_single() -> Result<(), anyhow::Error> {
    let test_f = TestFixture::new(None).await;
    let points_f = PointsFixture::new(std::rc::Rc::clone(&test_f.context), Keypair::new());

    points_f.try_initialize_global_points().await?;

    let mfi_account_f = test_f.create_marginfi_account().await;

    points_f.try_initialize_points_account(mfi_account_f.key).await?;

    Ok(())
}

#[tokio::test]
async fn accrue_points_single() -> Result<(), anyhow::Error> {
    let test_f = TestFixture::new(Some(fixtures::test::TestSettings {
        banks: vec![fixtures::test::TestBankSetting {
            mint: fixtures::test::BankMint::USDC,
            ..fixtures::test::TestBankSetting::default()
        }],
        ..fixtures::test::TestSettings::default()
    }))
    .await;
    let points_f = PointsFixture::new(std::rc::Rc::clone(&test_f.context), Keypair::new());

    points_f.try_initialize_global_points().await?;

    let mfi_account_f = test_f.create_marginfi_account().await;

    points_f.try_initialize_points_account(mfi_account_f.key).await?;

    // Deposit some funds so there's actually points to accrue
    let usdc_bank_f = test_f.get_bank(&fixtures::test::BankMint::USDC);
    let token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(2_000)
        .await;
    mfi_account_f
        .try_bank_deposit(token_account_usdc.key, usdc_bank_f, 2_000)
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

    Ok(())
}