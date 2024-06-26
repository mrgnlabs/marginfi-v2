use std::{fs::File, io::Read, path::PathBuf, str::FromStr};

use anchor_lang::{prelude::Clock, AccountDeserialize, InstructionData, ToAccountMetas};
use anchor_spl::token_2022::spl_token_2022::extension::{
    transfer_fee::{TransferFeeConfig, MAX_FEE_BASIS_POINTS},
    BaseStateWithExtensions,
};
use anyhow::bail;
use base64::{engine::general_purpose::STANDARD, Engine};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{assert_custom_error, assert_eq_noise, native, prelude::*};
use marginfi::{
    constants::TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE,
    prelude::{GroupConfig, MarginfiError, MarginfiGroup},
    state::marginfi_group::{
        Bank, BankConfig, BankConfigOpt, BankOperationalState, BankVaultType, InterestRateConfig,
    },
};
use pretty_assertions::assert_eq;
use solana_account_decoder::UiAccountData;
use solana_cli_output::CliAccount;
use solana_program::{instruction::Instruction, pubkey, system_program};
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
                mint: BankMint::Usdc,
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
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

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
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
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
                mint: BankMint::Usdc,
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
                mint: BankMint::Sol,
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

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

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
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
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
                mint: BankMint::Usdc,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

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
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
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
                mint: BankMint::Usdc,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

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
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 10_000)
        .await?;

    let mut borrower_mfi_account = borrower_mfi_account_f.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
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
                mint: BankMint::Usdc,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::Sol,
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
            test_f.get_bank(&BankMint::Usdc),
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
            test_f.get_bank(&BankMint::Sol),
            1_001,
        )
        .await?;

    let borrower_borrow_account = test_f.usdc_mint.create_empty_token_account().await;

    borrower_account
        .try_bank_borrow(
            borrower_borrow_account.key,
            test_f.get_bank(&BankMint::Usdc),
            10_000,
        )
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    borrower_account.set_account(&borrower_mfi_account).await?;

    {
        let (insurance_vault, _) = test_f
            .get_bank(&BankMint::Usdc)
            .get_vault(BankVaultType::Insurance);
        test_f
            .get_bank_mut(&BankMint::Usdc)
            .mint
            .mint_to(&insurance_vault, 10_000)
            .await;
    }

    test_f
        .marginfi_group
        .try_handle_bankruptcy(test_f.get_bank(&BankMint::Usdc), &borrower_account)
        .await?;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let usdc_bank = test_f.get_bank(&BankMint::Usdc).load().await;

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
        .get_bank(&BankMint::Usdc)
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq!(insurance_amount.balance().await, 0);

    // Test account is disabled

    // Deposit 1 SOL
    let res = borrower_account
        .try_bank_deposit(
            borrower_deposit_account.key,
            test_f.get_bank(&BankMint::Sol),
            1,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Withdraw 1 SOL
    let res = borrower_account
        .try_bank_withdraw(
            borrower_deposit_account.key,
            test_f.get_bank(&BankMint::Sol),
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
            test_f.get_bank(&BankMint::Usdc),
            1,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Repay 1 USDC
    let res = borrower_account
        .try_bank_repay(
            borrower_borrow_account.key,
            test_f.get_bank(&BankMint::Usdc),
            1,
            None,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_fully_insured_t22_with_fee() -> anyhow::Result<()>
{
    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let collateral_mint = BankMint::Sol;
    let debt_mint = BankMint::T22WithFee;

    let user_balance = 100_000.0 * (1f64 + MAX_FEE_BASIS_POINTS as f64 / 10_000f64);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_debt = test_f
        .get_bank_mut(&debt_mint)
        .mint
        .create_token_account_and_mint_to(user_balance)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(
            lender_token_account_debt.key,
            test_f.get_bank(&debt_mint),
            100_000,
        )
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_collateral_account = test_f
        .get_bank_mut(&collateral_mint)
        .mint
        .create_token_account_and_mint_to(1_001)
        .await;

    borrower_mfi_account_f
        .try_bank_deposit(
            borrower_collateral_account.key,
            test_f.get_bank_mut(&collateral_mint),
            1_001,
        )
        .await?;

    let borrower_debt_account = test_f
        .get_bank_mut(&debt_mint)
        .mint
        .create_empty_token_account()
        .await;

    let borrow_amount = 10_000;

    borrower_mfi_account_f
        .try_bank_borrow(
            borrower_debt_account.key,
            test_f.get_bank_mut(&debt_mint),
            borrow_amount as f64,
        )
        .await?;
    assert_eq!(
        borrower_debt_account.balance().await,
        native!(borrow_amount, "USDC")
    );

    let mut borrower_mfi_account = borrower_mfi_account_f.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    borrower_mfi_account_f
        .set_account(&borrower_mfi_account)
        .await?;

    {
        let (insurance_vault, _) = test_f
            .get_bank(&debt_mint)
            .get_vault(BankVaultType::Insurance);
        let max_amount_to_cover_bad_debt =
            borrow_amount as f64 * (1f64 + MAX_FEE_BASIS_POINTS as f64 / 10_000f64);
        test_f
            .get_bank_mut(&debt_mint)
            .mint
            .mint_to(&insurance_vault, max_amount_to_cover_bad_debt)
            .await;
    }

    let debt_bank = test_f.get_bank(&debt_mint);

    let (pre_liquidity_vault_balance, pre_insurance_vault_balance) = (
        debt_bank
            .get_vault_token_account(BankVaultType::Liquidity)
            .await
            .balance()
            .await,
        debt_bank
            .get_vault_token_account(BankVaultType::Insurance)
            .await
            .balance()
            .await,
    );

    test_f
        .marginfi_group
        .try_handle_bankruptcy(test_f.get_bank(&debt_mint), &borrower_mfi_account_f)
        .await?;

    let (post_liquidity_vault_balance, post_insurance_vault_balance) = (
        debt_bank
            .get_vault_token_account(BankVaultType::Liquidity)
            .await
            .balance()
            .await,
        debt_bank
            .get_vault_token_account(BankVaultType::Insurance)
            .await
            .balance()
            .await,
    );

    let borrower_mfi_account = borrower_mfi_account_f.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let debt_bank = test_f.get_bank(&debt_mint).load().await;

    let lender_collateral_value = debt_bank.get_asset_amount(
        lender_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    // Check that no loss was socialized
    assert_eq_noise!(
        lender_collateral_value,
        I80F48::from(native!(
            100_000,
            test_f.get_bank(&debt_mint).mint.mint.decimals
        )),
        I80F48::ONE
    );

    let debt_bank_mint_state = test_f.get_bank(&debt_mint).mint.load_state().await;
    // borrow + fee(borrow)
    let expected_liquidity_vault_delta = native!(
        borrow_amount,
        test_f.get_bank(&debt_mint).mint.mint.decimals
    ) + debt_bank_mint_state
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_inverse_epoch_fee(
                0,
                native!(
                    borrow_amount,
                    test_f.get_bank(&debt_mint).mint.mint.decimals
                ),
            )
            .unwrap_or(0)
        })
        .unwrap_or(0);
    let expected_liquidity_vault_delta = I80F48::from(expected_liquidity_vault_delta);
    let actual_liquidity_vault_delta = post_liquidity_vault_balance - pre_liquidity_vault_balance;
    let actual_insurance_vault_delta = pre_insurance_vault_balance - post_insurance_vault_balance;

    assert_eq!(expected_liquidity_vault_delta, actual_liquidity_vault_delta);
    assert!(actual_insurance_vault_delta > expected_liquidity_vault_delta);

    // Test account is disabled

    // Deposit 1 SOL
    let res = borrower_mfi_account_f
        .try_bank_deposit(
            borrower_collateral_account.key,
            test_f.get_bank(&collateral_mint),
            1,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Withdraw 1 SOL
    let res = borrower_mfi_account_f
        .try_bank_withdraw(
            borrower_collateral_account.key,
            test_f.get_bank(&collateral_mint),
            1,
            None,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Borrow 1 USDC
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_debt_account.key, test_f.get_bank(&debt_mint), 1)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::AccountDisabled);

    // Repay 1 USDC
    let res = borrower_mfi_account_f
        .try_bank_repay(
            borrower_debt_account.key,
            test_f.get_bank(&debt_mint),
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
                mint: BankMint::Usdc,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::Sol,
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
            test_f.get_bank(&BankMint::Usdc),
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
            test_f.get_bank(&BankMint::Sol),
            1_001,
        )
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_account
        .try_bank_borrow(
            borrower_token_account_usdc.key,
            test_f.get_bank(&BankMint::Usdc),
            10_000,
        )
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    borrower_account.set_account(&borrower_mfi_account).await?;

    test_f
        .usdc_mint
        .mint_to(
            &test_f
                .get_bank(&BankMint::Usdc)
                .load()
                .await
                .insurance_vault,
            5_000,
        )
        .await;

    test_f
        .marginfi_group
        .try_handle_bankruptcy(test_f.get_bank(&BankMint::Usdc), &borrower_account)
        .await?;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_usdc_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_usdc_balance.liability_shares),
        I80F48::ZERO
    );

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let usdc_bank = test_f.get_bank(&BankMint::Usdc).load().await;

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
        .get_bank(&BankMint::Usdc)
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq!(insurance_amount.balance().await, 0);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_partially_insured_t22_with_fee(
) -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let collateral_mint = BankMint::Sol;
    let debt_mint = BankMint::T22WithFee;

    let pre_lender_collateral_amount = 100_000.0;
    let user_balance =
        pre_lender_collateral_amount * (1f64 + MAX_FEE_BASIS_POINTS as f64 / 10_000f64);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_collateral_token_account = test_f
        .get_bank(&debt_mint)
        .mint
        .create_token_account_and_mint_to(user_balance)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(
            lender_collateral_token_account.key,
            test_f.get_bank(&debt_mint),
            pre_lender_collateral_amount,
        )
        .await?;

    let borrower_account = test_f.create_marginfi_account().await;
    let borrower_collateral_token_account = test_f
        .get_bank(&collateral_mint)
        .mint
        .create_token_account_and_mint_to(1_001)
        .await;
    borrower_account
        .try_bank_deposit(
            borrower_collateral_token_account.key,
            test_f.get_bank(&collateral_mint),
            1_001,
        )
        .await?;

    let borrow_amount = 10_000.0;

    let borrower_debt_token_account = test_f
        .get_bank(&debt_mint)
        .mint
        .create_empty_token_account()
        .await;
    borrower_account
        .try_bank_borrow(
            borrower_debt_token_account.key,
            test_f.get_bank(&debt_mint),
            borrow_amount,
        )
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    borrower_account.set_account(&borrower_mfi_account).await?;

    let initial_insurance_vault_balance = 5_000.0;
    let insurance_vault = test_f.get_bank(&debt_mint).load().await.insurance_vault;
    test_f
        .get_bank_mut(&debt_mint)
        .mint
        .mint_to(&insurance_vault, initial_insurance_vault_balance)
        .await;

    let debt_bank_f = test_f.get_bank(&debt_mint);

    let pre_borrower_debt_balance = borrower_account.load().await.lending_account.balances[1];

    let pre_liquidity_vault_balance = debt_bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;

    test_f
        .marginfi_group
        .try_handle_bankruptcy(test_f.get_bank(&debt_mint), &borrower_account)
        .await?;

    let post_liquidity_vault_balance = debt_bank_f
        .get_vault_token_account(BankVaultType::Liquidity)
        .await
        .balance()
        .await;

    let borrower_mfi_account = borrower_account.load().await;
    let borrower_debt_balance = borrower_mfi_account.lending_account.balances[1];

    assert_eq!(
        I80F48::from(borrower_debt_balance.liability_shares),
        I80F48::ZERO
    );

    let debt_bank = test_f.get_bank(&debt_mint).load().await;
    let pre_borrower_debt_amount =
        debt_bank.get_liability_amount(pre_borrower_debt_balance.liability_shares.into())?;

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let debt_bank = test_f.get_bank(&debt_mint).load().await;

    let post_lender_collateral_amount = debt_bank.get_asset_amount(
        lender_mfi_account.lending_account.balances[0]
            .asset_shares
            .into(),
    )?;

    let debt_bank_mint_state = test_f.get_bank(&debt_mint).mint.load_state().await;

    let available_insurance_amount = native!(
        initial_insurance_vault_balance,
        test_f.get_bank(&debt_mint).mint.mint.decimals,
        f64
    );
    let if_fee_amount = debt_bank_mint_state
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_epoch_fee(0, available_insurance_amount)
                .unwrap_or(0)
        })
        .unwrap_or(0);

    let available_insurance_amount = available_insurance_amount - if_fee_amount;

    let amount_not_covered = pre_borrower_debt_amount - I80F48::from(available_insurance_amount);

    let pre_lender_collateral_amount_native = I80F48::from(native!(
        pre_lender_collateral_amount,
        debt_bank.mint_decimals,
        f64
    ));

    let expected_post_lender_collateral_amount =
        pre_lender_collateral_amount_native - amount_not_covered;
    assert_eq_noise!(
        expected_post_lender_collateral_amount,
        post_lender_collateral_amount,
        I80F48::ONE
    );

    let insurance_amount = test_f
        .get_bank(&debt_mint)
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq!(insurance_amount.balance().await, 0);

    let actual_liquidity_vault_delta = post_liquidity_vault_balance - pre_liquidity_vault_balance;
    assert_eq!(available_insurance_amount, actual_liquidity_vault_delta);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_success_not_insured() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

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
    let borrower_borrow_account = test_f.usdc_mint.create_empty_token_account().await;
    borrower_account
        .try_bank_borrow(borrower_borrow_account.key, usdc_bank_f, 10_000)
        .await?;

    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();

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
                mint: BankMint::Usdc,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

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
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 10_000)
        .await?;

    let mut borrower_mfi_account = borrower_mfi_account_f.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();

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
            mint: BankMint::Usdc,
            config: None,
        }],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);

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
            mint: BankMint::Usdc,
            config: None,
        }],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000)
        .await?;

    usdc_bank_f
        .update_config(BankConfigOpt {
            operational_state: Some(BankOperationalState::ReduceOnly),
            ..Default::default()
        })
        .await?;

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
                mint: BankMint::Usdc,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

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

    let lender_2_token_account_sol = test_f.sol_mint.create_empty_token_account().await;
    lender_2_mfi_account
        .try_bank_borrow(lender_2_token_account_sol.key, sol_bank_f, 1)
        .await?;

    usdc_bank_f
        .update_config(BankConfigOpt {
            operational_state: Some(BankOperationalState::ReduceOnly),
            ..Default::default()
        })
        .await?;

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
                mint: BankMint::Usdc,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

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

    sol_bank_f
        .update_config(BankConfigOpt {
            operational_state: Some(BankOperationalState::ReduceOnly),
            ..Default::default()
        })
        .await?;

    let borrower_token_account_sol = test_f.sol_mint.create_empty_token_account().await;
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
            mint: BankMint::Usdc,
            config: None,
        }],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);

    usdc_bank_f
        .update_config(BankConfigOpt {
            operational_state: Some(BankOperationalState::ReduceOnly),
            ..Default::default()
        })
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
    assert_custom_error!(res.unwrap_err(), MarginfiError::BankReduceOnly);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_init_limit_0() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let sol_bank = test_f.get_bank(&BankMint::Sol);

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
    let depositor_sol_account = sol_bank.mint.create_empty_token_account().await;
    let res = usdc_depositor
        .try_bank_borrow(depositor_sol_account.key, sol_bank, 9.9)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::RiskEngineInitRejected);

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

