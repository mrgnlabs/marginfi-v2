use fixed::types::I80F48;
use fixtures::{assert_custom_error, prelude::*};
use marginfi::{
    prelude::MarginfiError,
    state::marginfi_group::{Bank, BankConfig, BankConfigOpt},
};
use pretty_assertions::assert_eq;
use solana_program_test::*;
use test_case::test_case;

#[tokio::test]
async fn add_bank_success() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let mints = vec![
        (
            MintFixture::new(test_f.context.clone(), None, None).await,
            *DEFAULT_USDC_TEST_BANK_CONFIG,
        ),
        (
            MintFixture::new_token_22(
                test_f.context.clone(),
                None,
                None,
                &[SupportedExtension::TransferFee],
            )
            .await,
            *DEFAULT_T22_WITH_FEE_TEST_BANK_CONFIG,
        ),
        (
            MintFixture::new_from_file(&test_f.context.clone(), "src/fixtures/pyUSD.json"),
            *DEFAULT_PYUSD_TEST_BANK_CONFIG,
        ),
    ];

    for (mint_f, bank_config) in mints {
        let res = test_f
            .marginfi_group
            .try_lending_pool_add_bank(&mint_f, bank_config)
            .await;
        assert!(res.is_ok());

        // Check bank is active
        let bank = res.unwrap();
        let bank = test_f.try_load(&bank.key).await?;
        assert!(bank.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn add_bank_with_seed_success() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let mints = vec![
        (
            MintFixture::new(test_f.context.clone(), None, None).await,
            *DEFAULT_USDC_TEST_BANK_CONFIG,
        ),
        (
            MintFixture::new_token_22(
                test_f.context.clone(),
                None,
                None,
                &[SupportedExtension::TransferFee],
            )
            .await,
            *DEFAULT_T22_WITH_FEE_TEST_BANK_CONFIG,
        ),
        (
            MintFixture::new_from_file(&test_f.context.clone(), "src/fixtures/pyUSD.json"),
            *DEFAULT_PYUSD_TEST_BANK_CONFIG,
        ),
    ];

    for (mint_f, bank_config) in mints {
        let bank_seed = 1200_u64;

        let res = test_f
            .marginfi_group
            .try_lending_pool_add_bank_with_seed(&mint_f, bank_config, bank_seed)
            .await;
        assert!(res.is_ok());

        // Check bank is active
        let bank = res.unwrap();
        let bank = test_f.try_load(&bank.key).await?;
        assert!(bank.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn marginfi_group_add_bank_failure_inexistent_pyth_feed() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let bank_asset_mint_fixture = MintFixture::new(test_f.context.clone(), None, None).await;

    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(
            &bank_asset_mint_fixture,
            BankConfig {
                oracle_setup: marginfi::state::price::OracleSetup::PythEma,
                oracle_keys: create_oracle_key_array(INEXISTENT_PYTH_USDC_FEED),
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::InvalidOracleAccount);

    Ok(())
}

#[test_case(BankMint::Usdc)]
#[test_case(BankMint::PyUSD)]
#[test_case(BankMint::T22WithFee)]
#[tokio::test]
async fn configure_bank_success(bank_mint: BankMint) -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let bank = test_f.get_bank(&bank_mint);

    let res = bank
        .update_config(BankConfigOpt {
            interest_rate_config: Some(marginfi::state::marginfi_group::InterestRateConfigOpt {
                optimal_utilization_rate: Some(I80F48::from_num(0.91).into()),
                plateau_interest_rate: Some(I80F48::from_num(0.44).into()),
                max_interest_rate: Some(I80F48::from_num(1.44).into()),
                insurance_fee_fixed_apr: Some(I80F48::from_num(0.13).into()),
                insurance_ir_fee: Some(I80F48::from_num(0.11).into()),
                protocol_fixed_fee_apr: Some(I80F48::from_num(0.51).into()),
                protocol_ir_fee: Some(I80F48::from_num(0.011).into()),
            }),
            ..BankConfigOpt::default()
        })
        .await;
    assert!(res.is_ok());

    // Load bank and check each property in config matches

    let bank: Bank = test_f.load_and_deserialize(&bank.key).await;

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.optimal_utilization_rate),
        I80F48::from_num(0.91)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.plateau_interest_rate),
        I80F48::from_num(0.44)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.max_interest_rate),
        I80F48::from_num(1.44)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_fee_fixed_apr),
        I80F48::from_num(0.13)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_ir_fee),
        I80F48::from_num(0.11)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_fixed_fee_apr),
        I80F48::from_num(0.51)
    );

    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_ir_fee),
        I80F48::from_num(0.011)
    );

    Ok(())
}
