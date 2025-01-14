use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{assert_custom_error, prelude::*};
use marginfi::{
    constants::{
        FREEZE_SETTINGS, INIT_BANK_ORIGINATION_FEE_DEFAULT, PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG,
    },
    prelude::MarginfiError,
    state::{
        bank::{Bank, BankConfig, BankConfigOpt},
        marginfi_group::BankVaultType,
    },
};
use pretty_assertions::assert_eq;
use solana_program_test::*;
use solana_sdk::pubkey::Pubkey;
use test_case::test_case;

#[tokio::test]
async fn add_bank_success() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let fee_wallet = test_f.marginfi_group.fee_wallet;

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
        // Load the fee state before the start of the test
        let fee_balance_before: u64;
        {
            let mut ctx = test_f.context.borrow_mut();
            fee_balance_before = ctx
                .banks_client
                .get_account(fee_wallet)
                .await
                .unwrap()
                .unwrap()
                .lamports;
        }

        let res = test_f
            .marginfi_group
            .try_lending_pool_add_bank(&mint_f, bank_config)
            .await;

        // Check bank
        let bank_f = res.unwrap();
        let Bank {
            mint,
            mint_decimals,
            group,
            asset_share_value,
            liability_share_value,
            liquidity_vault,
            liquidity_vault_bump,
            liquidity_vault_authority_bump,
            insurance_vault,
            insurance_vault_bump,
            insurance_vault_authority_bump,
            collected_insurance_fees_outstanding,
            fee_vault,
            fee_vault_bump,
            fee_vault_authority_bump,
            collected_group_fees_outstanding,
            total_liability_shares,
            total_asset_shares,
            last_update,
            config,
            flags,
            emissions_rate,
            emissions_remaining,
            emissions_mint,
            collected_program_fees_outstanding,
            _padding_0,
            _padding_1,
            .. // ignore internal padding
        } = bank_f.load().await;
        #[rustfmt::skip]
        let _ = {
            assert_eq!(mint, bank_f.mint.key);
            assert_eq!(mint_decimals, bank_f.mint.load_state().await.base.decimals);
            assert_eq!(group, test_f.marginfi_group.key);
            assert_eq!(asset_share_value, I80F48!(1.0).into());
            assert_eq!(liability_share_value, I80F48!(1.0).into());
            assert_eq!(liquidity_vault, bank_f.get_vault(BankVaultType::Liquidity).0);
            assert_eq!(liquidity_vault_bump, bank_f.get_vault(BankVaultType::Liquidity).1);
            assert_eq!(liquidity_vault_authority_bump, bank_f.get_vault_authority(BankVaultType::Liquidity).1);
            assert_eq!(insurance_vault, bank_f.get_vault(BankVaultType::Insurance).0);
            assert_eq!(insurance_vault_bump, bank_f.get_vault(BankVaultType::Insurance).1);
            assert_eq!(insurance_vault_authority_bump, bank_f.get_vault_authority(BankVaultType::Insurance).1);
            assert_eq!(fee_vault, bank_f.get_vault(BankVaultType::Fee).0);
            assert_eq!(fee_vault_bump, bank_f.get_vault(BankVaultType::Fee).1);
            assert_eq!(fee_vault_authority_bump, bank_f.get_vault_authority(BankVaultType::Fee).1);
            assert_eq!(collected_insurance_fees_outstanding, I80F48!(0.0).into());
            assert_eq!(collected_group_fees_outstanding, I80F48!(0.0).into());
            assert_eq!(total_liability_shares, I80F48!(0.0).into());
            assert_eq!(total_asset_shares, I80F48!(0.0).into());
            assert_eq!(config, bank_config);
            assert_eq!(flags, 0);
            assert_eq!(emissions_rate, 0);
            assert_eq!(emissions_mint, Pubkey::new_from_array([0; 32]));
            assert_eq!(emissions_remaining, I80F48!(0.0).into());
            assert_eq!(collected_program_fees_outstanding, I80F48!(0.0).into());

            assert_eq!(_padding_0, <[[u64; 2]; 27] as Default>::default());
            assert_eq!(_padding_1, <[[u64; 2]; 32] as Default>::default());

            // this is the only loosely checked field
            assert!(last_update >= 0 && last_update <= 5);
        };

        // Load the fee state after the test
        let fee_balance_after: u64;
        {
            let mut ctx = test_f.context.borrow_mut();
            fee_balance_after = ctx
                .banks_client
                .get_account(fee_wallet)
                .await
                .unwrap()
                .unwrap()
                .lamports;
        }
        let expected_fee_delta = INIT_BANK_ORIGINATION_FEE_DEFAULT as u64;
        let actual_fee_delta = fee_balance_after - fee_balance_before;
        assert_eq!(expected_fee_delta, actual_fee_delta);
    }

    Ok(())
}

