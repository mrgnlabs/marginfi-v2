use fixtures::{
    assert_custom_error,
    spl::TokenAccountFixture,
    test::{BankMint, TestBankSetting, TestFixture, TestSettings},
};
use marginfi::errors::MarginfiError;
use solana_program_test::tokio;

#[tokio::test]
async fn close_marginfi_account() -> anyhow::Result<()> {
    let mut test_f: TestFixture =
        TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let marginfi_account_f = test_f.create_marginfi_account().await;

    let owner = test_f.payer();
    let token_account_f =
        TokenAccountFixture::new(test_f.context.clone(), &test_f.usdc_mint.key, &owner).await;
    test_f.usdc_mint.mint_to(&token_account_f.key, 1_000).await;

    let usdc_bank_f = test_f.get_bank(&BankMint::USDC);

    marginfi_account_f
        .try_bank_deposit(token_account_f.key, usdc_bank_f, 1_000)
        .await?;

    let res = marginfi_account_f.try_close_account(0).await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalAction);

    marginfi_account_f
        .try_bank_withdraw(token_account_f.key, usdc_bank_f, 1_000, Some(true))
        .await?;

    let res = marginfi_account_f.try_close_account(1).await;

    assert!(res.is_ok());

    Ok(())
}
