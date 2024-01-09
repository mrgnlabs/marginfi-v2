use anchor_lang::prelude::{AnchorError, Clock};
use anchor_lang::{InstructionData, ToAccountMetas};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::prelude::*;
use fixtures::{assert_custom_error, assert_eq_noise, native};
use marginfi::constants::{
    EMISSIONS_FLAG_BORROW_ACTIVE, EMISSIONS_FLAG_LENDING_ACTIVE, MIN_EMISSIONS_START_TIME,
};
use marginfi::state::marginfi_account::{
    BankAccountWrapper, DISABLED_FLAG, FLASHLOAN_ENABLED_FLAG, IN_FLASHLOAN_FLAG,
};
use marginfi::state::{
    marginfi_account::MarginfiAccount,
    marginfi_group::{Bank, BankConfig, BankConfigOpt, BankVaultType},
};
use marginfi::{assert_eq_with_tolerance, prelude::*};
use pretty_assertions::assert_eq;

use solana_program::pubkey::Pubkey;
use solana_program::{instruction::Instruction, system_program};
use solana_program_test::*;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::timing::SECONDS_PER_YEAR;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

// Feature baseline

#[tokio::test]
async fn marginfi_account_create_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    // Create & initialize marginfi account
    let marginfi_account_key = Keypair::new();
    let accounts = marginfi::accounts::MarginfiAccountInitialize {
        marginfi_group: test_f.marginfi_group.key,
        marginfi_account: marginfi_account_key.pubkey(),
        authority: test_f.payer(),
        fee_payer: test_f.payer(),
        system_program: system_program::id(),
    };
    let init_marginfi_account_ix = Instruction {
        program_id: marginfi::id(),
        accounts: accounts.to_account_metas(Some(true)),
        data: marginfi::instruction::MarginfiAccountInitialize {}.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[init_marginfi_account_ix],
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
    assert_eq!(marginfi_account.authority, test_f.payer());
    assert!(marginfi_account
        .lending_account
        .balances
        .iter()
        .all(|bank| !bank.active));

    Ok(())
}

#[tokio::test]
async fn marginfi_account_deposit_success() -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::USDC,
            ..TestBankSetting::default()
        }],
        ..TestSettings::default()
    }))
    .await;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.payer();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &owner).await;
    test_f.usdc_mint.mint_to(&token_account_f.key, 1_000).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    let res = marginfi_account_f
        .try_bank_deposit(token_account_f.key, usdc_bank_f, 1_000)
        .await;
    assert!(res.is_ok());

    let marginfi_account = marginfi_account_f.load().await;

    // Check balance is active
    assert!(marginfi_account
        .lending_account
        .get_balance(&usdc_bank_f.key)
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
async fn marginfi_account_deposit_failure_capacity_exceeded() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::USDC,
            config: Some(BankConfig {
                deposit_limit: native!(100, "USDC"),
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            }),
        }],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);

    // Fund user account
    let user_mfi_account_f = test_f.create_marginfi_account().await;
    let user_token_account = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;

    // Make unlawful deposit
    let res = user_mfi_account_f
        .try_bank_deposit(user_token_account.key, usdc_bank, 100)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::BankAssetCapacityExceeded);

    // Make lawful deposit
    let res = user_mfi_account_f
        .try_bank_deposit(user_token_account.key, usdc_bank, 99)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_account_withdraw_success() -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::USDC,
            ..TestBankSetting::default()
        }],
        ..TestSettings::default()
    }))
    .await;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.payer();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &owner).await;
    test_f.usdc_mint.mint_to(&token_account_f.key, 1_000).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    marginfi_account_f
        .try_bank_deposit(token_account_f.key, usdc_bank_f, 1_000)
        .await
        .unwrap();

    let res = marginfi_account_f
        .try_bank_withdraw(token_account_f.key, usdc_bank_f, 900, None)
        .await;

    assert!(res.is_ok());

    let marginfi_account = marginfi_account_f.load().await;

    // Check balance is correct
    let usdc_bank = usdc_bank_f.load().await;

    let usdc_balance = usdc_bank
        .get_asset_amount(
            marginfi_account
                .lending_account
                .get_balance(&usdc_bank_f.key)
                .unwrap()
                .asset_shares
                .into(),
        )
        .unwrap();

    assert_eq!(usdc_balance, I80F48::from(native!(100, "USDC")));

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
async fn marginfi_account_withdraw_failure_withdrawing_too_much() -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::USDC,
            ..TestBankSetting::default()
        }],
        ..TestSettings::default()
    }))
    .await;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.payer();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &owner).await;
    test_f.usdc_mint.mint_to(&token_account_f.key, 1_000).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    marginfi_account_f
        .try_bank_deposit(token_account_f.key, usdc_bank_f, 1_000)
        .await
        .unwrap();

    let res = marginfi_account_f
        .try_bank_withdraw(token_account_f.key, usdc_bank_f, 1_001, None)
        .await;

    assert!(res.is_err());

    assert_custom_error!(res.unwrap_err(), MarginfiError::OperationWithdrawOnly);

    Ok(())
}