#[tokio::test]
async fn add_bank_with_seed_success() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(None).await;

    let fee_wallet = test_f.marginfi_group.fee_wallet;

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
        let fee_balance_before: u64;
        {
            let mut ctx = test_f.context.borrow_mut();
            fee_balance_before = ctx
                .banks_client
                .get_account(fee_wallet)
                .await
                .unwrap()
                .unwrap()
                .lamports;
        }

        let bank_seed = 1200_u64;

        let res = test_f
            .marginfi_group
            .try_lending_pool_add_bank_with_seed(&mint_f, bank_config, bank_seed)
            .await;
        assert!(res.is_ok());

        // Check bank
        let bank_f = res.unwrap();
        let Bank {
            mint,
            mint_decimals,
            group,
            asset_share_value,
            liability_share_value,
            liquidity_vault,
            liquidity_vault_bump,
            liquidity_vault_authority_bump,
            insurance_vault,
            insurance_vault_bump,
            insurance_vault_authority_bump,
            collected_insurance_fees_outstanding,
            fee_vault,
            fee_vault_bump,
            fee_vault_authority_bump,
            collected_group_fees_outstanding,
            total_liability_shares,
            total_asset_shares,
            last_update,
            config,
            flags,
            emissions_rate,
            emissions_remaining,
            emissions_mint,
            collected_program_fees_outstanding,
            _padding_0,
            _padding_1,
            .. // ignore internal padding
        } = bank_f.load().await;
        #[rustfmt::skip]
        let _ = {
            assert_eq!(mint, bank_f.mint.key);
            assert_eq!(mint_decimals, bank_f.mint.load_state().await.base.decimals);
            assert_eq!(group, test_f.marginfi_group.key);
            assert_eq!(asset_share_value, I80F48!(1.0).into());
            assert_eq!(liability_share_value, I80F48!(1.0).into());
            assert_eq!(liquidity_vault, bank_f.get_vault(BankVaultType::Liquidity).0);
            assert_eq!(liquidity_vault_bump, bank_f.get_vault(BankVaultType::Liquidity).1);
            assert_eq!(liquidity_vault_authority_bump, bank_f.get_vault_authority(BankVaultType::Liquidity).1);
            assert_eq!(insurance_vault, bank_f.get_vault(BankVaultType::Insurance).0);
            assert_eq!(insurance_vault_bump, bank_f.get_vault(BankVaultType::Insurance).1);
            assert_eq!(insurance_vault_authority_bump, bank_f.get_vault_authority(BankVaultType::Insurance).1);
            assert_eq!(fee_vault, bank_f.get_vault(BankVaultType::Fee).0);
            assert_eq!(fee_vault_bump, bank_f.get_vault(BankVaultType::Fee).1);
            assert_eq!(fee_vault_authority_bump, bank_f.get_vault_authority(BankVaultType::Fee).1);
            assert_eq!(collected_insurance_fees_outstanding, I80F48!(0.0).into());
            assert_eq!(collected_group_fees_outstanding, I80F48!(0.0).into());
            assert_eq!(total_liability_shares, I80F48!(0.0).into());
            assert_eq!(total_asset_shares, I80F48!(0.0).into());
            assert_eq!(config, bank_config);
            assert_eq!(flags, 0);
            assert_eq!(emissions_rate, 0);
            assert_eq!(emissions_mint, Pubkey::new_from_array([0; 32]));
            assert_eq!(emissions_remaining, I80F48!(0.0).into());
            assert_eq!(collected_program_fees_outstanding, I80F48!(0.0).into());

            assert_eq!(_padding_0, <[[u64; 2]; 27] as Default>::default());
            assert_eq!(_padding_1, <[[u64; 2]; 32] as Default>::default());

            // this is the only loosely checked field
            assert!(last_update >= 0 && last_update <= 5);
        };

        let fee_balance_after: u64;
        {
            let mut ctx = test_f.context.borrow_mut();
            fee_balance_after = ctx
                .banks_client
                .get_account(fee_wallet)
                .await
                .unwrap()
                .unwrap()
                .lamports;
        }
        let expected_fee_delta = INIT_BANK_ORIGINATION_FEE_DEFAULT as u64;
        let actual_fee_delta = fee_balance_after - fee_balance_before;
        assert_eq!(expected_fee_delta, actual_fee_delta);
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
                oracle_setup: marginfi::state::price::OracleSetup::PythLegacy,
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
#[test_case(BankMint::SolSwbPull)]
#[tokio::test]
async fn configure_bank_success(bank_mint: BankMint) -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let bank = test_f.get_bank(&bank_mint);
    let old_bank = bank.load().await;

    let config_bank_opt = BankConfigOpt {
        interest_rate_config: Some(marginfi::state::interest_rate::InterestRateConfigOpt {
            optimal_utilization_rate: Some(I80F48::from_num(0.91).into()),
            plateau_interest_rate: Some(I80F48::from_num(0.44).into()),
            max_interest_rate: Some(I80F48::from_num(1.44).into()),
            insurance_fee_fixed_apr: Some(I80F48::from_num(0.13).into()),
            insurance_ir_fee: Some(I80F48::from_num(0.11).into()),
            protocol_fixed_fee_apr: Some(I80F48::from_num(0.51).into()),
            protocol_ir_fee: Some(I80F48::from_num(0.011).into()),
            protocol_origination_fee: Some(I80F48::ZERO.into()),
        }),
        ..BankConfigOpt::default()
    };
    let res = bank.update_config(config_bank_opt.clone()).await;
    assert!(res.is_ok());

    // Load bank and check each property in config matches
    // Ensure bank didn't change any other fields. Only need to check the opt fields

    let bank: Bank = test_f.load_and_deserialize(&bank.key).await;
    let BankConfigOpt {
        interest_rate_config,
        asset_weight_init,
        asset_weight_maint,
        liability_weight_init,
        liability_weight_maint,
        deposit_limit,
        borrow_limit,
        operational_state,
        oracle,
        risk_tier,
        asset_tag,
        total_asset_value_init_limit,
        oracle_max_age,
        permissionless_bad_debt_settlement,
        freeze_settings,
    } = &config_bank_opt;
    // Compare bank field to opt field if Some, otherwise compare to old bank field
    macro_rules! check_bank_field {
        ($field:tt, $subfield:tt) => {
            assert_eq!(
                bank.config.$field.$subfield,
                $field
                    .as_ref()
                    .map(|opt| opt
                        .$subfield
                        .clone()
                        .unwrap_or(old_bank.config.$field.$subfield))
                    .unwrap()
            );
        };

        ($field:tt) => {
            assert_eq!(bank.config.$field, $field.unwrap_or(old_bank.config.$field));
        };
    }

    #[rustfmt::skip]
    let _ = {
        check_bank_field!(interest_rate_config, optimal_utilization_rate);
        check_bank_field!(interest_rate_config, plateau_interest_rate);
        check_bank_field!(interest_rate_config, max_interest_rate);
        check_bank_field!(interest_rate_config, insurance_fee_fixed_apr);
        check_bank_field!(interest_rate_config, insurance_ir_fee);
        check_bank_field!(interest_rate_config, protocol_fixed_fee_apr);
        check_bank_field!(interest_rate_config, protocol_ir_fee);
        check_bank_field!(interest_rate_config, protocol_origination_fee);

        check_bank_field!(asset_weight_init);
        check_bank_field!(asset_weight_maint);
        check_bank_field!(liability_weight_init);
        check_bank_field!(liability_weight_maint);
        check_bank_field!(deposit_limit);
        check_bank_field!(borrow_limit);
        check_bank_field!(operational_state);
        check_bank_field!(risk_tier);
        check_bank_field!(asset_tag);
        check_bank_field!(total_asset_value_init_limit);
        check_bank_field!(oracle_max_age);



        assert!(permissionless_bad_debt_settlement
            // If Some(...) check flag set properly
            .map(|set| set == bank.get_flag(PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG))
            // If None check flag is unchanged
            .unwrap_or( bank.get_flag(PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG) == old_bank.get_flag(PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG))
        );

        assert!(freeze_settings
            // If Some(...) check flag set properly
            .map(|set| set == bank.get_flag(FREEZE_SETTINGS))
            // If None check flag is unchanged
            .unwrap_or( bank.get_flag(FREEZE_SETTINGS) == old_bank.get_flag(FREEZE_SETTINGS))
        );

        assert_eq!(
            bank.config.oracle_keys,
            // If Some(...) check keys set properly
            // If None check keys unchanged
            oracle.map(|o| o.keys).unwrap_or(old_bank.config.oracle_keys));
        assert_eq!(
            bank.config.oracle_setup,
            // If Some(...) check setup set properly
            // If None check setup unchanged
            oracle.map(|o| o.setup).unwrap_or(old_bank.config.oracle_setup)
        );
    };

    Ok(())
}
