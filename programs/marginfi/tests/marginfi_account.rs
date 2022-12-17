#![cfg(feature = "test-bpf")]
#![allow(dead_code)]

mod fixtures;

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use fixtures::prelude::*;
use marginfi::{
    prelude::MarginfiError,
    state::{marginfi_account::MarginfiAccount, marginfi_group::BankConfig},
};
use pretty_assertions::assert_eq;
use solana_program::{
    instruction::Instruction,
    system_instruction::{self},
    system_program,
};
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use crate::fixtures::marginfi_account::MarginfiAccountFixture;

#[tokio::test]
async fn success_create_marginfi_account() {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Create & initialize marginfi account
    let marginfi_account_key = Keypair::new();

    let accounts = marginfi::accounts::InitializeMarginfiAccount {
        marginfi_group: test_f.marginfi_group.key,
        marginfi_account: marginfi_account_key.pubkey(),
        signer: test_f.payer(),
        system_program: system_program::id(),
    };
    let init_marginfi_account_ix = Instruction {
        program_id: marginfi::id(),
        accounts: accounts.to_account_metas(Some(true)),
        data: marginfi::instruction::InitializeMarginfiAccount {}.data(),
    };

    let size = MarginfiAccountFixture::get_size();
    let create_marginfi_account_ix = system_instruction::create_account(
        &*&test_f.payer(),
        &marginfi_account_key.pubkey(),
        test_f.get_minimum_rent_for_size(size).await,
        size as u64,
        &marginfi::id(),
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_marginfi_account_ix, init_marginfi_account_ix],
        Some(&test_f.payer()),
        &[&test_f.payer_keypair(), &marginfi_account_key],
        test_f.get_latest_blockhash().await,
    );

    test_f
        .context
        .borrow_mut()
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap();

    // Fetch & deserialize marginfi account
    let ai = test_f
        .context
        .borrow_mut()
        .banks_client
        .get_account(marginfi_account_key.pubkey())
        .await
        .unwrap()
        .unwrap();
    let marginfi_account = MarginfiAccount::try_deserialize(&mut ai.data.as_slice()).unwrap();

    // Check basic properties
    assert_eq!(marginfi_account.group, test_f.marginfi_group.key);
    assert_eq!(marginfi_account.owner, test_f.payer());
    assert!(marginfi_account
        .lending_account
        .balances
        .iter()
        .all(|bank| bank.is_none()));
}

#[tokio::test]
async fn success_deposit() {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let mut bank_asset_mint_f = MintFixture::new(test_f.context.clone()).await;

    let sample_bank_index = 8;
    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            bank_asset_mint_f.key,
            sample_bank_index,
            *DEFAULT_TEST_BANK_CONFIG,
        )
        .await;
    assert!(res.is_ok());

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.context.borrow().payer.pubkey();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &bank_asset_mint_f.key, &owner).await;

    bank_asset_mint_f
        .mint_to(&token_account_f.key, native!(1_000, "USDC"))
        .await;

    let res = marginfi_account_f
        .try_bank_deposit(
            bank_asset_mint_f.key,
            token_account_f.key,
            native!(1_000, "USDC"),
        )
        .await;

    assert!(res.is_ok());

    let marginfi_account = marginfi_account_f.load().await;
    let marginfi_group = test_f.marginfi_group.load().await;

    // Check balance is active
    assert!(marginfi_account
        .lending_account
        .get_balance(&bank_asset_mint_f.key, &marginfi_group.lending_pool.banks)
        .is_some());
    assert_eq!(
        marginfi_account
            .lending_account
            .get_active_balances_iter()
            .collect::<Vec<_>>()
            .len(),
        1
    );
}

#[tokio::test]
async fn failure_deposit_cpacity_exceeded() {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let mut bank_asset_mint_f = MintFixture::new(test_f.context.clone()).await;

    let sample_bank_index = 8;
    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            bank_asset_mint_f.key,
            sample_bank_index,
            BankConfig {
                pyth_oracle: PYTH_USDC_FEED,
                max_capacity: native!(100, "USDC"),
                ..Default::default()
            },
        )
        .await;
    assert!(res.is_ok());

    // Fund user account
    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.context.borrow().payer.pubkey();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &bank_asset_mint_f.key, &owner).await;

    bank_asset_mint_f
        .mint_to(&token_account_f.key, native!(1_000, "USDC"))
        .await;

    // Make lawful deposit
    let res = marginfi_account_f
        .try_bank_deposit(
            bank_asset_mint_f.key,
            token_account_f.key,
            native!(99, "USDC"),
        )
        .await;
    assert!(res.is_ok());

    // Make unlawful deposit
    let res = marginfi_account_f
        .try_bank_deposit(
            bank_asset_mint_f.key,
            token_account_f.key,
            native!(101, "USDC"),
        )
        .await;
    assert_custom_error!(res.unwrap_err(), MarginfiError::BankDepositCapacityExceeded);
}
