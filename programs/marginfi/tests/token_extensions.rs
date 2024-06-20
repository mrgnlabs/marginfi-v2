use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::spl::SupportedExtension;
use fixtures::test::{
    BankMint, TestBankSetting, TestFixture, TestSettings, DEFAULT_SOL_TEST_BANK_CONFIG,
};
use fixtures::{assert_eq_noise, native};
use marginfi::state::marginfi_group::{
    Bank, BankConfig, BankConfigOpt, BankVaultType, GroupConfig,
};
use solana_program_test::tokio;
use test_case::test_case;

#[test_case(vec![])]
#[test_case(vec![SupportedExtension::TransferHook])]
#[test_case(vec![SupportedExtension::PermanentDelegate])]
#[test_case(vec![SupportedExtension::InterestBearing])]
#[test_case(vec![SupportedExtension::MintCloseAuthority])]
#[test_case(vec![SupportedExtension::PermanentDelegate, SupportedExtension::InterestBearing])]
#[test_case(vec![SupportedExtension::MintCloseAuthority, SupportedExtension::InterestBearing])]
#[test_case(vec![SupportedExtension::PermanentDelegate,SupportedExtension::MintCloseAuthority])]
#[test_case(vec![SupportedExtension::InterestBearing, SupportedExtension::MintCloseAuthority])]
#[test_case(vec![SupportedExtension::PermanentDelegate, SupportedExtension::InterestBearing, SupportedExtension::MintCloseAuthority])]
#[tokio::test]
async fn marginfi_account_liquidation_success_with_extension(
    extensions: Vec<SupportedExtension>,
) -> anyhow::Result<()> {
    let test_f = TestFixture::new_with_t22_extension(
        Some(TestSettings {
            banks: vec![
                TestBankSetting {
                    mint: BankMint::USDC,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::USDCToken22,
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
        }),
        extensions,
    )
    .await;

    let usdc_t22_bank_f = test_f.get_bank(&BankMint::USDCToken22);
    let sol_bank_f = test_f.get_bank(&BankMint::SOL);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_t22_mint
        .create_token_account_and_mint_to(2_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_t22_bank_f, 2_000)
        .await
        .unwrap();

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f.sol_mint.create_token_account_and_mint_to(100).await;
    let borrower_token_account_usdc = test_f
        .usdc_t22_mint
        .create_token_account_and_mint_to(0)
        .await;

    // Borrower deposits 100 SOL worth of $1000
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 100)
        .await?;

    // Borrower borrows $999
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_t22_bank_f, 999)
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
        .try_liquidate(&borrower_mfi_account_f, sol_bank_f, 1, usdc_t22_bank_f)
        .await?;

    // Checks
    let sol_bank: Bank = sol_bank_f.load().await;
    let usdc_bank: Bank = usdc_t22_bank_f.load().await;

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
    let insurance_fund_usdc = usdc_t22_bank_f
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq_noise!(
        insurance_fund_usdc.balance().await as i64,
        native!(0.25, "USDC", f64) as i64,
        1
    );

    Ok(())
}