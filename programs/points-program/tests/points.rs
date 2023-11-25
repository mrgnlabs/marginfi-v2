use solana_program_test::tokio;
use fixtures::{
    test::TestFixture,
    points::PointsFixture,
};
use solana_sdk::signature::Keypair;

#[tokio::test]
async fn initialize_global_points() -> Result<(), anyhow::Error> {
    let test_f = TestFixture::new(None).await;
    let points_f = PointsFixture::new(std::rc::Rc::clone(&test_f.context), Keypair::new());

    points_f.try_initialize_global_points().await?;

    Ok(())
}

#[tokio::test]
async fn initialize_points_account() -> Result<(), anyhow::Error> {
    let test_f = TestFixture::new(None).await;
    let points_f = PointsFixture::new(std::rc::Rc::clone(&test_f.context), Keypair::new());

    points_f.try_initialize_global_points().await?;

    let mfi_account_f = test_f.create_marginfi_account().await;

    points_f.try_initialize_points_account(mfi_account_f.key).await?;

    Ok(())
}

#[tokio::test]
async fn accrue_points() -> Result<(), anyhow::Error> {
    let test_f = TestFixture::new(None).await;
    let points_f = PointsFixture::new(std::rc::Rc::clone(&test_f.context), Keypair::new());

    points_f.try_initialize_global_points().await?;

    let mfi_account_f = test_f.create_marginfi_account().await;

    points_f.try_initialize_points_account(mfi_account_f.key).await?;

    points_f.try_accrue_points(0).await?;

    Ok(())
}