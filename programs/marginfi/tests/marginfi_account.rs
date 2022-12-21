#![cfg(feature = "test-bpf")]
#![allow(dead_code)]

mod fixtures;

use crate::fixtures::marginfi_account::MarginfiAccountFixture;
use anchor_lang::{prelude::ErrorCode, InstructionData, ToAccountMetas};
use fixtures::prelude::*;
use marginfi::{
    prelude::MarginfiError,
    state::{
        marginfi_account::MarginfiAccount,
        marginfi_group::{BankConfig, BankVaultType},
    },
};
use pretty_assertions::assert_eq;
use solana_program::{instruction::Instruction, system_instruction, system_program};
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

#[tokio::test]
async fn success_create_marginfi_account() -> anyhow::Result<()> {
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

    let res = test_f
        .context
        .borrow_mut()
        .banks_client
        .process_transaction(tx)
        .await;
    assert!(res.is_ok());

    // Fetch & deserialize marginfi account
    let marginfi_account: MarginfiAccount = test_f
        .load_and_deserialize(&marginfi_account_key.pubkey())
        .await;

    // Check basic properties
    assert_eq!(marginfi_account.group, test_f.marginfi_group.key);
    assert_eq!(marginfi_account.owner, test_f.payer());
    assert!(marginfi_account
        .lending_account
        .balances
        .iter()
        .all(|bank| bank.is_none()));

    Ok(())
}

#[tokio::test]
async fn success_deposit() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let mut usdc_mint_f = MintFixture::new(test_f.context.clone(), None, None).await;

    let sample_bank_index = 8;
    test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            usdc_mint_f.key,
            sample_bank_index,
            *DEFAULT_USDC_TEST_BANK_CONFIG,
        )
        .await?;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.context.borrow().payer.pubkey();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &usdc_mint_f.key, &owner).await;

    usdc_mint_f
        .mint_to(&token_account_f.key, native!(1_000, "USDC"))
        .await;

    let res = marginfi_account_f
        .try_bank_deposit(usdc_mint_f.key, token_account_f.key, native!(1_000, "USDC"))
        .await;
    assert!(res.is_ok());

    let marginfi_account = marginfi_account_f.load().await;
    let marginfi_group = test_f.marginfi_group.load().await;

    // Check balance is active
    assert!(marginfi_account
        .lending_account
        .get_balance(&usdc_mint_f.key, &marginfi_group.lending_pool.banks)
        .is_some());
    assert_eq!(
        marginfi_account
            .lending_account
            .get_active_balances_iter()
            .collect::<Vec<_>>()
            .len(),
        1
    );

    Ok(())
}

#[tokio::test]
async fn failure_deposit_capacity_exceeded() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let mut usdc_mint_f = MintFixture::new(test_f.context.clone(), None, None).await;

    let sample_bank_index = 8;
    test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            usdc_mint_f.key,
            sample_bank_index,
            BankConfig {
                pyth_oracle: PYTH_USDC_FEED,
                max_capacity: native!(100, "USDC"),
                ..Default::default()
            },
        )
        .await?;

    // Fund user account
    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.context.borrow().payer.pubkey();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &usdc_mint_f.key, &owner).await;

    usdc_mint_f
        .mint_to(&token_account_f.key, native!(1_000, "USDC"))
        .await;

    // Make lawful deposit
    let res = marginfi_account_f
        .try_bank_deposit(usdc_mint_f.key, token_account_f.key, native!(99, "USDC"))
        .await;
    assert!(res.is_ok());

    // Make unlawful deposit
    let res = marginfi_account_f
        .try_bank_deposit(usdc_mint_f.key, token_account_f.key, native!(101, "USDC"))
        .await;
    assert_custom_error!(res.unwrap_err(), MarginfiError::BankDepositCapacityExceeded);

    Ok(())
}

