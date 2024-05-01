use fixtures::{
    spl::MintFixture,
    test::{TestFixture, DEFAULT_USDC_TEST_BANK_CONFIG},
};
use marginfi::{
    bank_authority_seed, bank_seed, instructions::InitMintParams,
    state::marginfi_group::BankVaultType,
};
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;

#[tokio::test]
async fn marginfi_create_liquid_insurance_fund_success() -> anyhow::Result<()> {
    // first create bank
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

    let bank_fixture = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&bank_asset_mint_fixture, *DEFAULT_USDC_TEST_BANK_CONFIG)
        .await?;

    let init_mint_params = InitMintParams {
        name: "InsuranceUSDC".to_string(),
        symbol: "IUSDC".to_string(),
        uri: "ipfs://".to_string(),
        decimals: 9,
    };

    let min_withdraw_period = 60_u64 * 60_u64 * 24_u64 * 14_u64; // 2 weeks

    test_f
        .marginfi_group
        .try_create_liquid_insurance_fund(
            &bank_asset_mint_fixture,
            init_mint_params,
            min_withdraw_period,
        )
        .await?;

    Ok(())
}
