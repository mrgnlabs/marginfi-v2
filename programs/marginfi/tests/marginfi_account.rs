use anchor_lang::prelude::Clock;
use anchor_lang::{InstructionData, ToAccountMetas};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{prelude::*, *};

use marginfi::{
    prelude::MarginfiError,
    state::{
        marginfi_account::MarginfiAccount,
        marginfi_group::{Bank, BankConfig, BankConfigOpt, BankVaultType},
    },
};
use pretty_assertions::assert_eq;
use solana_program::{instruction::Instruction, program_pack::Pack, system_program};
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
async fn success_deposit() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let mut test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(test_f.usdc_mint.key, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.context.borrow().payer.pubkey();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &owner).await;

    test_f
        .usdc_mint
        .mint_to(&token_account_f.key, native!(1_000, "USDC"))
        .await;

    let res = marginfi_account_f
        .try_bank_deposit(token_account_f.key, &usdc_bank, native!(1_000, "USDC"))
        .await;
    assert!(res.is_ok());

    let marginfi_account = marginfi_account_f.load().await;

    // Check balance is active
    let usdc_bank = usdc_bank.load().await;

    assert!(marginfi_account
        .lending_account
        .get_balance(&test_f.usdc_mint.key, &usdc_bank)
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
    let mut test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.usdc_mint.key,
            BankConfig {
                max_capacity: native!(100, "USDC"),
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await?;

    // Fund user account
    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.context.borrow().payer.pubkey();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &owner).await;

    test_f
        .usdc_mint
        .mint_to(&token_account_f.key, native!(1_000, "USDC"))
        .await;

    // Make lawful deposit
    let res = marginfi_account_f
        .try_bank_deposit(token_account_f.key, &usdc_bank, native!(99, "USDC"))
        .await;
    assert!(res.is_ok());

    // Make unlawful deposit
    let res = marginfi_account_f
        .try_bank_deposit(token_account_f.key, &usdc_bank, native!(101, "USDC"))
        .await;
    assert_custom_error!(res.unwrap_err(), MarginfiError::BankDepositCapacityExceeded);

    Ok(())
}

#[tokio::test]
async fn success_borrow() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let mut test_f = TestFixture::new(None).await;

    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(test_f.usdc_mint.key, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;
    let sol_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(test_f.sol_mint.key, *DEFAULT_SOL_TEST_BANK_CONFIG)
        .await?;

    let user_pubkey = test_f.context.borrow().payer.pubkey();
    let user_usdc_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &user_pubkey).await;
    let user_sol_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.sol_mint.key, &user_pubkey).await;

    test_f
        .sol_mint
        .mint_to(&user_sol_token_account_f.key, native!(1_000, "SOL"))
        .await;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let sol_depositor = test_f.create_marginfi_account().await;
    sol_depositor
        .try_bank_deposit(
            user_sol_token_account_f.key,
            &sol_bank,
            native!(1_000, "SOL"),
        )
        .await?;

    test_f
        .usdc_mint
        .mint_to(&user_usdc_token_account_f.key, native!(1_000, "USDC"))
        .await;

    let liquidity_vault = sol_bank.get_vault(BankVaultType::Liquidity).0;

    test_f
        .sol_mint
        .mint_to(&liquidity_vault, native!(1_000_000, "SOL"))
        .await;

    marginfi_account_f
        .try_bank_deposit(
            user_usdc_token_account_f.key,
            &usdc_bank,
            native!(1_000, "USDC"),
        )
        .await?;

    let res = marginfi_account_f
        .try_bank_withdraw(user_sol_token_account_f.key, &sol_bank, native!(99, "SOL"))
        .await;
    assert!(res.is_ok());

    // Check token balances are correct
    assert_eq!(
        user_usdc_token_account_f.balance().await,
        native!(0, "USDC")
    );
    assert_eq!(user_sol_token_account_f.balance().await, native!(99, "SOL"));

    // TODO: check health is sane

    Ok(())
}

