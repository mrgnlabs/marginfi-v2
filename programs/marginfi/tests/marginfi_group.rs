#![cfg(feature = "test-bpf")]
#![allow(dead_code)]

mod fixtures;

use fixtures::prelude::*;
use marginfi::state::marginfi_group::{BankConfig, GroupConfig};
use pretty_assertions::assert_eq;
use solana_program_test::*;

#[tokio::test]
async fn success_create_marginfi_group() {
    let test_f = TestFixture::new(Some(GroupConfig {
        admin: None,
        ..Default::default()
    }))
    .await;

    let marginfi_group = test_f.marginfi_group.load().await;

    assert_eq!(marginfi_group.admin, test_f.payer());
}

/// Tests the `lending_pool_add_bank` instruction
#[tokio::test]
async fn success_add_bank() {
    let test_f = TestFixture::new(Some(GroupConfig {
        admin: None,
        ..Default::default()
    }))
    .await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context).await;
    
    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(bank_asset_mint_fixture.key, 0, BankConfig::default())
        .await;
}