#[tokio::test]
async fn marginfi_account_withdraw_failure_borrow_limit() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    usdc_bank
        .update_config(BankConfigOpt {
            borrow_limit: Some(native!(1000, "USDC")),
            deposit_limit: Some(native!(10001, "USDC")),
            ..Default::default()
        })
        .await?;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let _owner = test_f.payer();
    let depositor_usdc_account = usdc_bank
        .mint
        .create_token_account_and_mint_to(10_000)
        .await;

    marginfi_account_f
        .try_bank_deposit(depositor_usdc_account.key, usdc_bank, 10_000)
        .await
        .unwrap();

    let borrower = test_f.create_marginfi_account().await;

    let borrower_sol_account = sol_bank
        .mint
        .create_token_account_and_mint_to(100_000)
        .await;

    let borrower_usdc_account = usdc_bank.mint.create_token_account_and_mint_to(0).await;

    borrower
        .try_bank_deposit(borrower_sol_account.key, sol_bank, 1000)
        .await?;

    let res = borrower
        .try_bank_borrow(borrower_usdc_account.key, usdc_bank, 1001)
        .await;

    assert!(res.is_err());
    assert_custom_error!(
        res.unwrap_err(),
        MarginfiError::BankLiabilityCapacityExceeded
    );

    let res = borrower
        .try_bank_borrow(borrower_usdc_account.key, usdc_bank, 999)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_account_withdraw_all_success() -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::USDC,
            ..TestBankSetting::default()
        }],
        ..TestSettings::default()
    }))
    .await;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.payer();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &owner).await;
    test_f.usdc_mint.mint_to(&token_account_f.key, 1_000).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    marginfi_account_f
        .try_bank_deposit(token_account_f.key, usdc_bank_f, 1_000)
        .await
        .unwrap();

    let res = marginfi_account_f
        .try_bank_withdraw(token_account_f.key, usdc_bank_f, 0, Some(true))
        .await;

    assert!(res.is_ok());

    let marginfi_account = marginfi_account_f.load().await;

    // Check balance is correct
    assert!(marginfi_account
        .lending_account
        .get_balance(&usdc_bank_f.key)
        .is_none());

    assert_eq!(
        marginfi_account
            .lending_account
            .get_active_balances_iter()
            .collect::<Vec<_>>()
            .len(),
        0
    );

    Ok(())
}

#[tokio::test]
async fn marginfi_account_repay_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 99)
        .await
        .unwrap();

    let res = borrower_mfi_account_f
        .try_bank_repay(borrower_token_account_f_sol.key, sol_bank, 80, None)
        .await;

    assert!(res.is_ok());

    // Check token balances are correct
    assert_eq!(
        borrower_token_account_f_usdc.balance().await,
        native!(0, "USDC")
    );

    assert_eq!(
        borrower_token_account_f_sol.balance().await,
        native!(19, "SOL")
    );

    // TODO: check health is sane

    Ok(())
}

#[tokio::test]
async fn marginfi_account_repay_failure_repaying_too_much() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 99)
        .await
        .unwrap();

    let res = borrower_mfi_account_f
        .try_bank_repay(borrower_token_account_f_sol.key, sol_bank, 110, None)
        .await;

    assert!(res.is_err());

    assert_custom_error!(res.unwrap_err(), MarginfiError::OperationRepayOnly);

    Ok(())
}

#[tokio::test]
async fn marginfi_account_repay_all_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank_f, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank_f, 99)
        .await
        .unwrap();

    let res = borrower_mfi_account_f
        .try_bank_repay(borrower_token_account_f_sol.key, sol_bank_f, 0, Some(true))
        .await;

    assert!(res.is_ok());

    let marginfi_account = borrower_mfi_account_f.load().await;

    assert!(marginfi_account
        .lending_account
        .get_balance(&sol_bank_f.key)
        .is_none());

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
async fn marginfi_account_borrow_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 99)
        .await;

    assert!(res.is_ok());

    // Check token balances are correct
    assert_eq!(
        borrower_token_account_f_usdc.balance().await,
        native!(0, "USDC")
    );

    assert_eq!(
        borrower_token_account_f_sol.balance().await,
        native!(99, "SOL")
    );

    // TODO: check health is sane

    Ok(())
}

#[tokio::test]
async fn marginfi_account_borrow_success_swb() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_swb_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 99)
        .await;

    assert!(res.is_ok());

    // Check token balances are correct
    assert_eq!(
        borrower_token_account_f_usdc.balance().await,
        native!(0, "USDC")
    );

    assert_eq!(
        borrower_token_account_f_sol.balance().await,
        native!(99, "SOL")
    );

    // TODO: check health is sane

    Ok(())
}