#[tokio::test]
async fn failure_borrow_not_enough_collateral() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let mut test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(test_f.usdc_mint.key, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;
    let sol_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(test_f.sol_mint.key, *DEFAULT_SOL_TEST_BANK_CONFIG)
        .await?;

    let lender_account = test_f.create_marginfi_account().await;
    let user_pubkey = test_f.context.borrow().payer.pubkey();
    let user_sol_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.sol_mint.key, &user_pubkey).await;

    test_f
        .sol_mint
        .mint_to(&user_sol_token_account_f.key, native!(1_000, "SOL"))
        .await;

    lender_account
        .try_bank_deposit(
            user_sol_token_account_f.key,
            &sol_bank,
            native!(1_000, "SOL"),
        )
        .await?;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let user_pubkey = test_f.context.borrow().payer.pubkey();
    let user_usdc_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &user_pubkey).await;
    let user_sol_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.sol_mint.key, &user_pubkey).await;

    test_f
        .usdc_mint
        .mint_to(&user_usdc_token_account_f.key, native!(1_000, "USDC"))
        .await;

    let liquidity_vault = sol_bank.get_vault(BankVaultType::Liquidity).0;

    test_f
        .sol_mint
        .mint_to(&liquidity_vault, native!(1_000_000, "SOL"))
        .await;

    marginfi_account_f
        .try_bank_deposit(
            user_usdc_token_account_f.key,
            &usdc_bank,
            native!(1_000, "USDC"),
        )
        .await?;

    let res = marginfi_account_f
        .try_bank_withdraw(user_sol_token_account_f.key, &sol_bank, native!(101, "SOL"))
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::BadAccountHealth);

    let res = marginfi_account_f
        .try_bank_withdraw(user_sol_token_account_f.key, &sol_bank, native!(100, "SOL"))
        .await;

    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn liquidation_successful() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.usdc_mint.key,
            BankConfig {
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await?;
    let sol_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.sol_mint.key,
            BankConfig {
                deposit_weight_init: I80F48!(1).into(),
                deposit_weight_maint: I80F48!(1).into(),
                ..*DEFAULT_SOL_TEST_BANK_CONFIG
            },
        )
        .await?;

    let depositor = test_f.create_marginfi_account().await;
    let deposit_account = test_f
        .usdc_mint
        .create_and_mint_to(native!(2_000, "USDC"))
        .await;
    depositor
        .try_bank_deposit(deposit_account, &usdc_bank, native!(2_000, "USDC"))
        .await?;

    let borrower = test_f.create_marginfi_account().await;
    let borrower_sol_account = test_f
        .sol_mint
        .create_and_mint_to(native!(100, "SOL"))
        .await;
    let borrower_usdc_account = test_f.usdc_mint.create_and_mint_to(0).await;
    // Borrower deposits 100 SOL worth of $1000
    borrower
        .try_bank_deposit(borrower_sol_account, &sol_bank, native!(100, "SOL"))
        .await?;
    // Borrower borrows $999
    borrower
        .try_bank_withdraw(borrower_usdc_account, &usdc_bank, native!(999, "USDC"))
        .await?;

    sol_bank
        .update_config(BankConfigOpt {
            deposit_weight_init: Some(I80F48!(0.25).into()),
            deposit_weight_maint: Some(I80F48!(0.5).into()),
            ..Default::default()
        })
        .await?;
    depositor
        .try_liquidate(&borrower, &sol_bank, native!(1, "SOL"), &usdc_bank)
        .await?;

    // Checks
    let sol_bank: Bank = sol_bank.load().await;
    let usdc_bank: Bank = usdc_bank.load().await;

    let depositor_ma = depositor.load().await;
    let borrower_ma = borrower.load().await;

    // Depositors should have 1 SOL
    assert_eq!(
        sol_bank
            .get_deposit_amount(
                depositor_ma.lending_account.balances[1]
                    .deposit_shares
                    .into()
            )
            .unwrap(),
        I80F48::from(native!(1, "SOL"))
    );

    // Depositors should have 1990.25 USDC
    assert_eq_noise!(
        usdc_bank
            .get_deposit_amount(
                depositor_ma.lending_account.balances[0]
                    .deposit_shares
                    .into()
            )
            .unwrap(),
        I80F48::from(native!(1990.25, "USDC", f64)),
        native!(0.00001, "USDC", f64)
    );

    // Borrower should have 99 SOL
    assert_eq!(
        sol_bank
            .get_deposit_amount(
                borrower_ma.lending_account.balances[0]
                    .deposit_shares
                    .into()
            )
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
    let mut ctx = test_f.context.borrow_mut();
    let insurance_fund = ctx
        .banks_client
        .get_account(usdc_bank.insurance_vault)
        .await?
        .unwrap();
    let token_account =
        anchor_spl::token::spl_token::state::Account::unpack_from_slice(&insurance_fund.data)?;

    assert_eq_noise!(
        token_account.amount as i64,
        native!(0.25, "USDC", f64) as i64,
        1
    );

    Ok(())
}
#[tokio::test]
async fn liquidation_failed_liquidatee_not_unhealthy() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.usdc_mint.key,
            BankConfig {
                deposit_weight_maint: I80F48!(1).into(),
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await?;
    let sol_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.sol_mint.key,
            BankConfig {
                deposit_weight_init: I80F48!(1).into(),
                deposit_weight_maint: I80F48!(1).into(),
                ..*DEFAULT_SOL_TEST_BANK_CONFIG
            },
        )
        .await?;

    let depositor = test_f.create_marginfi_account().await;
    let deposit_account = test_f
        .usdc_mint
        .create_and_mint_to(native!(200, "USDC"))
        .await;
    depositor
        .try_bank_deposit(deposit_account, &usdc_bank, native!(200, "USDC"))
        .await?;

    let borrower = test_f.create_marginfi_account().await;
    let borrower_sol_account = test_f
        .sol_mint
        .create_and_mint_to(native!(100, "SOL"))
        .await;
    let borrower_usdc_account = test_f.usdc_mint.create_and_mint_to(0).await;
    borrower
        .try_bank_deposit(borrower_sol_account, &sol_bank, native!(100, "SOL"))
        .await?;
    borrower
        .try_bank_withdraw(borrower_usdc_account, &usdc_bank, native!(100, "USDC"))
        .await?;

    let res = depositor
        .try_liquidate(&borrower, &sol_bank, native!(1, "SOL"), &usdc_bank)
        .await;

    assert!(res.is_err());

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    Ok(())
}
#[tokio::test]
async fn liquidation_failed_liquidation_too_severe() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.usdc_mint.key,
            BankConfig {
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await?;
    let sol_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.sol_mint.key,
            BankConfig {
                ..*DEFAULT_SOL_TEST_BANK_CONFIG
            },
        )
        .await?;

    let depositor = test_f.create_marginfi_account().await;
    let deposit_account = test_f
        .usdc_mint
        .create_and_mint_to(native!(200, "USDC"))
        .await;
    depositor
        .try_bank_deposit(deposit_account, &usdc_bank, native!(200, "USDC"))
        .await?;

    let borrower = test_f.create_marginfi_account().await;
    let borrower_sol_account = test_f.sol_mint.create_and_mint_to(native!(10, "SOL")).await;
    let borrower_usdc_account = test_f.usdc_mint.create_and_mint_to(0).await;
    borrower
        .try_bank_deposit(borrower_sol_account, &sol_bank, native!(10, "SOL"))
        .await?;
    borrower
        .try_bank_withdraw(borrower_usdc_account, &usdc_bank, native!(61, "USDC"))
        .await?;

    sol_bank
        .update_config(BankConfigOpt {
            deposit_weight_init: Some(I80F48!(0.25).into()),
            deposit_weight_maint: Some(I80F48!(0.5).into()),
            ..Default::default()
        })
        .await?;

    let res = depositor
        .try_liquidate(&borrower, &sol_bank, native!(10, "SOL"), &usdc_bank)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    let res = depositor
        .try_liquidate(&borrower, &sol_bank, native!(1, "SOL"), &usdc_bank)
        .await;

    assert!(res.is_ok());

    Ok(())
}
#[tokio::test]
async fn liquidation_failed_liquidator_no_collateral() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.usdc_mint.key,
            BankConfig {
                liability_weight_init: I80F48!(1.2).into(),
                liability_weight_maint: I80F48!(1.1).into(),
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await?;
    let sol_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.sol_mint.key,
            BankConfig {
                ..*DEFAULT_SOL_TEST_BANK_CONFIG
            },
        )
        .await?;
    let sol_2_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.sol_equivalent_mint.key,
            BankConfig {
                ..*DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG
            },
        )
        .await?;

    let depositor = test_f.create_marginfi_account().await;
    let deposit_account = test_f
        .usdc_mint
        .create_and_mint_to(native!(200, "USDC"))
        .await;
    depositor
        .try_bank_deposit(deposit_account, &usdc_bank, native!(200, "USDC"))
        .await?;

    let borrower = test_f.create_marginfi_account().await;
    let borrower_sol_account = test_f
        .sol_mint
        .create_and_mint_to(native!(100, "SOL"))
        .await;
    let borrower_sol_2_account = test_f
        .sol_equivalent_mint
        .create_and_mint_to(native!(100, "SOL"))
        .await;
    let borrower_usdc_account = test_f.usdc_mint.create_and_mint_to(0).await;
    borrower
        .try_bank_deposit(borrower_sol_account, &sol_bank, native!(10, "SOL"))
        .await?;
    borrower
        .try_bank_deposit(borrower_sol_2_account, &sol_2_bank, native!(1, "SOL"))
        .await?;

    borrower
        .try_bank_withdraw(borrower_usdc_account, &usdc_bank, native!(60, "USDC"))
        .await?;

    sol_bank
        .update_config(BankConfigOpt {
            deposit_weight_init: Some(I80F48!(0.25).into()),
            deposit_weight_maint: Some(I80F48!(0.3).into()),
            ..Default::default()
        })
        .await?;

    let res = depositor
        .try_liquidate(&borrower, &sol_2_bank, native!(2, "SOL"), &usdc_bank)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::BorrowingNotAllowed);

    let res = depositor
        .try_liquidate(&borrower, &sol_2_bank, native!(1, "SOL"), &usdc_bank)
        .await;

    assert!(res.is_ok());
    Ok(())
}

