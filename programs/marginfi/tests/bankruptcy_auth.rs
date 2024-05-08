use fixed_macro::types::I80F48;
use fixtures::{
    assert_custom_error,
    test::{BankMint, TestBankSetting, TestFixture, TestSettings, DEFAULT_SOL_TEST_BANK_CONFIG},
};
use marginfi::{
    errors::MarginfiError,
    state::marginfi_group::{BankConfig, BankConfigOpt, BankVaultType, GroupConfig},
};
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_unauthorized() -> anyhow::Result<()> {
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
        .value = 0_i128.to_le_bytes();
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
        .try_update(GroupConfig {
            admin: Some(Pubkey::new_unique()),
        })
        .await?;

    let bank = test_f.get_bank(&BankMint::USDC);

    let res = test_f
        .marginfi_group
        .try_handle_bankruptcy(bank, &borrower_account)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::Unauthorized);

    Ok(())
}

#[tokio::test]
async fn marginfi_group_handle_bankruptcy_perimssionless() -> anyhow::Result<()> {
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
        .value = 0_i128.to_le_bytes();
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

    let bank = test_f.get_bank(&BankMint::USDC);

    bank.update_config(BankConfigOpt {
        permissionless_bad_debt_settlement: Some(true),
        ..Default::default()
    })
    .await?;

    test_f
        .marginfi_group
        .try_update(GroupConfig {
            admin: Some(Pubkey::new_unique()),
        })
        .await?;

    let res = test_f
        .marginfi_group
        .try_handle_bankruptcy(bank, &borrower_account)
        .await;

    assert!(res.is_ok());

    Ok(())
}
