use anchor_lang::{prelude::Clock, InstructionData, ToAccountMetas};

use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::prelude::*;
use fixtures::{assert_custom_error, assert_eq_noise, native};
use marginfi::constants::TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE;
use marginfi::prelude::GroupConfig;
use marginfi::state::marginfi_group::{BankVaultType, InterestRateConfig};
use marginfi::{
    prelude::{MarginfiError, MarginfiGroup},
    state::marginfi_group::{Bank, BankConfig, BankConfigOpt, BankOperationalState},
};
use pretty_assertions::assert_eq;

use solana_program::{instruction::Instruction, system_program};
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

#[tokio::test]
async fn marginfi_group_create_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    // Create & initialize marginfi group
    let marginfi_group_key = Keypair::new();

    let accounts = marginfi::accounts::MarginfiGroupInitialize {
        marginfi_group: marginfi_group_key.pubkey(),
        admin: test_f.payer(),
        system_program: system_program::id(),
    };
    let init_marginfi_group_ix = Instruction {
        program_id: marginfi::id(),
        accounts: accounts.to_account_metas(Some(true)),
        data: marginfi::instruction::MarginfiGroupInitialize {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[init_marginfi_group_ix],
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

    Ok(())
}

#[tokio::test]
async fn marginfi_group_config_check() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    // Create & initialize marginfi group
    let marginfi_group_key = Keypair::new();

    let accounts = marginfi::accounts::MarginfiGroupInitialize {
        marginfi_group: marginfi_group_key.pubkey(),
        admin: test_f.payer(),
        system_program: system_program::id(),
    };
    let init_marginfi_group_ix = Instruction {
        program_id: marginfi::id(),
        accounts: accounts.to_account_metas(Some(true)),
        data: marginfi::instruction::MarginfiGroupInitialize {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[init_marginfi_group_ix],
        Some(&test_f.payer().clone()),
        &[&test_f.payer_keypair(), &marginfi_group_key],
        test_f.get_latest_blockhash().await,
    );
    test_f
        .context
        .borrow_mut()
        .banks_client
        .process_transaction(tx)
        .await?;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

    let bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&bank_asset_mint_fixture, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;

    let res = bank
        .update_config(BankConfigOpt {
            interest_rate_config: Some(marginfi::state::marginfi_group::InterestRateConfigOpt {
                optimal_utilization_rate: Some(I80F48::from_num(0.91).into()),
                plateau_interest_rate: Some(I80F48::from_num(0.44).into()),
                max_interest_rate: Some(I80F48::from_num(1.44).into()),
                insurance_fee_fixed_apr: Some(I80F48::from_num(0.13).into()),
                insurance_ir_fee: Some(I80F48::from_num(0.11).into()),
                protocol_fixed_fee_apr: Some(I80F48::from_num(0.51).into()),
                protocol_ir_fee: Some(I80F48::from_num(0.011).into()),
            }),
            ..BankConfigOpt::default()
        })
        .await;

    assert!(res.is_ok());

    // Load bank and check each property in config matches

    let bank: Bank = test_f.load_and_deserialize(&bank.key).await;

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.optimal_utilization_rate),
        I80F48::from_num(0.91)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.plateau_interest_rate),
        I80F48::from_num(0.44)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.max_interest_rate),
        I80F48::from_num(1.44)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_fee_fixed_apr),
        I80F48::from_num(0.13)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_ir_fee),
        I80F48::from_num(0.11)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_fixed_fee_apr),
        I80F48::from_num(0.51)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_ir_fee),
        I80F48::from_num(0.011)
    );

    Ok(())
}

#[tokio::test]
async fn marginfi_group_add_bank_success() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&bank_asset_mint_fixture, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await;
    assert!(res.is_ok());

    // Check bank is active
    let bank = res.unwrap();
    let bank = test_f.try_load(&bank.key).await?;
    assert!(bank.is_some());

    Ok(())
}