#[tokio::test]
async fn marginfi_account_borrow_failure_not_enough_collateral() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 101)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::BadAccountHealth);

    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 100)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                ..TestBankSetting::default()
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    asset_weight_maint: I80F48!(1).into(),
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
        .create_token_account_and_mint_to(2_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 2_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;

    // Borrower deposits 100 SOL worth of $1000
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 100)
        .await?;

    // Borrower borrows $999
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 999)
        .await?;

    // Synthetically bring down the borrower account health by reducing the asset weights of the SOL bank
    sol_bank_f
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.25).into()),
            asset_weight_maint: Some(I80F48!(0.5).into()),
            ..Default::default()
        })
        .await?;

    lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await?;

    // Checks
    let sol_bank: Bank = sol_bank_f.load().await;
    let usdc_bank: Bank = usdc_bank_f.load().await;

    let depositor_ma = lender_mfi_account_f.load().await;
    let borrower_ma = borrower_mfi_account_f.load().await;

    // Depositors should have 1 SOL
    assert_eq!(
        sol_bank
            .get_asset_amount(depositor_ma.lending_account.balances[1].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(1, "SOL"))
    );

    // Depositors should have 1990.25 USDC
    assert_eq_noise!(
        usdc_bank
            .get_asset_amount(depositor_ma.lending_account.balances[0].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(1990.25, "USDC", f64)),
        native!(0.00001, "USDC", f64)
    );

    // Borrower should have 99 SOL
    assert_eq!(
        sol_bank
            .get_asset_amount(borrower_ma.lending_account.balances[0].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(99, "SOL"))
    );

    // Borrower should have 989.50 USDC
    assert_eq_noise!(
        usdc_bank
            .get_liability_amount(
                borrower_ma.lending_account.balances[1]
                    .liability_shares
                    .into()
            )
            .unwrap(),
        I80F48::from(native!(989.50, "USDC", f64)),
        native!(0.00001, "USDC", f64)
    );

    // Check insurance fund fee
    let insurance_fund_usdc = usdc_bank_f
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq_noise!(
        insurance_fund_usdc.balance().await as i64,
        native!(0.25, "USDC", f64) as i64,
        1
    );

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_success_many_balances() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::many_banks_10())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);
    let sol_eq_bank_f = test_f.get_bank(&BankMint::SolEquivalent);
    let sol_eq1_bank_f = test_f.get_bank(&BankMint::SolEquivalent1);
    let sol_eq2_bank_f = test_f.get_bank(&BankMint::SolEquivalent2);
    let sol_eq3_bank_f = test_f.get_bank(&BankMint::SolEquivalent3);
    let sol_eq4_bank_f = test_f.get_bank(&BankMint::SolEquivalent4);
    let sol_eq5_bank_f = test_f.get_bank(&BankMint::SolEquivalent5);
    let sol_eq6_bank_f = test_f.get_bank(&BankMint::SolEquivalent6);
    let sol_eq7_bank_f = test_f.get_bank(&BankMint::SolEquivalent7);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(2_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 2_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    let borrower_token_account_sol_eq = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(5000)
        .await;

    // Borrower deposits 100 SOL worth of $1000
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 100)
        .await?;

    // Borrower borrows $999
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 999)
        .await?;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq1_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq2_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq3_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq4_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq5_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq6_bank_f, 0)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq7_bank_f, 0)
        .await?;

    // Synthetically bring down the borrower account health by reducing the asset weights of the SOL bank
    sol_bank_f
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.25).into()),
            asset_weight_maint: Some(I80F48!(0.5).into()),
            ..Default::default()
        })
        .await?;

    lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await?;

    // Checks
    let sol_bank: Bank = sol_bank_f.load().await;
    let usdc_bank: Bank = usdc_bank_f.load().await;

    let depositor_ma = lender_mfi_account_f.load().await;
    let borrower_ma = borrower_mfi_account_f.load().await;

    // Depositors should have 1 SOL
    assert_eq!(
        sol_bank
            .get_asset_amount(depositor_ma.lending_account.balances[1].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(1, "SOL"))
    );

    // Depositors should have 1990.25 USDC
    assert_eq_noise!(
        usdc_bank
            .get_asset_amount(depositor_ma.lending_account.balances[0].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(1990.25, "USDC", f64)),
        native!(0.00001, "USDC", f64)
    );

    // Borrower should have 99 SOL
    assert_eq!(
        sol_bank
            .get_asset_amount(borrower_ma.lending_account.balances[0].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(99, "SOL"))
    );

    // Borrower should have 989.50 USDC
    assert_eq_noise!(
        usdc_bank
            .get_liability_amount(
                borrower_ma.lending_account.balances[1]
                    .liability_shares
                    .into()
            )
            .unwrap(),
        I80F48::from(native!(989.50, "USDC", f64)),
        native!(0.00001, "USDC", f64)
    );

    // Check insurance fund fee
    let insurance_fund_usdc = usdc_bank_f
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq_noise!(
        insurance_fund_usdc.balance().await as i64,
        native!(0.25, "USDC", f64) as i64,
        1
    );

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_success_swb() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                ..TestBankSetting::default()
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    asset_weight_maint: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_SW_BANK_CONFIG
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
        .create_token_account_and_mint_to(2_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 2_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;

    // Borrower deposits 100 SOL worth of $1000
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 100)
        .await?;

    // Borrower borrows $999
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 999)
        .await?;

    // Synthetically bring down the borrower account health by reducing the asset weights of the SOL bank
    sol_bank_f
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.25).into()),
            asset_weight_maint: Some(I80F48!(0.5).into()),
            ..Default::default()
        })
        .await?;

    lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await?;

    // Checks
    let sol_bank: Bank = sol_bank_f.load().await;
    let usdc_bank: Bank = usdc_bank_f.load().await;

    let depositor_ma = lender_mfi_account_f.load().await;
    let borrower_ma = borrower_mfi_account_f.load().await;

    // Depositors should have 1 SOL
    assert_eq!(
        sol_bank
            .get_asset_amount(depositor_ma.lending_account.balances[1].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(1, "SOL"))
    );

    // Depositors should have 1990.25 USDC
    assert_eq_noise!(
        usdc_bank
            .get_asset_amount(depositor_ma.lending_account.balances[0].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(1990.25, "USDC", f64)),
        native!(0.01, "USDC", f64)
    );

    // Borrower should have 99 SOL
    assert_eq!(
        sol_bank
            .get_asset_amount(borrower_ma.lending_account.balances[0].asset_shares.into())
            .unwrap(),
        I80F48::from(native!(99, "SOL"))
    );

    // Borrower should have 989.50 USDC
    assert_eq_noise!(
        usdc_bank
            .get_liability_amount(
                borrower_ma.lending_account.balances[1]
                    .liability_shares
                    .into()
            )
            .unwrap(),
        I80F48::from(native!(989.50, "USDC", f64)),
        native!(0.01, "USDC", f64)
    );

    // Check insurance fund fee
    let insurance_fund_usdc = usdc_bank_f
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq_noise!(
        insurance_fund_usdc.balance().await as i64,
        native!(0.25, "USDC", f64) as i64,
        native!(0.001, "USDC", f64) as i64
    );

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_failure_liquidatee_not_unhealthy() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: Some(BankConfig {
                    asset_weight_maint: I80F48!(1).into(),
                    ..*DEFAULT_USDC_TEST_BANK_CONFIG
                }),
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    asset_weight_maint: I80F48!(1).into(),
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
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 200)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;

    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 100)
        .await?;

    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 100)
        .await?;

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await;

    assert!(res.is_err());

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_failure_liquidation_too_severe() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 200)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(10).await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 10)
        .await?;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 61)
        .await?;

    sol_bank_f
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.25).into()),
            asset_weight_maint: Some(I80F48!(0.5).into()),
            ..Default::default()
        })
        .await?;

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 10, usdc_bank_f)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_failure_liquidator_no_collateral() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::USDC,
                config: Some(BankConfig {
                    liability_weight_init: I80F48!(1.2).into(),
                    liability_weight_maint: I80F48!(1.1).into(),
                    ..*DEFAULT_USDC_TEST_BANK_CONFIG
                }),
            },
            TestBankSetting {
                mint: BankMint::SOL,
                config: None,
            },
            TestBankSetting {
                mint: BankMint::SolEquivalent,
                config: None,
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);
    let sol_eq_bank_f = test_f.get_bank(&BankMint::SolEquivalent);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 200)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_sol_eq = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(100)
        .await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 10)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq_bank_f, 1)
        .await?;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 60)
        .await?;

    sol_bank_f
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.25).into()),
            asset_weight_maint: Some(I80F48!(0.3).into()),
            ..Default::default()
        })
        .await?;

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_eq_bank_f, 2, usdc_bank_f)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_eq_bank_f, 1, usdc_bank_f)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn marginfi_account_liquidation_failure_bank_not_liquidatable() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);
    let sol_eq_bank_f = test_f.get_bank(&BankMint::SolEquivalent);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 200)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_sol_eq = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(100)
        .await;
    let borrower_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 10)
        .await?;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq_bank_f, 1)
        .await?;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 60)
        .await?;

    sol_bank_f
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.25).into()),
            asset_weight_maint: Some(I80F48!(0.4).into()),
            ..Default::default()
        })
        .await?;

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_eq_bank_f, 1, sol_bank_f)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    let res = lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_bank_f)
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn automatic_interest_payments() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    // Create lender user accounts and deposit SOL asset
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank_f, 1_000)
        .await?;

    // Create borrower user accounts and deposit USDC asset
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_usdc.key, usdc_bank_f, 1_000)
        .await?;

    // Borrow SOL from borrower mfi account
    borrower_mfi_account_f
        .try_bank_borrow(lender_token_account_sol.key, sol_bank_f, 99)
        .await?;

    // Let a year go by
    {
        let mut ctx = test_f.context.borrow_mut();
        let mut clock: Clock = ctx.banks_client.get_sysvar().await?;
        // Advance clock by 1 year
        clock.unix_timestamp += 365 * 24 * 60 * 60;
        ctx.set_sysvar(&clock);
    }

    // Repay principal, leaving only the accrued interest
    borrower_mfi_account_f
        .try_bank_repay(lender_token_account_sol.key, sol_bank_f, 99, None)
        .await?;

    let sol_bank = sol_bank_f.load().await;
    let borrower_mfi_account = borrower_mfi_account_f.load().await;
    let lender_mfi_account = lender_mfi_account_f.load().await;

    // Verify that interest accrued matches on both sides
    assert_eq_noise!(
        sol_bank
            .get_liability_amount(
                borrower_mfi_account.lending_account.balances[1]
                    .liability_shares
                    .into()
            )
            .unwrap(),
        I80F48::from(native!(11.761, "SOL", f64)),
        native!(0.0002, "SOL", f64)
    );

    assert_eq_noise!(
        sol_bank
            .get_asset_amount(
                lender_mfi_account.lending_account.balances[0]
                    .asset_shares
                    .into()
            )
            .unwrap(),
        I80F48::from(native!(1011.761, "SOL", f64)),
        native!(0.0002, "SOL", f64)
    );
    // TODO: check health is sane

    Ok(())
}

