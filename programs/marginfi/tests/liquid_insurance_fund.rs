use fixed_macro::types::I80F48;
use fixtures::{
    lif::LiquidInsuranceFundAccountFixture,
    spl::{MintFixture, TokenAccountFixture},
    test::{
        BankMint, TestBankSetting, TestFixture, TestSettings, DEFAULT_SOL_TEST_BANK_CONFIG,
        DEFAULT_USDC_TEST_BANK_CONFIG,
    },
};
use marginfi::state::marginfi_group::{BankConfig, BankConfigOpt, BankVaultType, GroupConfig};
use solana_program_test::tokio;

#[tokio::test]
async fn marginfi_liquid_insurance_fund_create_success() -> anyhow::Result<()> {
    // first create bank
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

    let bank_fixture = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&bank_asset_mint_fixture, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;

    let min_withdraw_period = 60_u64 * 60_u64 * 24_u64 * 14_u64; // 2 weeks

    test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(&bank_fixture, min_withdraw_period)
        .await?;

    Ok(())
}

#[tokio::test]
async fn marginfi_liquid_insurance_fund_admin_deposit_withdraw_success() -> anyhow::Result<()> {
    // first create bank
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    let min_withdraw_period = 60_u64 * 60_u64 * 24_u64 * 14_u64; // 2 weeks

    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(usdc_bank_f, min_withdraw_period)
        .await?;

    // Deposit through deposit ix should update admin shares
    let source_account = test_f.usdc_mint.create_token_account_and_mint_to(100).await;

    usdc_bank_f
        .try_admin_deposit_insurance(&source_account, 100_000_000)
        .await
        .unwrap();

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48!(100_000_000.0));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        usdc_bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 100_000_000);
    assert_eq!(source_account.balance().await, 0);

    // Withdraw
    usdc_bank_f
        .try_admin_withdraw_insurance(&source_account, I80F48!(100_000_000))
        .await
        .unwrap();

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48!(0.0));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        usdc_bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 0);
    assert_eq!(source_account.balance().await, 100_000_000);

    Ok(())
}

#[tokio::test]
async fn marginfi_liquid_insurance_fund_user_deposit_withdraw_success() -> anyhow::Result<()> {
    // first create bank
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    // Seconds
    let min_withdraw_period = 2;

    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(usdc_bank_f, min_withdraw_period)
        .await?;

    // Create user lif and fund user with usdc
    let mut user_lif_account: LiquidInsuranceFundAccountFixture =
        lif_f.create_lif_user_account().await.unwrap();
    let source_account = usdc_bank_f
        .mint
        .create_token_account_with_owner_and_mint_to(user_lif_account.user(), 100.0)
        .await;

    // User deposit
    user_lif_account
        .try_user_deposit_insurance(usdc_bank_f, &source_account, 100_000_000)
        .await
        .unwrap();

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(admin_shares, I80F48!(0.0));
    assert_eq!(total_shares, I80F48!(100_000_000.0));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        usdc_bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 100_000_000);
    assert_eq!(source_account.balance().await, 0);

    // Initiate withdrawal of first half of shares
    user_lif_account
        .try_user_withdraw_insurance_request(&usdc_bank_f, Some(I80F48!(50_000_000)), 1)
        .await
        .unwrap();

    // Check withdrawal and deposit state
    let account = user_lif_account.load().await;
    let withdrawal = &account.withdrawals[0];
    assert_eq!(withdrawal.shares(), I80F48!(50_000_000));
    let deposit = account.get_deposit(&lif_f.key).unwrap();
    assert_eq!(deposit.shares(), I80F48!(50_000_000));

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(admin_shares, I80F48!(0.0));
    assert_eq!(total_shares, I80F48!(100_000_000));

    // Initiate withdrawal of second half of shares
    user_lif_account
        .try_user_withdraw_insurance_request(&usdc_bank_f, Some(I80F48!(50_000_000)), 2)
        .await
        .unwrap();

    // Check withdrawal and deposit state (deposit no longer exists)
    let account = user_lif_account.load().await;
    let withdrawal = &account.withdrawals[1];
    assert_eq!(withdrawal.shares(), I80F48!(50_000_000));
    assert!(account.get_deposit(&lif_f.key).is_none());
    assert_eq!(account.balances[0], Default::default());

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(admin_shares, I80F48!(0.0));
    assert_eq!(total_shares, I80F48!(100_000_000));

    // Withdrawal claim should fail now, but should succeed after withdrawal period
    assert!(user_lif_account
        .try_user_withdraw_insurance_claim(&usdc_bank_f, &source_account, 1)
        .await
        .is_err());
    let old_time = test_f.get_clock().await.unix_timestamp;
    test_f.advance_time(min_withdraw_period as i64).await;
    let new_time = test_f.get_clock().await.unix_timestamp;
    eprintln!("time {} -> {}", old_time, new_time);
    user_lif_account
        .try_user_withdraw_insurance_claim(&usdc_bank_f, &source_account, 2)
        .await
        .unwrap();

    // Check only first withdrawal was freed
    let account = user_lif_account.load().await;
    assert_eq!(account.withdrawals[0], Default::default());
    assert!(!account.withdrawals[1].is_empty());

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        usdc_bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 50_000_000);
    assert_eq!(source_account.balance().await, 50_000_000);

    user_lif_account
        .try_user_withdraw_insurance_claim(&usdc_bank_f, &source_account, 3)
        .await
        .unwrap();

    // Check second withdrawal was freed
    let account = user_lif_account.load().await;
    assert!(account.withdrawals[1].is_empty());

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        usdc_bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 0);
    assert_eq!(source_account.balance().await, 100_000_000);

    Ok(())
}

