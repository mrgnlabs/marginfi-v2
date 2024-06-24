use fixtures::test::{BankMint, TestFixture, TestSettings};
use marginfi::state::marginfi_group::GroupConfig;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;

#[tokio::test]
async fn marginfi_group_withdraw_fees_and_insurance() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    // Mint 1000 USDC to the insurance vault
    let bank_f = test_f.banks.get(&BankMint::Usdc).unwrap();
    let bank = bank_f.load().await;
    test_f.usdc_mint.mint_to(&bank.insurance_vault, 1000).await;

    // Create a receiving account and try to withdraw 1000 USDC from the insurance vault
    let receiving_account = test_f.usdc_mint.create_empty_token_account().await;
    bank_f
        .try_withdraw_insurance(&receiving_account, 1000)
        .await?;
    assert_eq!(receiving_account.balance().await, 1000); // Verifies that the receiving account balance is 1000 USDC

    // Mint 750 USDC to the fee vault
    test_f.usdc_mint.mint_to(&bank.fee_vault, 750).await;

    // Create a receiving account and try to withdraw 750 USDC from the fee vault
    let receiving_account = test_f.usdc_mint.create_empty_token_account().await;
    bank_f.try_withdraw_fees(&receiving_account, 750).await?;
    assert_eq!(receiving_account.balance().await, 750); // Verifies that the receiving account balance is 750 USDC

    // Update the admin of the marginfi group
    test_f
        .marginfi_group
        .try_update(GroupConfig {
            admin: Some(Pubkey::new_unique()),
        })
        .await?;

    // Mint 1000 USDC to the insurance vault
    test_f.usdc_mint.mint_to(&bank.insurance_vault, 1000).await;

    // Create a receiving account and try to withdraw 1000 USDC from the insurance vault
    let receiving_account = test_f.usdc_mint.create_empty_token_account().await;
    let res = bank_f
        .try_withdraw_insurance(&receiving_account, 1000)
        .await;
    assert!(res.is_err()); // Unable to withdraw 1000 USDC from the insurance vault, because the signer is not the admin

    // Mint 750 USDC to the fee vault
    test_f.usdc_mint.mint_to(&bank.fee_vault, 750).await;

    // Create a receiving account and try to withdraw 750 USDC from the fee vault
    let receiving_account = test_f.usdc_mint.create_empty_token_account().await;
    let res = bank_f.try_withdraw_fees(&receiving_account, 750).await;
    assert!(res.is_err()); // Unable to withdraw 750 USDC from the fee vault, because the signer is not the admin

    Ok(())
}