// Regression

#[tokio::test]
async fn marginfi_account_correct_balance_selection_after_closing_position() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank_f, 1_000)
        .await?;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(2_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 2_000)
        .await?;

    lender_mfi_account_f
        .try_bank_withdraw(lender_token_account_sol.key, sol_bank_f, 0, Some(true))
        .await
        .unwrap();

    let mut marginfi_account = lender_mfi_account_f.load().await;
    let mut usdc_bank = usdc_bank_f.load().await;

    let bank_account = BankAccountWrapper::find(
        &usdc_bank_f.key,
        &mut usdc_bank,
        &mut marginfi_account.lending_account,
    );

    assert!(bank_account.is_ok());

    let bank_account = bank_account.unwrap();

    assert_eq!(
        bank_account
            .bank
            .get_asset_amount(bank_account.balance.asset_shares.into())
            .unwrap()
            .to_num::<u64>(),
        native!(2_000, "USDC")
    );

    Ok(())
}

#[tokio::test]
async fn isolated_borrows() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_one_isolated())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_eq_bank = test_f.get_bank(&BankMint::SolEquivalent);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_eq_bank, 1_000)
        .await?;

    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(0)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL EQ
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_eq_bank, 10)
        .await;

    assert!(res.is_ok());

    // Repay isolated SOL EQ borrow and borrow SOL successfully,
    let borrower_sol_account = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_sol_account.key, sol_bank, 10)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::IsolatedAccountIllegalState);

    borrower_mfi_account_f
        .try_bank_repay(borrower_token_account_f_sol.key, sol_eq_bank, 0, Some(true))
        .await?;

    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_sol_account.key, sol_bank, 10)
        .await;

    assert!(res.is_ok());

    // Borrowing SOL EQ again fails
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_eq_bank, 10)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::IsolatedAccountIllegalState);

    Ok(())
}