#[tokio::test]
async fn marginfi_liquid_insurance_fund_admin_user_withdraw_limits() -> anyhow::Result<()> {
    // first create bank and lif
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    // Seconds
    let min_withdraw_period = 2;
    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(usdc_bank_f, min_withdraw_period)
        .await?;

    // Admin deposit
    let admin_source_account = test_f.usdc_mint.create_token_account_and_mint_to(100).await;
    usdc_bank_f
        .try_admin_deposit_insurance(&admin_source_account, 100_000_000)
        .await
        .unwrap();

    // User deposit
    let user_lif_account: LiquidInsuranceFundAccountFixture =
        lif_f.create_lif_user_account().await.unwrap();
    let user_source_account = usdc_bank_f
        .mint
        .create_token_account_with_owner_and_mint_to(user_lif_account.user(), 100.0)
        .await;

    // Deposit through deposit ix should update
    user_lif_account
        .try_user_deposit_insurance(usdc_bank_f, &user_source_account, 100_000_000)
        .await
        .unwrap();

    let lif = lif_f.load().await;
    assert_eq!(lif.total_shares, I80F48!(200_000_000).into());
    assert_eq!(lif.admin_shares, I80F48!(100_000_000).into());

    // Admin should not be able to withdraw more than 100_000_000 shares
    assert!(usdc_bank_f
        .try_admin_withdraw_insurance(&admin_source_account, I80F48!(100_000_001))
        .await
        .is_err());

    // User should not be able to submit claim for more than 100_000_000 shares
    assert!(user_lif_account
        .try_user_withdraw_insurance_request(&usdc_bank_f, Some(I80F48!(100_000_001)), 1)
        .await
        .is_err());

    // Both should be able to withdraw their full shares
    usdc_bank_f
        .try_admin_withdraw_insurance(&admin_source_account, I80F48!(100_000_000))
        .await
        .unwrap();

    // User should not be able to submit claim for more than 100_000_000 shares
    user_lif_account
        .try_user_withdraw_insurance_request(&usdc_bank_f, Some(I80F48!(100_000_000)), 1)
        .await
        .unwrap();
    test_f.advance_time(2).await;
    user_lif_account
        .try_user_withdraw_insurance_claim(&usdc_bank_f, &user_source_account, 1)
        .await
        .unwrap();

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        usdc_bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 0);
    assert_eq!(admin_source_account.balance().await, 100_000_000);
    assert_eq!(user_source_account.balance().await, 100_000_000);

    Ok(())
}

#[tokio::test]
async fn marginfi_liquid_insurance_fund_admin_deposit_withdraw_with_liquidation_success(
) -> anyhow::Result<()> {
    // first create bank
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    let min_withdraw_period = 60_u64 * 60_u64 * 24_u64 * 14_u64; // 2 weeks

    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(usdc_bank_f, min_withdraw_period)
        .await?;

    // Deposit through deposit ix should update admin shares
    let source_account = test_f.usdc_mint.create_token_account_and_mint_to(100).await;

    usdc_bank_f
        .try_admin_deposit_insurance(&source_account, 100_000_000)
        .await
        .unwrap();

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48!(100_000_000.0));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        usdc_bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 100_000_000);
    assert_eq!(source_account.balance().await, 0);

    // Liquidation occurs
    liquidation(&test_f).await?;

    // Withdraw
    usdc_bank_f
        .try_admin_withdraw_insurance(&source_account, I80F48!(100_000_000))
        .await
        .unwrap();

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48!(0.0));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        usdc_bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 1);
    assert!(dbg!(source_account.balance().await) > 100_000_000);

    Ok(())
}

#[tokio::test]
async fn marginfi_liquid_insurance_fund_admin_deposit_withdraw_with_bad_debt_success(
) -> anyhow::Result<()> {
    // first create bank
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
    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    let min_withdraw_period = 60_u64 * 60_u64 * 24_u64 * 14_u64; // 2 weeks

    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(usdc_bank_f, min_withdraw_period)
        .await?;

    // Deposit through deposit ix should update admin shares
    let source_account = test_f.usdc_mint.create_token_account_and_mint_to(100).await;

    usdc_bank_f
        .try_admin_deposit_insurance(&source_account, 100_000_000)
        .await
        .unwrap();

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48!(100_000_000.0));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        usdc_bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 100_000_000);
    assert_eq!(source_account.balance().await, 0);

    // Liquidation occurs
    bad_debt(&mut test_f).await?;

    // Withdraw
    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);
    usdc_bank_f
        .try_admin_withdraw_insurance(&source_account, I80F48!(100_000_000))
        .await
        .unwrap();

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48!(0.0));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        usdc_bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 0);
    assert!(dbg!(source_account.balance().await) < 100_000_000);

    Ok(())
}

// TODO:
// [x] users deposit/withdraw success
// [x] admin cannot withdraw more than their share value (when user deposits are present)
// [x] users cannot withdraw more than their share value (when other user deposits or admin deposits are present)
// [x] with liquidation
// [x] with bad debt

async fn liquidation(test_f: &TestFixture) -> anyhow::Result<()> {
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

    Ok(())
}

async fn bad_debt(test_f: &mut TestFixture) -> anyhow::Result<()> {
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
        .value = 0_i128.to_le_bytes();
    borrower_account.set_account(&borrower_mfi_account).await?;

    let bank = test_f.get_bank(&BankMint::USDC);
    test_f
        .marginfi_group
        .try_handle_bankruptcy(bank, &borrower_account)
        .await
        .unwrap();

    Ok(())
}