#[tokio::test]
async fn marginfi_group_add_bank_with_seed_success() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;
    let bank_seed = 1200_u64;

    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank_with_seed(
            &bank_asset_mint_fixture,
            *DEFAULT_USDC_TEST_BANK_CONFIG,
            bank_seed,
        )
        .await;
    assert!(res.is_ok());

    // Check bank is active
    let bank = res.unwrap();
    let bank = test_f.try_load(&bank.key).await?;
    assert!(bank.is_some());

    Ok(())
}

#[tokio::test]
async fn marginfi_group_add_bank_failure_fake_pyth_feed() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            &bank_asset_mint_fixture,
            BankConfig {
                oracle_setup: marginfi::state::price::OracleSetup::PythEma,
                oracle_keys: create_oracle_key_array(FAKE_PYTH_USDC_FEED),
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::InvalidOracleAccount);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_accrue_interest_rates_success_1() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        group_config: Some(GroupConfig { admin: None }),
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: Some(BankConfig {
                    interest_rate_config: InterestRateConfig {
                        optimal_utilization_rate: I80F48!(0.9).into(),
                        plateau_interest_rate: I80F48!(1).into(),
                        ..*DEFAULT_TEST_BANK_INTEREST_RATE_CONFIG
                    },
                    ..*DEFAULT_USDC_TEST_BANK_CONFIG
                }),
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(100).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 999)
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 90)
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
        .try_accrue_interest(usdc_bank_f)
        .await?;

    let borrower_mfi_account = borrower_mfi_account_f.load().await;
    let borrower_bank_account = borrower_mfi_account.lending_account.balances[1];
    let usdc_bank: Bank = usdc_bank_f.load().await;
    let liabilities =
        usdc_bank.get_liability_amount(borrower_bank_account.liability_shares.into())?;

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let lender_bank_account = lender_mfi_account.lending_account.balances[0];
    let assets = usdc_bank.get_asset_amount(lender_bank_account.asset_shares.into())?;

    assert_eq_noise!(
        liabilities,
        I80F48::from(native!(180, "USDC")),
        I80F48!(100)
    );
    assert_eq_noise!(assets, I80F48::from(native!(190, "USDC")), I80F48!(100));

    Ok(())
}