#[tokio::test]
async fn bank_field_values_reg() -> anyhow::Result<()> {
    let bank_fixtures_path = "tests/fixtures/bank";

    // Sample 1 (Jito)

    let mut path = PathBuf::from_str(bank_fixtures_path).unwrap();
    path.push("bank_sample_1.json");
    let mut file = File::open(&path).unwrap();
    let mut account_info_raw = String::new();
    file.read_to_string(&mut account_info_raw).unwrap();

    let account: CliAccount = serde_json::from_str(&account_info_raw).unwrap();
    let UiAccountData::Binary(data, _) = account.keyed_account.account.data else {
        bail!("Expecting Binary format for fixtures")
    };
    let bank = Bank::try_deserialize(&mut STANDARD.decode(data)?.as_slice())?;

    assert_eq!(
        bank.mint,
        pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn")
    );
    assert_eq!(bank.mint_decimals, 9);
    assert_eq!(
        bank.group,
        pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8")
    );
    assert_eq!(
        I80F48::from(bank.asset_share_value),
        I80F48::from_str("1.000561264812955").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.liability_share_value),
        I80F48::from_str("1.00737674726716").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_insurance_fees_outstanding),
        I80F48::from_str("61174.580321107215052").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_group_fees_outstanding),
        I80F48::from_str("35660072279.35465946938668").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_liability_shares),
        I80F48::from_str("79763493059362.858709822356737").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_asset_shares),
        I80F48::from_str("998366336320727.44918120920092").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_init),
        I80F48::from_str("0.649999976158142").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_maint),
        I80F48::from_str("0.80000001192093").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_init),
        I80F48::from_str("1.3").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_maint),
        I80F48::from_str("1.2").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.optimal_utilization_rate),
        I80F48::from_str("0.8").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.plateau_interest_rate),
        I80F48::from_str("0.1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.max_interest_rate),
        I80F48::from_str("3").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_fee_fixed_apr),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_ir_fee),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_fixed_fee_apr),
        I80F48::from_str("0.01").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_ir_fee),
        I80F48::from_str("0.05").unwrap()
    );

    // Sample 2 (META)

    let mut path = PathBuf::from_str(bank_fixtures_path).unwrap();
    path.push("bank_sample_2.json");
    let mut file = File::open(&path).unwrap();
    let mut account_info_raw = String::new();
    file.read_to_string(&mut account_info_raw).unwrap();

    let account: CliAccount = serde_json::from_str(&account_info_raw).unwrap();
    let UiAccountData::Binary(data, _) = account.keyed_account.account.data else {
        bail!("Expecting Binary format for fixtures")
    };
    let bank = Bank::try_deserialize(&mut STANDARD.decode(data)?.as_slice())?;

    assert_eq!(
        bank.mint,
        pubkey!("METADDFL6wWMWEoKTFJwcThTbUmtarRJZjRpzUvkxhr")
    );
    assert_eq!(bank.mint_decimals, 9);
    assert_eq!(
        bank.group,
        pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8")
    );
    assert_eq!(
        I80F48::from(bank.asset_share_value),
        I80F48::from_str("1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.liability_share_value),
        I80F48::from_str("1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_insurance_fees_outstanding),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_group_fees_outstanding),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_liability_shares),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_asset_shares),
        I80F48::from_str("698503862367").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_init),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_maint),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_init),
        I80F48::from_str("2.5").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_maint),
        I80F48::from_str("1.5").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.optimal_utilization_rate),
        I80F48::from_str("0.8").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.plateau_interest_rate),
        I80F48::from_str("0.1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.max_interest_rate),
        I80F48::from_str("3").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_fee_fixed_apr),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_ir_fee),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_fixed_fee_apr),
        I80F48::from_str("0.01").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_ir_fee),
        I80F48::from_str("0.05").unwrap()
    );

    // Sample 3 (USDT)

    let mut path = PathBuf::from_str(bank_fixtures_path).unwrap();
    path.push("bank_sample_3.json");
    let mut file = File::open(&path).unwrap();
    let mut account_info_raw = String::new();
    file.read_to_string(&mut account_info_raw).unwrap();

    let account: CliAccount = serde_json::from_str(&account_info_raw).unwrap();
    let UiAccountData::Binary(data, _) = account.keyed_account.account.data else {
        bail!("Expecting Binary format for fixtures")
    };
    let bank = Bank::try_deserialize(&mut STANDARD.decode(data)?.as_slice())?;

    assert_eq!(
        bank.mint,
        pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB")
    );
    assert_eq!(bank.mint_decimals, 6);
    assert_eq!(
        bank.group,
        pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8")
    );
    assert_eq!(
        I80F48::from(bank.asset_share_value),
        I80F48::from_str("1.063003765188338").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.liability_share_value),
        I80F48::from_str("1.12089611736063").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_insurance_fees_outstanding),
        I80F48::from_str("45839.746526861401865").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_group_fees_outstanding),
        I80F48::from_str("28634360131.219557095675654").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_liability_shares),
        I80F48::from_str("32109684419718.204607882232235").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_asset_shares),
        I80F48::from_str("43231381120800.339303417329994").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_init),
        I80F48::from_str("1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_maint),
        I80F48::from_str("1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_init),
        I80F48::from_str("1.25").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_maint),
        I80F48::from_str("1.1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.optimal_utilization_rate),
        I80F48::from_str("0.8").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.plateau_interest_rate),
        I80F48::from_str("0.2").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.max_interest_rate),
        I80F48::from_str("4").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_fee_fixed_apr),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_ir_fee),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_fixed_fee_apr),
        I80F48::from_str("0.01").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_ir_fee),
        I80F48::from_str("0.05").unwrap()
    );

    Ok(())
}