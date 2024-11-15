use anchor_spl::token_2022::spl_token_2022::extension::{
    transfer_fee::TransferFeeConfig, BaseStateWithExtensions,
};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{
    bank::BankFixture,
    lif::LiquidInsuranceFundAccountFixture,
    spl::TokenAccountFixture,
    test::{BankMint, TestFixture, TestSettings},
};
use marginfi::state::marginfi_group::{BankConfigOpt, BankVaultType};
use solana_program_test::tokio;
use test_case::test_case;

#[test_case(BankMint::Usdc)]
#[test_case(BankMint::Sol)]
#[test_case(BankMint::PyUSD)]
#[test_case(BankMint::T22WithFee)]
#[tokio::test]
async fn marginfi_liquid_insurance_fund_create_success(bank_mint: BankMint) -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let bank_fixture = test_f.banks.get_mut(&bank_mint).unwrap();

    let min_withdraw_period = 60_u64 * 60_u64 * 24_u64 * 14_u64; // 2 weeks

    test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(&bank_fixture, min_withdraw_period)
        .await?;

    Ok(())
}

#[test_case(BankMint::Usdc)]
#[test_case(BankMint::Sol)]
#[test_case(BankMint::PyUSD)]
#[test_case(BankMint::T22WithFee)]
#[tokio::test]
async fn marginfi_liquid_insurance_fund_admin_deposit_withdraw_success(
    bank_mint: BankMint,
) -> anyhow::Result<()> {
    // first create bank
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let bank_f = test_f.get_bank(&bank_mint);

    let deposit_amount_ui = 100_f64;
    let deposit_amount: u64 =
        (deposit_amount_ui as u64) * 10_u64.pow(bank_f.mint.mint.decimals as u32);
    let min_withdraw_period = 60_u64 * 60_u64 * 24_u64 * 14_u64; // 2 weeks

    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(bank_f, min_withdraw_period)
        .await?;

    // Deposit through deposit ix should update admin shares
    let source_account = bank_f
        .mint
        .create_token_account_and_mint_to(deposit_amount_ui)
        .await;

    bank_f
        .try_admin_deposit_insurance(&source_account, deposit_amount)
        .await
        .unwrap();

    let first_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| tf.calculate_epoch_fee(0, deposit_amount).unwrap_or(0))
        .unwrap_or(0);
    let postfee_deposit_amount = deposit_amount - first_transfer_fee;

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48::from(postfee_deposit_amount));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, postfee_deposit_amount);
    assert_eq!(source_account.balance().await, 0);

    // Withdraw
    bank_f
        .try_admin_withdraw_insurance(&source_account, admin_shares)
        .await
        .unwrap();

    let second_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_epoch_fee(0, deposit_amount - first_transfer_fee)
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let postfee_withdraw_amount = deposit_amount - first_transfer_fee - second_transfer_fee;

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48!(0.0));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 0);
    assert_eq!(source_account.balance().await, postfee_withdraw_amount);

    Ok(())
}

#[test_case(BankMint::Usdc)]
#[test_case(BankMint::Sol)]
#[test_case(BankMint::PyUSD)]
#[test_case(BankMint::T22WithFee)]
#[tokio::test]
async fn marginfi_liquid_convert_insurance_fund_admin_deposit_withdraw_success(
    bank_mint: BankMint,
) -> anyhow::Result<()> {
    // first create bank
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let mint_f = test_f.get_mint_fixture(&bank_mint);
    let bank_f = test_f.get_bank(&bank_mint);
    let insurance_vault_key = bank_f.get_vault(BankVaultType::Insurance).0;

    // Transfer to insurance fund vault
    let deposit_amount_ui = 100_f64;
    let deposit_amount: u64 =
        (deposit_amount_ui as u64) * 10_u64.pow(bank_f.mint.mint.decimals as u32);
    let min_withdraw_period = 60_u64 * 60_u64 * 24_u64 * 14_u64; // 2 weeks
    let source_account = bank_f
        .mint
        .create_token_account_with_owner_and_mint_to(&test_f.payer_keypair(), deposit_amount_ui)
        .await;
    source_account
        .transfer(
            &test_f.payer_keypair(),
            mint_f,
            &insurance_vault_key,
            deposit_amount,
        )
        .await
        .unwrap();

    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(bank_f, min_withdraw_period)
        .await?;

    let first_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| tf.calculate_epoch_fee(0, deposit_amount).unwrap_or(0))
        .unwrap_or(0);
    let postfee_deposit_amount = deposit_amount - first_transfer_fee;

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48::from(postfee_deposit_amount));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, postfee_deposit_amount);
    assert_eq!(source_account.balance().await, 0);

    // Withdraw
    bank_f
        .try_admin_withdraw_insurance(&source_account, admin_shares)
        .await
        .unwrap();

    let second_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_epoch_fee(0, deposit_amount - first_transfer_fee)
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let postfee_withdraw_amount = deposit_amount - first_transfer_fee - second_transfer_fee;

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48!(0.0));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 0);
    assert_eq!(source_account.balance().await, postfee_withdraw_amount);

    Ok(())
}