#[tokio::test]
async fn emissions_test() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_one_isolated())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Setup emissions (Deposit for USDC, Borrow for SOL)

    let funding_account = test_f.usdc_mint.create_token_account_and_mint_to(100).await;

    usdc_bank
        .try_setup_emissions(
            EMISSIONS_FLAG_LENDING_ACTIVE,
            1_000_000,
            native!(50, "USDC"),
            usdc_bank.mint.key,
            funding_account.key,
        )
        .await?;

    // SOL Emissions are not in SOL Bank mint
    let sol_emissions_mint = MintFixture::new(test_f.context.clone(), None, Some(6)).await;

    let funding_account = sol_emissions_mint
        .create_token_account_and_mint_to(200)
        .await;

    sol_bank
        .try_setup_emissions(
            EMISSIONS_FLAG_BORROW_ACTIVE,
            1_000_000,
            native!(100, 6),
            sol_emissions_mint.key,
            funding_account.key,
        )
        .await?;

    let sol_emissions_mint_2 = MintFixture::new(test_f.context.clone(), None, Some(6)).await;

    let res = sol_bank
        .try_setup_emissions(
            EMISSIONS_FLAG_BORROW_ACTIVE,
            1_000_000,
            native!(50, 6),
            sol_emissions_mint_2.key,
            funding_account.key,
        )
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::EmissionsAlreadySetup);

    // Fund SOL bank
    let sol_lender_account = test_f.create_marginfi_account().await;
    let sol_lender_token_account = test_f.sol_mint.create_token_account_and_mint_to(100).await;

    sol_lender_account
        .try_bank_deposit(sol_lender_token_account.key, sol_bank, 100)
        .await?;

    // Create account and setup positions
    test_f.set_time(MIN_EMISSIONS_START_TIME as i64);
    test_f
        .set_pyth_oracle_timestamp(PYTH_USDC_FEED, MIN_EMISSIONS_START_TIME as i64)
        .await;
    test_f
        .set_pyth_oracle_timestamp(PYTH_SOL_FEED, MIN_EMISSIONS_START_TIME as i64)
        .await;

    let mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(50).await;

    mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank, 50)
        .await?;

    let sol_account = test_f.sol_mint.create_token_account_and_mint_to(0).await;

    mfi_account_f
        .try_bank_borrow(sol_account.key, sol_bank, 2)
        .await?;

    // Advance for half a year and claim half emissions
    test_f.advance_time((SECONDS_PER_YEAR / 2.0) as i64).await;

    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(0).await;

    mfi_account_f
        .try_withdraw_emissions(usdc_bank, lender_token_account_usdc.key)
        .await?;

    let sol_emissions_ta = sol_emissions_mint.create_token_account_and_mint_to(0).await;

    mfi_account_f
        .try_withdraw_emissions(sol_bank, sol_emissions_ta.key)
        .await?;

    assert_eq_with_tolerance!(
        lender_token_account_usdc.balance().await as i64,
        native!(25, "USDC") as i64,
        native!(1, "USDC") as i64
    );

    assert_eq_with_tolerance!(
        sol_emissions_ta.balance().await as i64,
        native!(1, 6) as i64,
        native!(0.1, 6, f64) as i64
    );

    // Advance for another half a year and claim the rest

    test_f.advance_time((SECONDS_PER_YEAR / 2.0) as i64).await;

    {
        let slot = test_f.get_slot().await;
        test_f
            .context
            .borrow_mut()
            .warp_to_slot(slot + 100)
            .unwrap();
    }

    mfi_account_f
        .try_withdraw_emissions(usdc_bank, lender_token_account_usdc.key)
        .await?;

    mfi_account_f
        .try_withdraw_emissions(sol_bank, sol_emissions_ta.key)
        .await?;

    assert_eq_with_tolerance!(
        lender_token_account_usdc.balance().await as i64,
        native!(50, "USDC") as i64,
        native!(1, "USDC") as i64
    );

    assert_eq_with_tolerance!(
        sol_emissions_ta.balance().await as i64,
        native!(2, 6) as i64,
        native!(0.1, 6, f64) as i64
    );

    // Advance a year, and no more USDC emissions can be claimed (drained), SOL emissions can be claimed

    {
        let slot = test_f.get_slot().await;
        test_f
            .context
            .borrow_mut()
            .warp_to_slot(slot + 100)
            .unwrap();
    }

    test_f.advance_time((SECONDS_PER_YEAR / 2.0) as i64).await;

    mfi_account_f
        .try_withdraw_emissions(usdc_bank, lender_token_account_usdc.key)
        .await?;

    mfi_account_f
        .try_withdraw_emissions(sol_bank, sol_emissions_ta.key)
        .await?;

    assert_eq_with_tolerance!(
        lender_token_account_usdc.balance().await as i64,
        native!(50, "USDC") as i64,
        native!(1, "USDC") as i64
    );

    assert_eq_with_tolerance!(
        sol_emissions_ta.balance().await as i64,
        native!(3, 6) as i64,
        native!(0.1, 6, f64) as i64
    );

    // SOL lendeing account can't claim emissions, bc SOL is borrow only emissions
    let sol_lender_emissions = sol_emissions_mint.create_token_account_and_mint_to(0).await;

    sol_lender_account
        .try_withdraw_emissions(sol_bank, sol_lender_emissions.key)
        .await?;

    assert_eq!(sol_lender_emissions.balance().await as i64, 0);

    Ok(())
}

