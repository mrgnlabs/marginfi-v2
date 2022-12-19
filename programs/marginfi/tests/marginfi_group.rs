#![cfg(feature = "test-bpf")]
#![allow(dead_code)]

mod fixtures;

use anchor_lang::{InstructionData, ToAccountMetas};
use fixtures::prelude::*;
use marginfi::{
    prelude::{MarginfiError, MarginfiGroup},
    state::marginfi_group::BankConfig,
};
use pretty_assertions::assert_eq;
use solana_program::{
    instruction::Instruction,
    system_instruction::{self, SystemError},
    system_program,
};
use solana_program_test::*;
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
    transaction::{Transaction, TransactionError},
};

#[tokio::test]
async fn success_create_marginfi_group() -> anyhow::Result<()> {
    // Setup test executor
    let test_f = TestFixture::new(None).await;

    // Create & initialize marginfi group
    let marginfi_group_key = Keypair::new();

    let accounts = marginfi::accounts::InitializeMarginfiGroup {
        marginfi_group: marginfi_group_key.pubkey(),
        admin: test_f.payer(),
        system_program: system_program::id(),
    };
    let init_marginfi_group_ix = Instruction {
        program_id: marginfi::id(),
        accounts: accounts.to_account_metas(Some(true)),
        data: marginfi::instruction::InitializeMarginfiGroup {}.data(),
    };
    let size = MarginfiGroupFixture::get_size();
    let create_marginfi_group_ix = system_instruction::create_account(
        &test_f.payer(),
        &marginfi_group_key.pubkey(),
        test_f.get_minimum_rent_for_size(size).await,
        size as u64,
        &marginfi::id(),
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_marginfi_group_ix, init_marginfi_group_ix],
        Some(&test_f.payer().clone()),
        &[&test_f.payer_keypair(), &marginfi_group_key],
        test_f.get_latest_blockhash().await,
    );
    let res = test_f
        .context
        .borrow_mut()
        .banks_client
        .process_transaction(tx)
        .await;
    assert!(res.is_ok());

    // Fetch & deserialize marginfi group account
    let marginfi_group: MarginfiGroup = test_f
        .load_and_deserialize(&marginfi_group_key.pubkey())
        .await;

    // Check basic properties
    assert_eq!(marginfi_group.admin, test_f.payer());
    assert!(marginfi_group
        .lending_pool
        .banks
        .iter()
        .all(|bank| bank.is_none()));

    Ok(())
}

// #[tokio::test]
// async fn success_configure_marginfi_group() {
//     todo!()
// }

#[tokio::test]
async fn success_add_bank() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None).await;

    let new_bank_index = 0;
    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            bank_asset_mint_fixture.key,
            new_bank_index,
            *DEFAULT_USDC_TEST_BANK_CONFIG,
        )
        .await;
    assert!(res.is_ok());

    let marginfi_group = test_f.marginfi_group.load().await;

    // Check bank is active
    assert!(marginfi_group
        .lending_pool
        .get_bank(&bank_asset_mint_fixture.key)
        .is_some());
    assert_eq!(
        marginfi_group
            .lending_pool
            .banks
            .iter()
            .filter(|bank| bank.is_some())
            .collect::<Vec<_>>()
            .len(),
        1
    );

    Ok(())
}

#[tokio::test]
async fn failure_add_bank_fake_pyth_feed() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None).await;

    let new_bank_index = 0;
    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            bank_asset_mint_fixture.key,
            new_bank_index,
            BankConfig {
                pyth_oracle: FAKE_PYTH_USDC_FEED,
                ..Default::default()
            },
        )
        .await;
    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::InvalidPythAccount);

    Ok(())
}

#[tokio::test]
async fn failure_add_bank_already_exists() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None).await;

    let new_bank_index = 0;
    test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            bank_asset_mint_fixture.key,
            new_bank_index,
            *DEFAULT_USDC_TEST_BANK_CONFIG,
        )
        .await?;
    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            bank_asset_mint_fixture.key,
            new_bank_index,
            BankConfig::default(),
        )
        .await;

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, SystemError::AccountAlreadyInUse.into())
    );

    Ok(())
}
