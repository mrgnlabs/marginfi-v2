#![cfg(feature = "test-bpf")]
#![allow(dead_code)]

mod fixtures;

use anchor_lang::{prelude::Clock, InstructionData, ToAccountMetas};
use anchor_spl::token::{self};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::prelude::*;
use marginfi::{
    prelude::{MarginfiError, MarginfiGroup},
    state::marginfi_group::{BankConfig, InterestRateConfig},
};
use pretty_assertions::assert_eq;
use solana_program::{
    instruction::Instruction,
    program_pack::Pack,
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

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

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
        .find_bank_by_mint(&bank_asset_mint_fixture.key)
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

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

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

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

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

#[tokio::test]
async fn success_accrue_interest_rates_1() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    let usdc_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;
    let sol_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

    test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            usdc_mint_fixture.key,
            0,
            BankConfig {
                interest_rate_config: InterestRateConfig {
                    optimal_utilization_rate: I80F48!(0.9).into(),
                    plateau_interest_rate: I80F48!(1).into(),
                    ..Default::default()
                },
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await?;

    test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            sol_mint_fixture.key,
            1,
            BankConfig {
                deposit_weight_init: I80F48!(1).into(),
                ..*DEFAULT_SOL_TEST_BANK_CONFIG
            },
        )
        .await?;

    let lender_account = test_f.create_marginfi_account().await;
    let funding_account = usdc_mint_fixture
        .create_and_mint_to(native!(100, "USDC"))
        .await;
    lender_account
        .try_bank_deposit(usdc_mint_fixture.key, funding_account, native!(100, "USDC"))
        .await?;

    let borrower_account = test_f.create_marginfi_account().await;
    let funding_account = sol_mint_fixture
        .create_and_mint_to(native!(1000, "SOL"))
        .await;
    borrower_account
        .try_bank_deposit(sol_mint_fixture.key, funding_account, native!(999, "SOL"))
        .await?;

    let destination_account = usdc_mint_fixture.create_and_mint_to(0).await;
    borrower_account
        .try_bank_withdraw(
            usdc_mint_fixture.key,
            destination_account,
            native!(90, "USDC"),
        )
        .await?;

    {
        let mut ctx = test_f.context.borrow_mut();
        let mut clock: Clock = ctx.banks_client.get_sysvar().await?;
        // Advance clock by 1 year
        clock.unix_timestamp += 365 * 24 * 60 * 60;
        ctx.set_sysvar(&clock);
    }

    test_f
        .marginfi_group
        .try_accrue_interest(&usdc_mint_fixture.key, 0)
        .await?;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_bank_account = borrower_mfi_account.lending_account.balances[1].unwrap();
    let marginfi_group = test_f.marginfi_group.load().await;
    let bank = marginfi_group.lending_pool.banks[0].unwrap();
    let liabilities = bank.get_liability_amount(borrower_bank_account.liability_shares.into())?;

    let lender_mfi_account = lender_account.load().await;
    let lender_bank_account = lender_mfi_account.lending_account.balances[0].unwrap();
    let deposits = bank.get_deposit_amount(lender_bank_account.deposit_shares.into())?;

    assert_eq_noise!(
        liabilities,
        I80F48::from(native!(180, "USDC")),
        I80F48!(100)
    );
    assert_eq_noise!(deposits, I80F48::from(native!(190, "USDC")), I80F48!(100));

    Ok(())
}

#[tokio::test]
async fn success_accrue_interest_rates_2() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    let usdc_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;
    let sol_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

    test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            usdc_mint_fixture.key,
            0,
            BankConfig {
                interest_rate_config: InterestRateConfig {
                    optimal_utilization_rate: I80F48!(0.9).into(),
                    plateau_interest_rate: I80F48!(1).into(),
                    protocol_fixed_fee_apr: I80F48!(0.01).into(),
                    insurance_fee_fixed_apr: I80F48!(0.01).into(),
                    ..Default::default()
                },
                max_capacity: native!(1_000_000_000, "USDC").into(),
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await?;

    test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            sol_mint_fixture.key,
            1,
            BankConfig {
                deposit_weight_init: I80F48!(1).into(),
                max_capacity: native!(200_000_000, "SOL").into(),
                ..*DEFAULT_SOL_TEST_BANK_CONFIG
            },
        )
        .await?;

    let lender_account = test_f.create_marginfi_account().await;
    let funding_account = usdc_mint_fixture
        .create_and_mint_to(native!(100_000_000, "USDC"))
        .await;
    lender_account
        .try_bank_deposit(
            usdc_mint_fixture.key,
            funding_account,
            native!(100_000_000, "USDC"),
        )
        .await?;

    let borrower_account = test_f.create_marginfi_account().await;
    let funding_account = sol_mint_fixture
        .create_and_mint_to(native!(10_000_000, "SOL"))
        .await;
    borrower_account
        .try_bank_deposit(
            sol_mint_fixture.key,
            funding_account,
            native!(10_000_000, "SOL"),
        )
        .await?;

    let destination_account = usdc_mint_fixture.create_and_mint_to(0).await;
    borrower_account
        .try_bank_withdraw(
            usdc_mint_fixture.key,
            destination_account,
            native!(90_000_000, "USDC"),
        )
        .await?;

    {
        let mut ctx = test_f.context.borrow_mut();
        let mut clock: Clock = ctx.banks_client.get_sysvar().await?;
        // Advance clock by 1 year
        clock.unix_timestamp += 60;
        ctx.set_sysvar(&clock);
    }

    test_f
        .marginfi_group
        .try_accrue_interest(&usdc_mint_fixture.key, 0)
        .await?;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_bank_account = borrower_mfi_account.lending_account.balances[1].unwrap();
    let marginfi_group = test_f.marginfi_group.load().await;
    let bank = marginfi_group.lending_pool.banks[0].unwrap();
    let liabilities = bank.get_liability_amount(borrower_bank_account.liability_shares.into())?;

    let lender_mfi_account = lender_account.load().await;
    let lender_bank_account = lender_mfi_account.lending_account.balances[0].unwrap();
    let deposits = bank.get_deposit_amount(lender_bank_account.deposit_shares.into())?;

    assert_eq_noise!(liabilities, I80F48!(90000174657530), I80F48!(10));
    assert_eq_noise!(deposits, I80F48!(100000171232862), I80F48!(10));

    let mut ctx = test_f.context.borrow_mut();
    let protocol_fees = ctx.banks_client.get_account(bank.fee_vault).await?.unwrap();
    let insurance_fees = ctx
        .banks_client
        .get_account(bank.insurance_vault)
        .await?
        .unwrap();

    let protocol_fees =
        token::spl_token::state::Account::unpack_from_slice(protocol_fees.data.as_slice())?;
    let insurance_fees =
        token::spl_token::state::Account::unpack_from_slice(insurance_fees.data.as_slice())?;

    assert_eq!(protocol_fees.amount, 1712326);
    assert_eq!(insurance_fees.amount, 1712326);

    Ok(())
}