#[tokio::test]
async fn emissions_test_2() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_one_isolated())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);

    let funding_account = test_f.usdc_mint.create_token_account_and_mint_to(100).await;

    usdc_bank
        .try_setup_emissions(
            EMISSIONS_FLAG_LENDING_ACTIVE,
            1_000_000,
            native!(50, "USDC"),
            usdc_bank.mint.key,
            funding_account.key,
        )
        .await?;

    let usdc_bank_data = usdc_bank.load().await;

    assert_eq!(
        usdc_bank_data.emissions_flags,
        EMISSIONS_FLAG_LENDING_ACTIVE
    );

    assert_eq!(usdc_bank_data.emissions_rate, 1_000_000);

    assert_eq!(
        I80F48::from(usdc_bank_data.emissions_remaining),
        I80F48::from_num(native!(50, "USDC"))
    );

    usdc_bank
        .try_update_emissions(
            Some(EMISSIONS_FLAG_BORROW_ACTIVE),
            Some(500_000),
            Some((native!(25, "USDC"), funding_account.key)),
        )
        .await?;

    let usdc_bank_data = usdc_bank.load().await;

    assert_eq!(usdc_bank_data.emissions_flags, EMISSIONS_FLAG_BORROW_ACTIVE);

    assert_eq!(usdc_bank_data.emissions_rate, 500_000);

    assert_eq!(
        I80F48::from(usdc_bank_data.emissions_remaining),
        I80F48::from_num(native!(75, "USDC"))
    );

    Ok(())
}

