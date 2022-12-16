#![cfg(feature = "test-bpf")]
#![allow(dead_code)]

mod fixtures;

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
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
use std::{borrow::BorrowMut, mem::size_of_val};

#[tokio::test]
async fn success_create_marginfi_group() {
    // Setup test executor
    let program = ProgramTest::new("marginfi", marginfi::ID, processor!(marginfi::entry));
    let mut context = program.start_with_context().await;

    // Create & initialize marginfi group
    let group_key = Keypair::new();

    let accounts = marginfi::accounts::InitializeMarginfiGroup {
        marginfi_group: group_key.pubkey(),
        admin: context.payer.pubkey(),
        system_program: system_program::id(),
    };
    let init_marginfi_group_ix = Instruction {
        program_id: marginfi::id(),
        accounts: accounts.to_account_metas(Some(true)),
        data: marginfi::instruction::InitializeMarginfiGroup {}.data(),
    };
    let rent = context.banks_client.get_rent().await.unwrap();
    let size = MarginfiGroupFixture::get_size();
    let create_marginfi_group_ix = system_instruction::create_account(
        &context.payer.pubkey(),
        &group_key.pubkey(),
        rent.minimum_balance(size),
        size as u64,
        &marginfi::id(),
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_marginfi_group_ix, init_marginfi_group_ix],
        Some(&context.payer.pubkey().clone()),
        &[&context.payer, &group_key],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Fetch & deserialize marginfi group account
    let ai = context
        .borrow_mut()
        .banks_client
        .get_account(group_key.pubkey())
        .await
        .unwrap()
        .unwrap();
    let marginfi_group = MarginfiGroup::try_deserialize(&mut ai.data.as_slice()).unwrap();

    // Check basic properties
    assert_eq!(marginfi_group.admin, context.payer.pubkey());
    assert!(marginfi_group
        .lending_pool
        .banks
        .iter()
        .all(|bank| bank.is_none()));
}

// #[tokio::test]
// async fn success_configure_marginfi_group() {
//     todo!()
// }

#[tokio::test]
async fn success_add_bank() {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context).await;

    let new_bank_index = 0;
    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            bank_asset_mint_fixture.key,
            new_bank_index,
            *DEFAULT_TEST_BANK_CONFIG,
        )
        .await;
    assert!(res.is_ok());

    let marginfi_group = test_f.marginfi_group.load().await;

    // Check bank is active
    assert!(!marginfi_group
        .lending_pool
        .get_bank(&bank_asset_mint_fixture.key)
        .is_none());
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
}

#[tokio::test]
async fn failure_add_bank_fake_pyth_feed() {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context).await;

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
}

#[tokio::test]
async fn failure_add_bank_already_exists() {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context).await;

    let marginfi_group = test_f.marginfi_group.load().await;
    println!(
        "{:?} - {}",
        marginfi_group,
        size_of_val(&marginfi_group.lending_pool.banks)
    );

    let new_bank_index = 0;
    test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            bank_asset_mint_fixture.key,
            new_bank_index,
            *DEFAULT_TEST_BANK_CONFIG,
        )
        .await
        .unwrap();
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
}
