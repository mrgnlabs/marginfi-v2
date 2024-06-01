//! Pythnet crosschain native oracle implementation
//!
//! - Creating banks with pythnet oracles
//! - Update the native oracle with
//!
use std::error::Error;

use fixtures::test::{TestFixture, TestSettings};
use solana_program_test::tokio;

#[tokio::test]
pub async fn oracle_setup() -> Result<(), Box<dyn Error>> {
    let test = TestFixture::new(Some(TestSettings::all_banks_one_native())).await;

    Ok(())
}