#[test_case(BankMint::Usdc)]
#[test_case(BankMint::Sol)]
#[test_case(BankMint::PyUSD)]
#[test_case(BankMint::T22WithFee)]
#[tokio::test]
async fn marginfi_liquid_insurance_fund_user_deposit_withdraw_success(
    bank_mint: BankMint,
) -> anyhow::Result<()> {
    // first create bank
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let bank_f = test_f.get_bank(&bank_mint);

    let deposit_amount_ui = 100_f64;
    let deposit_amount: u64 =
        (deposit_amount_ui as u64) * 10_u64.pow(bank_f.mint.mint.decimals as u32);

    // Seconds
    let min_withdraw_period = 2;

    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(bank_f, min_withdraw_period)
        .await?;

    // Create user lif and fund user with usdc
    let mut user_lif_account: LiquidInsuranceFundAccountFixture =
        lif_f.create_lif_user_account().await.unwrap();
    let source_account = bank_f
        .mint
        .create_token_account_with_owner_and_mint_to(user_lif_account.user(), deposit_amount_ui)
        .await;

    // User deposit
    user_lif_account
        .try_user_deposit_insurance(bank_f, &source_account, deposit_amount)
        .await
        .unwrap();

    let first_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| tf.calculate_epoch_fee(0, deposit_amount).unwrap_or(0))
        .unwrap_or(0);
    let postfee_deposit_amount = deposit_amount - first_transfer_fee;

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(admin_shares, I80F48!(0.0));
    assert_eq!(total_shares, I80F48::from(postfee_deposit_amount));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, postfee_deposit_amount);
    assert_eq!(source_account.balance().await, 0);

    // Initiate withdrawal of first half of shares
    user_lif_account
        .try_user_withdraw_insurance_request(&bank_f, Some(total_shares >> 1), 1)
        .await
        .unwrap();

    // Check withdrawal and deposit state
    let account = user_lif_account.load().await;
    let withdrawal = &account.withdrawals[0];
    assert_eq!(withdrawal.shares(), total_shares >> 1);
    let deposit = account.get_deposit(&lif_f.key).unwrap();
    assert_eq!(deposit.shares(), total_shares >> 1);

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(admin_shares, I80F48!(0.0));
    assert_eq!(total_shares, I80F48::from(postfee_deposit_amount));

    // Initiate withdrawal of second half of shares
    user_lif_account
        .try_user_withdraw_insurance_request(&bank_f, Some(total_shares >> 1), 2)
        .await
        .unwrap();

    // Check withdrawal and deposit state (deposit no longer exists)
    let account = user_lif_account.load().await;
    let withdrawal = &account.withdrawals[1];
    assert_eq!(withdrawal.shares(), total_shares >> 1);
    assert!(account.get_deposit(&lif_f.key).is_none());
    assert_eq!(account.balances[0], Default::default());

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(admin_shares, I80F48!(0.0));
    assert_eq!(total_shares, I80F48::from(postfee_deposit_amount));

    // Withdrawal claim should fail now, but should succeed after withdrawal period
    assert!(user_lif_account
        .try_user_withdraw_insurance_claim(&bank_f, &source_account, 1)
        .await
        .is_err());
    let old_time = test_f.get_clock().await.unix_timestamp;
    test_f.advance_time(min_withdraw_period as i64).await;
    let new_time = test_f.get_clock().await.unix_timestamp;
    eprintln!("time {} -> {}", old_time, new_time);
    user_lif_account
        .try_user_withdraw_insurance_claim(&bank_f, &source_account, 2)
        .await
        .unwrap();

    // Check only first withdrawal was freed
    let account = user_lif_account.load().await;
    assert_eq!(account.withdrawals[0], Default::default());
    assert!(!account.withdrawals[1].is_empty());

    let second_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_epoch_fee(0, postfee_deposit_amount >> 1)
                .unwrap_or(0)
        })
        .unwrap_or(0);

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, postfee_deposit_amount >> 1);
    assert_eq!(
        source_account.balance().await,
        (postfee_deposit_amount >> 1) - second_transfer_fee
    );

    user_lif_account
        .try_user_withdraw_insurance_claim(&bank_f, &source_account, 3)
        .await
        .unwrap();

    // Check second withdrawal was freed
    let account = user_lif_account.load().await;
    assert!(account.withdrawals[1].is_empty());

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 0);
    assert_eq!(
        source_account.balance().await,
        postfee_deposit_amount - 2 * second_transfer_fee
    );

    Ok(())
}