#[tokio::test]
async fn account_flags() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let mfi_account_f = test_f.create_marginfi_account().await;

    mfi_account_f.try_set_flag(FLASHLOAN_ENABLED_FLAG).await?;

    let mfi_account_data = mfi_account_f.load().await;

    assert_eq!(mfi_account_data.account_flags, FLASHLOAN_ENABLED_FLAG);

    assert!(mfi_account_data.get_flag(FLASHLOAN_ENABLED_FLAG));

    mfi_account_f.try_unset_flag(FLASHLOAN_ENABLED_FLAG).await?;

    let mfi_account_data = mfi_account_f.load().await;

    assert_eq!(mfi_account_data.account_flags, 0);

    let res = mfi_account_f.try_set_flag(DISABLED_FLAG).await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalFlag);

    let res = mfi_account_f.try_unset_flag(IN_FLASHLOAN_FLAG).await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalFlag);

    Ok(())
}

// Flashloan tests
// 1. Flashloan success (1 action)
// 2. Flashloan success (3 actions)
// 3. Flashloan fails because of bad account health
// 4. Flashloan fails because of non whitelisted account
// 5. Flashloan fails because of missing `end_flashloan` ix
// 6. Flashloan fails because of invalid instructions sysvar
// 7. Flashloan fails because of invalid `end_flashloan` ix order
// 8. Flashloan fails because `end_flashloan` ix is for another account
// 9. Flashloan fails because account is already in a flashloan

#[tokio::test]
async fn flashloan_success_1op() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let repay_ix = borrower_mfi_account_f
        .make_bank_repay_ix(
            borrower_token_account_f_sol.key,
            sol_bank,
            1_000,
            Some(true),
        )
        .await;

    let flash_loan_result = borrower_mfi_account_f
        .try_flashloan(vec![borrow_ix, repay_ix], vec![], vec![])
        .await;

    assert!(flash_loan_result.is_ok());

    Ok(())
}