#[tokio::test]
async fn liquidation_failed_bank_not_liquidatable() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    // Setup sample bank
    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.usdc_mint.key,
            BankConfig {
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await?;
    let sol_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.sol_mint.key,
            BankConfig {
                ..*DEFAULT_SOL_TEST_BANK_CONFIG
            },
        )
        .await?;
    let sol_2_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            test_f.sol_equivalent_mint.key,
            BankConfig {
                ..*DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG
            },
        )
        .await?;

    let depositor = test_f.create_marginfi_account().await;
    let deposit_account = test_f
        .usdc_mint
        .create_and_mint_to(native!(200, "USDC"))
        .await;
    depositor
        .try_bank_deposit(deposit_account, &usdc_bank, native!(200, "USDC"))
        .await?;

    let borrower = test_f.create_marginfi_account().await;
    let borrower_sol_account = test_f
        .sol_mint
        .create_and_mint_to(native!(100, "SOL"))
        .await;
    let borrower_sol_2_account = test_f
        .sol_equivalent_mint
        .create_and_mint_to(native!(100, "SOL"))
        .await;
    let borrower_usdc_account = test_f.usdc_mint.create_and_mint_to(0).await;
    borrower
        .try_bank_deposit(borrower_sol_account, &sol_bank, native!(10, "SOL"))
        .await?;
    borrower
        .try_bank_deposit(borrower_sol_2_account, &sol_2_bank, native!(1, "SOL"))
        .await?;

    borrower
        .try_bank_withdraw(borrower_usdc_account, &usdc_bank, native!(60, "USDC"))
        .await?;

    sol_bank
        .update_config(BankConfigOpt {
            deposit_weight_init: Some(I80F48!(0.25).into()),
            deposit_weight_maint: Some(I80F48!(0.4).into()),
            ..Default::default()
        })
        .await?;

    let res = depositor
        .try_liquidate(&borrower, &sol_2_bank, native!(1, "SOL"), &sol_bank)
        .await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalLiquidation);

    let res = depositor
        .try_liquidate(&borrower, &sol_bank, native!(1, "SOL"), &usdc_bank)
        .await;

    assert!(res.is_ok());
    Ok(())
}

