use fixtures::{assert_custom_error, prelude::*};
use marginfi::{
    constants::TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE, prelude::MarginfiError,
    state::bank::BankConfigOpt,
};
use pretty_assertions::assert_eq;
use solana_program_test::*;

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