#[tokio::test]
async fn flashloan_success_3op() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;

    // Create borrow and repay instructions
    let mut ixs = Vec::new();
    for _ in 0..3 {
        let borrow_ix = borrower_mfi_account_f
            .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
            .await;
        ixs.push(borrow_ix);

        let repay_ix = borrower_mfi_account_f
            .make_bank_repay_ix(
                borrower_token_account_f_sol.key,
                sol_bank,
                1_000,
                Some(true),
            )
            .await;
        ixs.push(repay_ix);
    }

    ixs.push(
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
    );

    let flash_loan_result = borrower_mfi_account_f
        .try_flashloan(ixs, vec![], vec![])
        .await;

    assert!(flash_loan_result.is_ok());

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_account_health() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let flash_loan_result = borrower_mfi_account_f
        .try_flashloan(vec![borrow_ix], vec![], vec![sol_bank.key])
        .await;

    assert_custom_error!(
        flash_loan_result.unwrap_err(),
        MarginfiError::BadAccountHealth
    );

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_missing_flag() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let repay_ix = borrower_mfi_account_f
        .make_bank_repay_ix(
            borrower_token_account_f_sol.key,
            sol_bank,
            1_000,
            Some(true),
        )
        .await;

    let flash_loan_result = borrower_mfi_account_f
        .try_flashloan(vec![borrow_ix, repay_ix], vec![], vec![])
        .await;

    assert_custom_error!(
        flash_loan_result.unwrap_err(),
        MarginfiError::IllegalFlashloan
    );

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_missing_fe_ix() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let repay_ix = borrower_mfi_account_f
        .make_bank_repay_ix(
            borrower_token_account_f_sol.key,
            sol_bank,
            1_000,
            Some(true),
        )
        .await;

    let mut ixs = vec![borrow_ix, repay_ix];

    let start_ix = borrower_mfi_account_f
        .make_lending_account_start_flashloan_ix(ixs.len() as u64)
        .await;

    ixs.insert(0, start_ix);

    let mut ctx = test_f.context.borrow_mut();

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey().clone()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalFlashloan);

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_missing_invalid_sysvar_ixs() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let repay_ix = borrower_mfi_account_f
        .make_bank_repay_ix(
            borrower_token_account_f_sol.key,
            sol_bank,
            1_000,
            Some(true),
        )
        .await;

    let mut ixs = vec![borrow_ix, repay_ix];

    let start_ix = Instruction {
        program_id: marginfi::id(),
        accounts: marginfi::accounts::LendingAccountStartFlashloan {
            marginfi_account: borrower_mfi_account_f.key,
            signer: test_f.context.borrow().payer.pubkey(),
            ixs_sysvar: Pubkey::default(),
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::LendingAccountStartFlashloan {
            end_index: ixs.len() as u64 + 1,
        }
        .data(),
    };

    let end_ix = borrower_mfi_account_f
        .make_lending_account_end_flashloan_ix(vec![], vec![])
        .await;

    ixs.insert(0, start_ix);
    ixs.push(end_ix);

    let mut ctx = test_f.context.borrow_mut();

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey().clone()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;

    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_invalid_end_fl_order() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let mut ixs = vec![borrow_ix];

    let start_ix = borrower_mfi_account_f
        .make_lending_account_start_flashloan_ix(0)
        .await;

    let end_ix = borrower_mfi_account_f
        .make_lending_account_end_flashloan_ix(vec![], vec![])
        .await;

    ixs.insert(0, start_ix);
    ixs.insert(0, end_ix);

    let mut ctx = test_f.context.borrow_mut();

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey().clone()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalFlashloan);

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_invalid_end_fl_different_m_account() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let mut ixs = vec![borrow_ix];

    let start_ix = borrower_mfi_account_f
        .make_lending_account_start_flashloan_ix(ixs.len() as u64 + 1)
        .await;

    let end_ix = lender_mfi_account_f
        .make_lending_account_end_flashloan_ix(vec![], vec![])
        .await;

    ixs.insert(0, start_ix);
    ixs.push(end_ix);

    let mut ctx = test_f.context.borrow_mut();

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey().clone()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalFlashloan);

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_already_in_flashloan() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_token_account_and_mint_to(0).await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let mut ixs = vec![borrow_ix];

    let start_ix = borrower_mfi_account_f
        .make_lending_account_start_flashloan_ix(ixs.len() as u64 + 2)
        .await;

    let end_ix = borrower_mfi_account_f
        .make_lending_account_end_flashloan_ix(vec![], vec![])
        .await;

    ixs.insert(0, start_ix.clone());
    ixs.insert(0, start_ix.clone());
    ixs.push(end_ix);

    let mut ctx = test_f.context.borrow_mut();

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey().clone()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalFlashloan);

    Ok(())
}

#[tokio::test]
async fn lending_account_close_balance() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let usdc_bank = test_f.get_bank(&BankMint::USDC);
    let sol_eq_bank = test_f.get_bank(&BankMint::SolEquivalent);
    let sol_bank = test_f.get_bank(&BankMint::SOL);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_sol = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_eq_bank, 1_000)
        .await?;

    let lender_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_sol.key, sol_bank, 1_000)
        .await?;

    let res = lender_mfi_account_f.try_balance_close(sol_bank).await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalBalanceState);

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    let borrower_token_account_f_sol_eq = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_f_usdc.key, usdc_bank, 1_000)
        .await?;

    // Borrow SOL EQ
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol_eq.key, sol_eq_bank, 0.01)
        .await;

    assert!(res.is_ok());

    // Borrow SOL
    let res = borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_f_sol.key, sol_bank, 0.01)
        .await;

    assert!(res.is_ok());

    // This issue is not that bad, because the user can still borrow other assets (isolated liab < empty threshold)
    let res = borrower_mfi_account_f.try_balance_close(sol_bank).await;
    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalBalanceState);

    // Let a second go b
    {
        let mut ctx = test_f.context.borrow_mut();
        let mut clock: Clock = ctx.banks_client.get_sysvar().await?;
        // Advance clock by 1 second
        clock.unix_timestamp += 1;
        ctx.set_sysvar(&clock);
    }

    // Repay isolated SOL EQ borrow successfully
    let res = borrower_mfi_account_f
        .try_bank_repay(
            borrower_token_account_f_sol_eq.key,
            sol_eq_bank,
            0.01,
            Some(false),
        )
        .await;
    assert!(res.is_ok());

    // Liability share in balance is smaller than 0.0001, so repay all should fail
    let res = borrower_mfi_account_f
        .try_bank_repay(
            borrower_token_account_f_sol_eq.key,
            sol_eq_bank,
            1,
            Some(true),
        )
        .await;
    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::NoLiabilityFound);

    // This issue is not that bad, because the user can still borrow other assets (isolated liab < empty threshold)
    let res = borrower_mfi_account_f.try_balance_close(sol_eq_bank).await;
    assert!(res.is_ok());

    Ok(())
}