#[tokio::test]
async fn automatic_interest_payments() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let mut test_f = TestFixture::new(None).await;

    let usdc_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(test_f.usdc_mint.key, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;
    let sol_bank = test_f
        .marginfi_group
        .try_lending_pool_add_bank(test_f.sol_mint.key, *DEFAULT_SOL_TEST_BANK_CONFIG)
        .await?;

    let user_pubkey = test_f.context.borrow().payer.pubkey();
    let user_usdc_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &user_pubkey).await;
    let user_sol_token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.sol_mint.key, &user_pubkey).await;

    test_f
        .sol_mint
        .mint_to(&user_sol_token_account_f.key, native!(1_000, "SOL"))
        .await;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let sol_depositor = test_f.create_marginfi_account().await;
    sol_depositor
        .try_bank_deposit(
            user_sol_token_account_f.key,
            &sol_bank,
            native!(1_000, "SOL"),
        )
        .await?;

    test_f
        .usdc_mint
        .mint_to(&user_usdc_token_account_f.key, native!(1_000, "USDC"))
        .await;

    let liquidity_vault = sol_bank.get_vault(BankVaultType::Liquidity).0;

    test_f
        .sol_mint
        .mint_to(&liquidity_vault, native!(1_000_000, "SOL"))
        .await;

    marginfi_account_f
        .try_bank_deposit(
            user_usdc_token_account_f.key,
            &usdc_bank,
            native!(1_000, "USDC"),
        )
        .await?;

    marginfi_account_f
        .try_bank_withdraw(user_sol_token_account_f.key, &sol_bank, native!(99, "SOL"))
        .await?;

    {
        let mut ctx = test_f.context.borrow_mut();
        let mut clock: Clock = ctx.banks_client.get_sysvar().await?;
        // Advance clock by 1 year
        clock.unix_timestamp += 365 * 24 * 60 * 60;
        ctx.set_sysvar(&clock);
    }

    marginfi_account_f
        .try_bank_deposit(user_sol_token_account_f.key, &sol_bank, native!(99, "SOL"))
        .await?;

    let sol_ba = sol_bank.load().await;

    let b_ma = marginfi_account_f.load().await;

    assert_eq_noise!(
        sol_ba
            .get_liability_amount(b_ma.lending_account.balances[1].liability_shares.into())
            .unwrap(),
        I80F48::from(native!(11.76, "SOL", f64)),
        native!(0.00001, "SOL", f64)
    );

    let lender_ma = sol_depositor.load().await;

    assert_eq_noise!(
        sol_ba
            .get_deposit_amount(lender_ma.lending_account.balances[0].deposit_shares.into())
            .unwrap(),
        I80F48::from(native!(1011.76, "SOL", f64)),
        native!(0.00001, "SOL", f64)
    );
    // TODO: check health is sane

    Ok(())
}
