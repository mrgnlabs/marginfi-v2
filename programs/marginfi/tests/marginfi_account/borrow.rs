use fixtures::prelude::*;
use fixtures::{assert_custom_error, native};
use marginfi::prelude::*;
use marginfi::state::marginfi_group::BankConfigOpt;
use pretty_assertions::assert_eq;

use solana_program_test::*;

#[tokio::test]
async fn marginfi_account_borrow_failure_borrow_limit() -> anyhow::Result<()> {
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
