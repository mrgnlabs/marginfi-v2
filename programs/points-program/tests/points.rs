use solana_program_test::tokio;
use fixtures::{
    test::TestFixture,
    points::PointsFixture,
};
use solana_sdk::signature::Keypair;

#[tokio::test]
async fn initialize_global_points() -> Result<(), anyhow::Error> {
    let test_f = TestFixture::new(None).await;
    let points_f = PointsFixture::new(test_f.context, Keypair::new());

    points_f.try_initialize_global_points().await?;

    Ok(())
}