#[tokio::test]
async fn failure_deposit_bank_not_found() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_mint_f = MintFixture::new(test_f.context.clone(), None, None).await;
    let mut sol_mint_f = MintFixture::new(test_f.context.clone(), None, Some(9)).await;

    let sample_bank_index = 8;
    test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            usdc_mint_f.key,
            sample_bank_index,
            BankConfig {
                pyth_oracle: PYTH_USDC_FEED,
                max_capacity: native!(100, "USDC"),
                ..Default::default()
            },
        )
        .await?;

    // Fund user account
    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.context.borrow().payer.pubkey();
    let sol_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &sol_mint_f.key, &owner).await;

    sol_mint_f
        .mint_to(&sol_token_account_f.key, native!(1_000, "SOL"))
        .await;

    let res = marginfi_account_f
        .try_bank_deposit(sol_mint_f.key, sol_token_account_f.key, native!(1, "SOL"))
        .await;
    assert_anchor_error!(res.unwrap_err(), ErrorCode::AccountNotInitialized);

    Ok(())
}

#[tokio::test]
async fn success_borrow() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let mut test_f = TestFixture::new(None).await;

    test_f
        .marginfi_group
        .try_lending_pool_add_bank(test_f.usdc_mint.key, 0, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;
    test_f
        .marginfi_group
        .try_lending_pool_add_bank(test_f.sol_mint.key, 1, *DEFAULT_SOL_TEST_BANK_CONFIG)
        .await?;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.context.borrow().payer.pubkey();
    let usdc_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &owner).await;
    let sol_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.sol_mint.key, &owner).await;

    test_f
        .usdc_mint
        .mint_to(&usdc_token_account_f.key, native!(1_000, "USDC"))
        .await;
    let liquidity_vault = find_bank_vault_pda(
        &test_f.marginfi_group.key,
        &test_f.sol_mint.key,
        BankVaultType::Liquidity,
    );

    test_f
        .sol_mint
        .mint_to(&liquidity_vault.0, native!(1_000_000, "SOL"))
        .await;

    marginfi_account_f
        .try_bank_deposit(
            test_f.usdc_mint.key,
            usdc_token_account_f.key,
            native!(1_000, "USDC"),
        )
        .await?;

    let res = marginfi_account_f
        .try_bank_withdraw(
            test_f.sol_mint.key,
            sol_token_account_f.key,
            native!(2, "SOL"),
        )
        .await;
    assert!(res.is_ok());

    // Check token balances are correct
    assert_eq!(usdc_token_account_f.balance().await, native!(0, "USDC"));
    assert_eq!(sol_token_account_f.balance().await, native!(2, "SOL"));

    // TODO: check health is sane

    Ok(())
}

#[tokio::test]
async fn failure_borrow_not_enough_collateral() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let mut usdc_mint_f = MintFixture::new(test_f.context.clone(), None, None).await;
    let mut sol_mint_f = MintFixture::new(test_f.context.clone(), None, Some(9)).await;

    test_f
        .marginfi_group
        .try_lending_pool_add_bank(usdc_mint_f.key, 0, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;
    test_f
        .marginfi_group
        .try_lending_pool_add_bank(sol_mint_f.key, 1, *DEFAULT_SOL_TEST_BANK_CONFIG)
        .await?;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.context.borrow().payer.pubkey();
    let usdc_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &usdc_mint_f.key, &owner).await;
    let sol_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &sol_mint_f.key, &owner).await;

    usdc_mint_f
        .mint_to(&usdc_token_account_f.key, native!(1_000, "USDC"))
        .await;

    let liquidity_vault = find_bank_vault_pda(
        &test_f.marginfi_group.key,
        &sol_mint_f.key,
        BankVaultType::Liquidity,
    );
    sol_mint_f
        .mint_to(&liquidity_vault.0, native!(1_000_000, "SOL"))
        .await;

    marginfi_account_f
        .try_bank_deposit(
            usdc_mint_f.key,
            usdc_token_account_f.key,
            native!(1, "USDC"),
        )
        .await?;

    let res = marginfi_account_f
        .try_bank_withdraw(sol_mint_f.key, sol_token_account_f.key, native!(1, "SOL"))
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::BadAccountHealth);

    Ok(())
}