#[test_case(BankMint::Usdc)]
#[test_case(BankMint::Sol)]
#[test_case(BankMint::PyUSD)]
#[test_case(BankMint::T22WithFee)]
#[tokio::test]
async fn marginfi_liquid_insurance_fund_admin_user_withdraw_limits(
    bank_mint: BankMint,
) -> anyhow::Result<()> {
    // first create bank and lif
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let bank_f = test_f.get_bank(&bank_mint);

    let deposit_amount_ui = 100_f64;
    let deposit_amount: u64 =
        (deposit_amount_ui as u64) * 10_u64.pow(bank_f.mint.mint.decimals as u32);

    // Seconds
    let min_withdraw_period = 2;
    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(bank_f, min_withdraw_period)
        .await?;

    // Admin deposit
    let admin_source_account = bank_f
        .mint
        .create_token_account_and_mint_to(deposit_amount_ui)
        .await;
    bank_f
        .try_admin_deposit_insurance(&admin_source_account, deposit_amount)
        .await
        .unwrap();

    // User deposit
    let user_lif_account: LiquidInsuranceFundAccountFixture =
        lif_f.create_lif_user_account().await.unwrap();
    let user_source_account = bank_f
        .mint
        .create_token_account_with_owner_and_mint_to(user_lif_account.user(), deposit_amount_ui)
        .await;

    // Deposit through deposit ix should update
    user_lif_account
        .try_user_deposit_insurance(bank_f, &user_source_account, deposit_amount)
        .await
        .unwrap();

    let first_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| tf.calculate_epoch_fee(0, deposit_amount).unwrap_or(0))
        .unwrap_or(0);
    let postfee_deposit_amount = deposit_amount - first_transfer_fee;

    let lif = lif_f.load().await;
    assert_eq!(
        lif.total_shares,
        I80F48::from(2 * postfee_deposit_amount).into()
    );
    assert_eq!(
        lif.admin_shares,
        I80F48::from(postfee_deposit_amount).into()
    );

    // Admin should not be able to withdraw more than 100_000_000 shares
    assert!(bank_f
        .try_admin_withdraw_insurance(
            &admin_source_account,
            I80F48::from(postfee_deposit_amount + 1)
        )
        .await
        .is_err());

    // User should not be able to submit claim for more than 100_000_000 shares
    assert!(user_lif_account
        .try_user_withdraw_insurance_request(
            &bank_f,
            Some(I80F48::from(postfee_deposit_amount + 1)),
            1
        )
        .await
        .is_err());

    // Both should be able to withdraw their full shares
    bank_f
        .try_admin_withdraw_insurance(&admin_source_account, postfee_deposit_amount.into())
        .await
        .unwrap();

    // User should not be able to submit claim for more than 100_000_000 shares
    user_lif_account
        .try_user_withdraw_insurance_request(&bank_f, Some(postfee_deposit_amount.into()), 1)
        .await
        .unwrap();
    test_f.advance_time(2).await;
    user_lif_account
        .try_user_withdraw_insurance_claim(&bank_f, &user_source_account, 1)
        .await
        .unwrap();

    // Check balance
    let second_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_epoch_fee(0, postfee_deposit_amount)
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 0);
    assert_eq!(
        admin_source_account.balance().await,
        deposit_amount - first_transfer_fee - second_transfer_fee
    );
    assert_eq!(
        user_source_account.balance().await,
        deposit_amount - first_transfer_fee - second_transfer_fee
    );

    Ok(())
}