#[tokio::test]
async fn marginfi_group_accrue_interest_rates_success_2() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: Some(BankConfig {
                    deposit_limit: native!(1_000_000_000, "USDC"),
                    interest_rate_config: InterestRateConfig {
                        optimal_utilization_rate: I80F48!(0.9).into(),
                        plateau_interest_rate: I80F48!(1).into(),
                        protocol_fixed_fee_apr: I80F48!(0.01).into(),
                        insurance_fee_fixed_apr: I80F48!(0.01).into(),
                        ..*DEFAULT_TEST_BANK_INTEREST_RATE_CONFIG
                    },
                    ..*DEFAULT_USDC_TEST_BANK_CONFIG
                }),
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    deposit_limit: native!(200_000_000, "SOL"),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(10_000_000)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 10_000_000)
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 90_000_000)
        .await?;

    // Advance clock by 1 minute
    {
        let mut ctx = test_f.context.borrow_mut();
        let mut clock: Clock = ctx.banks_client.get_sysvar().await?;
        clock.unix_timestamp += 60;
        ctx.set_sysvar(&clock);
    }

    test_f
        .marginfi_group
        .try_accrue_interest(usdc_bank_f)
        .await?;

    test_f.marginfi_group.try_collect_fees(usdc_bank_f).await?;

    let borrower_mfi_account = borrower_mfi_account_f.load().await;
    let borrower_bank_account = borrower_mfi_account.lending_account.balances[1];
    let usdc_bank = usdc_bank_f.load().await;
    let liabilities =
        usdc_bank.get_liability_amount(borrower_bank_account.liability_shares.into())?;

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let lender_bank_account = lender_mfi_account.lending_account.balances[0];
    let assets = usdc_bank.get_asset_amount(lender_bank_account.asset_shares.into())?;

    assert_eq_noise!(liabilities, I80F48!(90000174657530), I80F48!(10));
    assert_eq_noise!(assets, I80F48!(100000171232876), I80F48!(10));

    let protocol_fees = usdc_bank_f
        .get_vault_token_account(BankVaultType::Fee)
        .await;
    let insurance_fees = usdc_bank_f
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq!(protocol_fees.balance().await, 1712328);
    assert_eq!(insurance_fees.balance().await, 1712328);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_failure_not_bankrupt() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 1_001)
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 10_000)
        .await?;

    let res = test_f
        .marginfi_group
        .try_handle_bankruptcy(usdc_bank_f, &borrower_mfi_account_f)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountNotBankrupt);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_no_debt() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 1_001)
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 10_000)
        .await?;

    let mut borrower_mfi_account = borrower_mfi_account_f.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0;
    borrower_mfi_account_f
        .set_account(&borrower_mfi_account)
        .await?;

    let res = test_f
        .marginfi_group
        .try_handle_bankruptcy(sol_bank_f, &borrower_mfi_account_f)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::BalanceNotBadDebt);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_fully_insured() -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings {
        group_config: Some(GroupConfig { admin: None }),
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
    }))
    .await;

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(
            lender_token_account_usdc.key,
            test_f.get_bank(&BankMint::USDC),
            100_000,
        )
        .await?;

    let borrower_account = test_f.create_marginfi_account().await;
    let borrower_deposit_account = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;

    borrower_account
        .try_bank_deposit(
            borrower_deposit_account.key,
            test_f.get_bank(&BankMint::SOL),
            1_001,
        )
        .await?;

    let borrower_borrow_account = test_f.usdc_mint.create_token_account_and_mint_to(0).await;

    borrower_account
        .try_bank_borrow(
            borrower_borrow_account.key,
            test_f.get_bank(&BankMint::USDC),
            10_000,
        )
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0;
    borrower_account.set_account(&borrower_mfi_account).await?;

    {
        let (insurance_vault, _) = test_f
            .get_bank(&BankMint::USDC)
            .get_vault(BankVaultType::Insurance);
        test_f
            .get_bank_mut(&BankMint::USDC)
            .mint
            .mint_to(&insurance_vault, 10_000)
            .await;
    }

    test_f
        .marginfi_group
        .try_handle_bankruptcy(test_f.get_bank(&BankMint::USDC), &borrower_account)
        .await?;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let usdc_bank = test_f.get_bank(&BankMint::USDC).load().await;

    let lender_usdc_value = usdc_bank.get_asset_amount(
        lender_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    assert_eq_noise!(
        lender_usdc_value,
        I80F48::from(native!(100_000, "USDC")),
        I80F48::ONE
    );

    let insurance_amount = test_f
        .get_bank(&BankMint::USDC)
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq!(insurance_amount.balance().await, 0);

    // Test account is disabled

    // Deposit 1 SOL
    let res = borrower_account
        .try_bank_deposit(
            borrower_deposit_account.key,
            test_f.get_bank(&BankMint::SOL),
            1,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Withdraw 1 SOL
    let res = borrower_account
        .try_bank_withdraw(
            borrower_deposit_account.key,
            test_f.get_bank(&BankMint::SOL),
            1,
            None,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Borrow 1 USDC
    let res = borrower_account
        .try_bank_borrow(
            borrower_borrow_account.key,
            test_f.get_bank(&BankMint::USDC),
            1,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Repay 1 USDC
    let res = borrower_account
        .try_bank_repay(
            borrower_borrow_account.key,
            test_f.get_bank(&BankMint::USDC),
            1,
            None,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_partially_insured() -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(
            lender_token_account_usdc.key,
            test_f.get_bank(&BankMint::USDC),
            100_000,
        )
        .await?;

    let borrower_account = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_account
        .try_bank_deposit(
            borrower_token_account_sol.key,
            test_f.get_bank(&BankMint::SOL),
            1_001,
        )
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    borrower_account
        .try_bank_borrow(
            borrower_token_account_usdc.key,
            test_f.get_bank(&BankMint::USDC),
            10_000,
        )
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0;
    borrower_account.set_account(&borrower_mfi_account).await?;

    test_f
        .usdc_mint
        .mint_to(
            &test_f
                .get_bank(&BankMint::USDC)
                .load()
                .await
                .insurance_vault,
            5_000,
        )
        .await;

    test_f
        .marginfi_group
        .try_handle_bankruptcy(test_f.get_bank(&BankMint::USDC), &borrower_account)
        .await?;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let usdc_bank = test_f.get_bank(&BankMint::USDC).load().await;

    let lender_usdc_value = usdc_bank.get_asset_amount(
        lender_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    assert_eq_noise!(
        lender_usdc_value,
        I80F48::from(native!(95_000, "USDC")),
        I80F48::ONE
    );

    let insurance_amount = test_f
        .get_bank(&BankMint::USDC)
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq!(insurance_amount.balance().await, 0);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_not_insured() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000)
        .await?;

    let borrower_account = test_f.create_marginfi_account().await;
    let borrower_deposit_account = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_account
        .try_bank_deposit(borrower_deposit_account.key, sol_bank_f, 1_001)
        .await?;
    let borrower_borrow_account = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    borrower_account
        .try_bank_borrow(borrower_borrow_account.key, usdc_bank_f, 10_000)
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0;

    borrower_account.set_account(&borrower_mfi_account).await?;

    test_f
        .marginfi_group
        .try_handle_bankruptcy(usdc_bank_f, &borrower_account)
        .await?;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let usdc_bank = usdc_bank_f.load().await;

    let lender_usdc_value = usdc_bank.get_asset_amount(
        lender_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    assert_eq_noise!(
        lender_usdc_value,
        I80F48::from(native!(90_000, "USDC")),
        I80F48::ONE
    );

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_not_insured_3_depositors() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_1_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_1_token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_1_mfi_account_f
        .try_bank_deposit(lender_1_token_account.key, usdc_bank_f, 100_000)
        .await?;

    let lender_2_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_2_token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_2_mfi_account_f
        .try_bank_deposit(lender_2_token_account.key, usdc_bank_f, 100_000)
        .await?;

    let lender_3_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_3_token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_3_mfi_account_f
        .try_bank_deposit(lender_3_token_account.key, usdc_bank_f, 100_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 1_001)
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 10_000)
        .await?;

    let mut borrower_mfi_account = borrower_mfi_account_f.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0;

    borrower_mfi_account_f
        .set_account(&borrower_mfi_account)
        .await?;

    test_f
        .marginfi_group
        .try_handle_bankruptcy(usdc_bank_f, &borrower_mfi_account_f)
        .await?;

    let borrower_mfi_account = borrower_mfi_account_f.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_1_mfi_account = lender_1_mfi_account_f.load().await;
    let usdc_bank = usdc_bank_f.load().await;

    let lender_usdc_value = usdc_bank.get_asset_amount(
        lender_1_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    assert_eq_noise!(
        lender_usdc_value,
        I80F48::from(native!(96_666, "USDC")),
        I80F48::from(native!(1, "USDC"))
    );

    Ok(())
}

#[tokio::test]
async fn marginfi_group_bank_paused_should_error() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::USDC,
            config: None,
        }],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    test_f
        .marginfi_group
        .try_lending_pool_configure_bank(
            usdc_bank_f,
            BankConfigOpt {
                operational_state: Some(BankOperationalState::Paused),
                ..BankConfigOpt::default()
            },
        )
        .await?;

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    let res = lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::BankPaused);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_bank_reduce_only_withdraw_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::USDC,
            config: None,
        }],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000)
        .await?;

    test_f
        .set_bank_operational_state(usdc_bank_f, BankOperationalState::ReduceOnly)
        .await
        .unwrap();

    let res = lender_mfi_account_f
        .try_bank_withdraw(lender_token_account_usdc.key, usdc_bank_f, 0, Some(true))
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_group_bank_reduce_only_deposit_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_1_mfi_account = test_f.create_marginfi_account().await;
    let lender_1_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    lender_1_mfi_account
        .try_bank_deposit(lender_1_token_account_sol.key, sol_bank_f, 100)
        .await?;

    let lender_2_mfi_account = test_f.create_marginfi_account().await;
    let lender_2_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_2_mfi_account
        .try_bank_deposit(lender_2_token_account_usdc.key, usdc_bank_f, 100_000)
        .await?;

    let lender_2_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    lender_2_mfi_account
        .try_bank_borrow(lender_2_token_account_sol.key, sol_bank_f, 1)
        .await?;

    test_f
        .set_bank_operational_state(usdc_bank_f, BankOperationalState::ReduceOnly)
        .await
        .unwrap();

    let res = lender_2_mfi_account
        .try_bank_repay(lender_2_token_account_sol.key, sol_bank_f, 1, None)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_group_bank_reduce_only_borrow_failure() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_mfi_account = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    lender_mfi_account
        .try_bank_deposit(lender_token_account_sol.key, sol_bank_f, 100)
        .await?;

    let borrower_mfi_account = test_f.create_marginfi_account().await;
    let borrower_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    borrower_mfi_account
        .try_bank_deposit(borrower_token_account_usdc.key, usdc_bank_f, 100_000)
        .await?;

    test_f
        .set_bank_operational_state(sol_bank_f, BankOperationalState::ReduceOnly)
        .await
        .unwrap();

    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    let res = borrower_mfi_account
        .try_bank_borrow(borrower_token_account_sol.key, sol_bank_f, 1)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::BankReduceOnly);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_bank_reduce_only_deposit_failure() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::USDC,
            config: None,
        }],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    test_f
        .set_bank_operational_state(usdc_bank_f, BankOperationalState::ReduceOnly)
        .await
        .unwrap();

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;

    let res = lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::BankReduceOnly);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_init_limit_0() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    usdc_bank
        .update_config(BankConfigOpt {
            total_asset_value_init_limit: Some(101),
            ..BankConfigOpt::default()
        })
        .await?;

    let sol_depositor = test_f.create_marginfi_account().await;
    let usdc_depositor = test_f.create_marginfi_account().await;

    let sol_token_account = test_f.sol_mint.create_token_account_and_mint_to(100).await;

    sol_depositor
        .try_bank_deposit(sol_token_account.key, sol_bank, 100)
        .await?;

    let usdc_token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;

    sol_depositor
        .try_bank_deposit(usdc_token_account.key, usdc_bank, 1900)
        .await?;

    usdc_depositor
        .try_bank_deposit(usdc_token_account.key, usdc_bank, 100)
        .await?;

    // Borrowing 10 SOL should fail bc of init limit
    let depositor_sol_account = sol_bank.mint.create_token_account_and_mint_to(0).await;
    let res = usdc_depositor
        .try_bank_borrow(depositor_sol_account.key, sol_bank, 9.9)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::BadAccountHealth);

    sol_depositor
        .try_bank_withdraw(usdc_token_account.key, usdc_bank, 1900, Some(true))
        .await?;

    // Borrowing 10 SOL should succeed now
    let res = usdc_depositor
        .try_bank_borrow(depositor_sol_account.key, sol_bank, 10)
        .await;

    usdc_bank
        .update_config(BankConfigOpt {
            total_asset_value_init_limit: Some(TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE),
            ..BankConfigOpt::default()
        })
        .await?;

    assert!(res.is_ok());

    sol_depositor
        .try_bank_deposit(usdc_token_account.key, usdc_bank, 1901)
        .await?;

    usdc_depositor
        .try_bank_deposit(usdc_token_account.key, usdc_bank, 100)
        .await?;

    // Borrowing 10 SOL should succeed now
    let res = usdc_depositor
        .try_bank_borrow(depositor_sol_account.key, sol_bank, 10)
        .await;

    assert!(res.is_ok());

    Ok(())
}