#[test_case(BankMint::Usdc)]
#[test_case(BankMint::Sol)]
#[test_case(BankMint::PyUSD)]
#[test_case(BankMint::T22WithFee)]
#[tokio::test]
async fn marginfi_liquid_insurance_fund_admin_deposit_withdraw_with_liquidation_success(
    bank_mint: BankMint,
) -> anyhow::Result<()> {
    // first create bank
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let bank_f = test_f.get_bank(&bank_mint);

    let deposit_amount_ui = 100_f64;
    let deposit_amount: u64 =
        (deposit_amount_ui as u64) * 10_u64.pow(bank_f.mint.mint.decimals as u32);

    let min_withdraw_period = 60_u64 * 60_u64 * 24_u64 * 14_u64; // 2 weeks

    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(bank_f, min_withdraw_period)
        .await?;

    // Deposit through deposit ix should update admin shares
    let source_account = bank_f
        .mint
        .create_token_account_and_mint_to(deposit_amount_ui)
        .await;

    bank_f
        .try_admin_deposit_insurance(&source_account, deposit_amount)
        .await
        .unwrap();

    let first_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| tf.calculate_epoch_fee(0, deposit_amount).unwrap_or(0))
        .unwrap_or(0);
    let postfee_deposit_amount = deposit_amount - first_transfer_fee;

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48::from(postfee_deposit_amount));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, postfee_deposit_amount);
    assert_eq!(source_account.balance().await, 0);

    // Liquidation occurs
    liquidation(&test_f, &bank_mint).await.unwrap();

    // Second transfer fee
    let second_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_epoch_fee(0, postfee_deposit_amount)
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let pre_liq_withdraw_amount = postfee_deposit_amount - second_transfer_fee;

    // Withdraw
    bank_f
        .try_admin_withdraw_insurance(&source_account, admin_shares)
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
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 1);
    assert!(source_account.balance().await > pre_liq_withdraw_amount);

    Ok(())
}

#[test_case(BankMint::Usdc)]
#[test_case(BankMint::Sol)]
#[test_case(BankMint::PyUSD)]
#[test_case(BankMint::T22WithFee)]
#[tokio::test]
async fn marginfi_liquid_insurance_fund_admin_deposit_withdraw_with_bad_debt_success(
    bank_mint: BankMint,
) -> anyhow::Result<()> {
    // first create bank
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let bank_f = test_f.get_bank(&bank_mint);

    let deposit_amount_ui = 100_f64;
    let deposit_amount: u64 =
        (deposit_amount_ui as u64) * 10_u64.pow(bank_f.mint.mint.decimals as u32);

    let min_withdraw_period = 60_u64 * 60_u64 * 24_u64 * 14_u64; // 2 weeks

    let mut lif_f = test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(bank_f, min_withdraw_period)
        .await?;

    // Deposit through deposit ix should update admin shares
    let source_account = bank_f
        .mint
        .create_token_account_and_mint_to(deposit_amount_ui)
        .await;

    bank_f
        .try_admin_deposit_insurance(&source_account, deposit_amount)
        .await
        .unwrap();

    let first_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| tf.calculate_epoch_fee(0, deposit_amount).unwrap_or(0))
        .unwrap_or(0);
    let postfee_deposit_amount = deposit_amount - first_transfer_fee;

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48::from(postfee_deposit_amount));

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, postfee_deposit_amount);
    assert_eq!(source_account.balance().await, 0);

    // Bad Debt handling occurs
    bad_debt(&test_f, bank_f).await?;

    // Withdraw
    bank_f
        .try_admin_withdraw_insurance(&source_account, postfee_deposit_amount.into())
        .await
        .unwrap();

    // Check shares
    let lif = lif_f.load().await;
    let admin_shares = lif.get_admin_shares();
    let total_shares = lif.get_total_shares();
    assert_eq!(total_shares, admin_shares);
    assert_eq!(total_shares, I80F48!(0.0));

    let second_transfer_fee = bank_f
        .mint
        .load_state()
        .await
        .get_extension::<TransferFeeConfig>()
        .map(|tf| {
            tf.calculate_epoch_fee(0, postfee_deposit_amount)
                .unwrap_or(0)
        })
        .unwrap_or(0);
    // In the absence of bad debt
    let postfee_withdraw_amount = deposit_amount - first_transfer_fee - second_transfer_fee;

    // Check balance
    let insurance_vault = TokenAccountFixture::fetch(
        test_f.context.clone(),
        bank_f.get_vault(BankVaultType::Insurance).0,
    )
    .await;
    assert_eq!(insurance_vault.balance().await, 0);
    assert!(dbg!(source_account.balance().await) < postfee_withdraw_amount);

    Ok(())
}

// TODO:
// [x] users deposit/withdraw success
// [x] admin cannot withdraw more than their share value (when user deposits are present)
// [x] users cannot withdraw more than their share value (when other user deposits or admin deposits are present)
// [x] with liquidation
// [x] with bad debt

async fn liquidation(test_f: &TestFixture, bank_mint: &BankMint) -> anyhow::Result<()> {
    let bank_f = test_f.get_bank(&bank_mint);
    let sol_eq = test_f.get_bank(&BankMint::SolEquivalent);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account = bank_f.mint.create_token_account_and_mint_to(3_000).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account.key, bank_f, 2_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol_eq = sol_eq.mint.create_token_account_and_mint_to(200).await;
    let borrower_token_account_bank = bank_f.mint.create_token_account_and_mint_to(0).await;

    // Borrower deposits 100 SOL worth of $1000
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol_eq.key, sol_eq, 100)
        .await?;

    // Borrower borrows 20 from bank_f
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_bank.key, bank_f, 20)
        .await?;
    let borrower_account = borrower_mfi_account_f.load().await;
    println!(
        "borrower balances: {:?}",
        borrower_account.lending_account.balances
    );

    // Synthetically bring down the borrower account health by reducing the asset weights of the SOL bank
    sol_eq
        .update_config(BankConfigOpt {
            asset_weight_init: Some(I80F48!(0.000025).into()),
            asset_weight_maint: Some(I80F48!(0.00005).into()),
            ..Default::default()
        })
        .await?;

    lender_mfi_account_f
        .try_liquidate(&borrower_mfi_account_f, sol_eq, 1, bank_f)
        .await
        .unwrap();

    Ok(())
}

async fn bad_debt(test_f: &TestFixture, bank_f: &BankFixture) -> anyhow::Result<()> {
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account = bank_f.mint.create_token_account_and_mint_to(200_000).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account.key, bank_f, 100_000)
        .await?;

    let borrower_account = test_f.create_marginfi_account().await;
    let borrower_deposit_account = test_f
        .sol_equivalent_mint
        .create_token_account_and_mint_to(2_000)
        .await;

    borrower_account
        .try_bank_deposit(
            borrower_deposit_account.key,
            test_f.get_bank(&BankMint::SolEquivalent),
            1_000,
        )
        .await?;

    let borrower_borrow_account = bank_f.mint.create_token_account_and_mint_to(0).await;

    borrower_account
        .try_bank_borrow(borrower_borrow_account.key, bank_f, 1_000)
        .await?;

    // Artificially bankrupt user
    let mut borrower_mfi_account = borrower_account.load().await;
    borrower_mfi_account.lending_account.balances[0]
        .asset_shares
        .value = 0_i128.to_le_bytes();
    borrower_account.set_account(&borrower_mfi_account).await?;

    test_f
        .marginfi_group
        .try_handle_bankruptcy(bank_f, &borrower_account)
        .await
        .unwrap();

    Ok(())
}
