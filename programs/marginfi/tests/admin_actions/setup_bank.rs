use anchor_lang::error::ErrorCode;
use anchor_lang::{InstructionData, ToAccountMetas};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{assert_anchor_error, assert_custom_error, prelude::*};
use marginfi::{
    constants::INIT_BANK_ORIGINATION_FEE_DEFAULT,
    prelude::MarginfiError,
    state::{
        bank::{BankImpl, BankVaultType},
        marginfi_group::MarginfiGroupImpl,
    },
};
use marginfi_type_crate::{
    constants::{CLOSE_ENABLED_FLAG, FREEZE_SETTINGS, PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG},
    types::{
        make_points, Bank, BankCache, BankConfig, BankConfigOpt, EmodeEntry, InterestRateConfigOpt,
        MarginfiGroup, OracleSetup, RatePoint, EMODE_ON, INTEREST_CURVE_SEVEN_POINT,
    },
};
use pretty_assertions::assert_eq;
use solana_program_test::*;
use solana_sdk::{
    clock::Clock, instruction::Instruction, pubkey::Pubkey, transaction::Transaction,
};
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
            MintFixture::new(test_f.context.clone(), None, None).await,
            *DEFAULT_SOL_TEST_BANK_CONFIG,
        ),
        (
            MintFixture::new_from_file(&test_f.context.clone(), "src/fixtures/pyUSD.json"),
            *DEFAULT_PYUSD_TEST_BANK_CONFIG,
        ),
    ];

    let marginfi_group: MarginfiGroup = test_f
        .load_and_deserialize(&test_f.marginfi_group.key)
        .await;
    let mut last_update = marginfi_group.fee_state_cache.last_update;
    assert_eq!(last_update, 0);

    for (mint_f, bank_config) in mints {
        // This is just to test that the group's last_update field is properly updated upon bank creation
        {
            let ctx = test_f.context.borrow_mut();
            let mut clock: Clock = ctx.banks_client.get_sysvar().await?;
            // Advance clock by 1 sec
            clock.unix_timestamp += 1;
            ctx.set_sysvar(&clock);
        }

        // Load the fee state before the start of the test
        let fee_balance_before: u64;
        {
            let ctx = test_f.context.borrow_mut();
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

        let marginfi_group: MarginfiGroup = test_f
            .load_and_deserialize(&test_f.marginfi_group.key)
            .await;
        assert_eq!(marginfi_group.fee_state_cache.last_update, last_update + 1);
        last_update = marginfi_group.fee_state_cache.last_update;

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
            fees_destination_account,
            cache,
            lending_position_count,
            borrowing_position_count,
            _padding_0,
            kamino_reserve,
            kamino_obligation,
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
            assert_eq!(flags, CLOSE_ENABLED_FLAG);
            assert_eq!(emissions_rate, 0);
            assert_eq!(emissions_mint, Pubkey::new_from_array([0; 32]));
            assert_eq!(emissions_remaining, I80F48!(0.0).into());
            assert_eq!(collected_program_fees_outstanding, I80F48!(0.0).into());
            assert_eq!(fees_destination_account, Pubkey::default());
            assert_eq!(cache, BankCache::default());

            assert_eq!(lending_position_count, 0);
            assert_eq!(borrowing_position_count, 0);
            assert_eq!(_padding_0, <[u8; 16] as Default>::default());
            assert_eq!(kamino_reserve, Pubkey::default());
            assert_eq!(kamino_obligation, Pubkey::default());
            assert_eq!(_padding_1, <[[u64; 2]; 15] as Default>::default());

            // this is the only loosely checked field
            assert!(last_update >= 0 && last_update <= 5);
        };

        // Load the fee state after the test
        let fee_balance_after: u64;
        {
            let ctx = test_f.context.borrow_mut();
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
            MintFixture::new(test_f.context.clone(), None, None).await,
            *DEFAULT_SOL_TEST_BANK_CONFIG,
        ),
        (
            MintFixture::new_from_file(&test_f.context.clone(), "src/fixtures/pyUSD.json"),
            *DEFAULT_PYUSD_TEST_BANK_CONFIG,
        ),
    ];

    for (mint_f, bank_config) in mints {
        let fee_balance_before: u64;
        {
            let ctx = test_f.context.borrow_mut();
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
            fees_destination_account,
            cache,
            lending_position_count,
            borrowing_position_count,
            _padding_0,
            kamino_reserve,
            kamino_obligation,
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
            assert_eq!(flags, CLOSE_ENABLED_FLAG);
            assert_eq!(emissions_rate, 0);
            assert_eq!(emissions_mint, Pubkey::new_from_array([0; 32]));
            assert_eq!(emissions_remaining, I80F48!(0.0).into());
            assert_eq!(collected_program_fees_outstanding, I80F48!(0.0).into());
            assert_eq!(fees_destination_account, Pubkey::default());
            assert_eq!(cache, BankCache::default());

            assert_eq!(lending_position_count, 0);
            assert_eq!(borrowing_position_count, 0);
            assert_eq!(_padding_0, <[u8; 16] as Default>::default());
            assert_eq!(kamino_reserve, Pubkey::default());
            assert_eq!(kamino_obligation, Pubkey::default());
            assert_eq!(_padding_1, <[[u64; 2]; 15] as Default>::default());

            // this is the only loosely checked field
            assert!(last_update >= 0 && last_update <= 5);
        };

        let fee_balance_after: u64;
        {
            let ctx = test_f.context.borrow_mut();
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
                oracle_setup: OracleSetup::PythPushOracle,
                oracle_keys: create_oracle_key_array(INEXISTENT_PYTH_USDC_FEED),
                ..*DEFAULT_USDC_TEST_BANK_CONFIG
            },
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::PythPushWrongAccountOwner);

    Ok(())
}

#[tokio::test]
async fn configure_bank_to_fixed_oracle() -> anyhow::Result<()> {
    let test_settings = TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::Usdc,
            ..Default::default()
        }],
        ..Default::default()
    };
    let test_f = TestFixture::new(Some(test_settings)).await;

    let bank_f = test_f.get_bank(&BankMint::Usdc);
    let bank_before = bank_f.load().await;
    assert_ne!(bank_before.config.oracle_setup, OracleSetup::Fixed);

    let price_value = I80F48!(3.5);
    let price_wrapped = price_value.into();

    {
        let mut ctx = test_f.context.borrow_mut();
        let ix = Instruction {
            program_id: marginfi::ID,
            accounts: marginfi::accounts::LendingPoolSetFixedOraclePrice {
                group: test_f.marginfi_group.key,
                admin: ctx.payer.pubkey(),
                bank: bank_f.key,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingPoolSetFixedOraclePrice {
                price: price_wrapped,
            }
            .data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let bank_after = bank_f.load().await;
    assert_eq!(bank_after.config.oracle_setup, OracleSetup::Fixed);
    assert_eq!(I80F48::from(bank_after.config.fixed_price), price_value);
    assert_eq!(bank_after.config.oracle_keys[0], Pubkey::default());

    Ok(())
}

#[tokio::test]
async fn update_fixed_bank_price() -> anyhow::Result<()> {
    let test_settings = TestSettings {
        banks: vec![TestBankSetting {
            mint: BankMint::Fixed,
            ..Default::default()
        }],
        ..Default::default()
    };
    let test_f = TestFixture::new(Some(test_settings)).await;

    let bank_f = test_f.get_bank(&BankMint::Fixed);
    let bank_before = bank_f.load().await;
    assert_eq!(bank_before.config.oracle_setup, OracleSetup::Fixed);
    assert_eq!(I80F48::from(bank_before.config.fixed_price), I80F48!(2.0));

    let new_price_value = I80F48!(4.2);
    let new_price_wrapped = new_price_value.into();

    {
        let mut ctx = test_f.context.borrow_mut();
        let ix = Instruction {
            program_id: marginfi::ID,
            accounts: marginfi::accounts::LendingPoolSetFixedOraclePrice {
                group: test_f.marginfi_group.key,
                admin: ctx.payer.pubkey(),
                bank: bank_f.key,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingPoolSetFixedOraclePrice {
                price: new_price_wrapped,
            }
            .data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let bank_after = bank_f.load().await;
    assert_eq!(bank_after.config.oracle_setup, OracleSetup::Fixed);
    assert_eq!(I80F48::from(bank_after.config.fixed_price), new_price_value);

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
    let exp_points = make_points(&vec![
        RatePoint::new(1234, 56789),
        RatePoint::new(2345, 67890),
    ]);

    let config_bank_opt = BankConfigOpt {
        interest_rate_config: Some(InterestRateConfigOpt {
            insurance_fee_fixed_apr: Some(I80F48::from_num(0.13).into()),
            insurance_ir_fee: Some(I80F48::from_num(0.11).into()),
            protocol_fixed_fee_apr: Some(I80F48::from_num(0.51).into()),
            protocol_ir_fee: Some(I80F48::from_num(0.011).into()),
            protocol_origination_fee: Some(I80F48::ZERO.into()),
            zero_util_rate: Some(123),
            hundred_util_rate: Some(1234567),
            points: Some(exp_points),
        }),
        ..BankConfigOpt::default()
    };
    let res = bank.update_config(config_bank_opt.clone(), None).await;
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
        risk_tier,
        asset_tag,
        total_asset_value_init_limit,
        oracle_max_age,
        oracle_max_confidence,
        permissionless_bad_debt_settlement,
        freeze_settings,
    } = &config_bank_opt;
    // Compare bank field to opt field if Some, otherwise compare to old bank field
    macro_rules! check_bank_field {
        // Note: some nested fields (e.g. optimal_utilization_rate) don't exist on the config struct
        ($field:ident, $subfield:ident) => {
            assert_eq!(
                bank.config.$field.$subfield,
                $field
                    .as_ref()
                    .and_then(|opt| opt.$subfield.clone())
                    .unwrap_or(old_bank.config.$field.$subfield)
            );
        };

        // Top-level fields are always expected with the same name
        ($field:ident) => {
            assert_eq!(bank.config.$field, $field.unwrap_or(old_bank.config.$field));
        };
    }

    let _ = {
        check_bank_field!(interest_rate_config, insurance_fee_fixed_apr);
        check_bank_field!(interest_rate_config, insurance_ir_fee);
        check_bank_field!(interest_rate_config, protocol_fixed_fee_apr);
        check_bank_field!(interest_rate_config, protocol_ir_fee);
        check_bank_field!(interest_rate_config, protocol_origination_fee);
        check_bank_field!(interest_rate_config, zero_util_rate);
        check_bank_field!(interest_rate_config, hundred_util_rate);
        check_bank_field!(interest_rate_config, points);

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
        check_bank_field!(oracle_max_confidence);

        assert!(permissionless_bad_debt_settlement
            // If Some(...) check flag set properly
            .map(|set| set == bank.get_flag(PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG))
            // If None check flag is unchanged
            .unwrap_or(
                bank.get_flag(PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG)
                    == old_bank.get_flag(PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG)
            ));

        assert!(freeze_settings
            // If Some(...) check flag set properly
            .map(|set| set == bank.get_flag(FREEZE_SETTINGS))
            // If None check flag is unchanged
            .unwrap_or(bank.get_flag(FREEZE_SETTINGS) == old_bank.get_flag(FREEZE_SETTINGS)));

        // Oracles no longer update in the standard config instruction
        assert_eq!(
            bank.config.oracle_keys, old_bank.config.oracle_keys,
            "The config does not update oracles, try config_oracle"
        );
        assert_eq!(
            bank.config.oracle_setup, old_bank.config.oracle_setup,
            "The config does not update oracles, try config_oracle"
        );
    };

    Ok(())
}

#[tokio::test]
async fn add_too_many_arena_banks() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    let group_before = test_f.marginfi_group.load().await;

    let res = test_f
        .marginfi_group
        .try_update(
            group_before.admin,
            group_before.emode_admin,
            group_before.delegate_curve_admin,
            group_before.delegate_limit_admin,
            group_before.delegate_emissions_admin,
            true,
        )
        .await;
    assert!(res.is_ok());
    let group_after = test_f.marginfi_group.load().await;
    assert_eq!(group_after.is_arena_group(), true);

    // The first two banks/mints, which will succeed
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
    ];

    for (mint_f, bank_config) in mints {
        let res = test_f
            .marginfi_group
            .try_lending_pool_add_bank(&mint_f, bank_config)
            .await;
        assert!(res.is_ok());
    }

    // Adding a third bank fails
    let another_mint =
        MintFixture::new_from_file(&test_f.context.clone(), "src/fixtures/pyUSD.json");
    let another_config = *DEFAULT_PYUSD_TEST_BANK_CONFIG;

    let res = test_f
        .marginfi_group
        .try_lending_pool_add_bank(&another_mint, another_config)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::ArenaBankLimit);

    // Arena banks cannot be restored to non-arena

    let res = test_f
        .marginfi_group
        .try_update(
            group_before.admin,
            group_before.emode_admin,
            group_before.delegate_curve_admin,
            group_before.delegate_limit_admin,
            group_before.delegate_emissions_admin,
            false,
        )
        .await;
    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::ArenaSettingCannotChange);

    Ok(())
}

#[tokio::test]
async fn config_group_as_arena_too_many_banks() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    // Add three banks
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
    }

    let group_before = test_f.marginfi_group.load().await;
    let res = test_f
        .marginfi_group
        .try_update(
            group_before.admin,
            group_before.emode_admin,
            group_before.delegate_curve_admin,
            group_before.delegate_limit_admin,
            group_before.delegate_emissions_admin,
            true,
        )
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::ArenaBankLimit);

    Ok(())
}

#[tokio::test]
async fn config_group_admins() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;

    let new_admin = Pubkey::new_unique();
    let new_emode_admin = Pubkey::new_unique();
    let new_curve_admin = Pubkey::new_unique();
    let new_limit_admin = Pubkey::new_unique();
    let new_emissions_admin = Pubkey::new_unique();

    let res = test_f
        .marginfi_group
        .try_update(
            new_admin,
            new_emode_admin,
            new_curve_admin,
            new_limit_admin,
            new_emissions_admin,
            false,
        )
        .await;

    assert!(res.is_ok());
    let group_after = test_f.marginfi_group.load().await;
    assert_eq!(group_after.admin, new_admin);
    assert_eq!(group_after.emode_admin, new_emode_admin);
    assert_eq!(group_after.delegate_curve_admin, new_curve_admin);
    assert_eq!(group_after.delegate_limit_admin, new_limit_admin);
    assert_eq!(group_after.delegate_emissions_admin, new_emissions_admin);
    Ok(())
}

#[test_case(BankMint::Usdc)]
#[test_case(BankMint::PyUSD)]
#[test_case(BankMint::T22WithFee)]
#[test_case(BankMint::SolSwbPull)]
#[tokio::test]
async fn configure_bank_emode_success(bank_mint: BankMint) -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let bank = test_f.get_bank(&bank_mint);
    let old_bank = bank.load().await;

    assert_eq!(old_bank.emode.flags, 0u64);
    assert_eq!(old_bank.emode.emode_tag, 0u16);

    // First try to enable emode without any entries
    let empty_emode_tag = 1u16;

    let res = test_f
        .marginfi_group
        .try_lending_pool_configure_bank_emode(&bank, empty_emode_tag, &[])
        .await;
    assert!(res.is_ok());

    // Load bank and check that the emode settings got applied
    let loaded_bank: Bank = test_f.load_and_deserialize(&bank.key).await;
    let timestamp = {
        let ctx = test_f.context.borrow_mut();
        let clock: Clock = ctx.banks_client.get_sysvar().await?;
        clock.unix_timestamp
    };

    assert_eq!(loaded_bank.emode.flags, 0u64); // EMODE_ON is still not set because there are no entries
    assert_eq!(loaded_bank.emode.emode_tag, empty_emode_tag);
    assert_eq!(loaded_bank.emode.timestamp, timestamp);
    assert_eq!(old_bank.emode.emode_config, loaded_bank.emode.emode_config); // config stays the same
    assert_eq!(old_bank.config, loaded_bank.config); // everything else also stays the same

    // Now update the tag and add some entries
    let emode_tag = 2u16;
    let emode_entries = vec![EmodeEntry {
        collateral_bank_emode_tag: emode_tag, // sharing the same tag is allowed
        flags: 1,
        pad0: [0, 0, 0, 0, 0],
        asset_weight_init: loaded_bank.config.asset_weight_init,
        asset_weight_maint: loaded_bank.config.asset_weight_maint,
    }];

    let res = test_f
        .marginfi_group
        .try_lending_pool_configure_bank_emode(&bank, emode_tag, &emode_entries)
        .await;
    assert!(res.is_ok());

    // Load bank and check that the emode settings got applied
    let loaded_bank: Bank = test_f.load_and_deserialize(&bank.key).await;
    let timestamp = {
        let ctx = test_f.context.borrow_mut();
        let clock: Clock = ctx.banks_client.get_sysvar().await?;
        clock.unix_timestamp
    };

    assert_eq!(loaded_bank.emode.flags, EMODE_ON);
    assert_eq!(loaded_bank.emode.emode_tag, emode_tag);
    assert_eq!(loaded_bank.emode.timestamp, timestamp);
    // Due to sorting by tag, the newly added entry is the last one
    let last_entry_index = loaded_bank.emode.emode_config.entries.len() - 1;
    assert_eq!(
        loaded_bank.emode.emode_config.entries[last_entry_index],
        emode_entries[0]
    );
    // All other entries are still "inactive"
    for i in 0..last_entry_index {
        assert_eq!(
            loaded_bank.emode.emode_config.entries[i].collateral_bank_emode_tag,
            0
        );
    }

    Ok(())
}

#[test_case(BankMint::Usdc)]
#[test_case(BankMint::PyUSD)]
#[test_case(BankMint::T22WithFee)]
#[test_case(BankMint::SolSwbPull)]
#[tokio::test]
async fn configure_bank_emode_invalid_args(bank_mint: BankMint) -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let bank = test_f.get_bank(&bank_mint);

    // Try to set an emode config with invalid weight params -> should fail
    let emode_tag = 1u16;
    let emode_entries = vec![EmodeEntry {
        collateral_bank_emode_tag: emode_tag,
        flags: 1,
        pad0: [0, 0, 0, 0, 0],
        asset_weight_init: I80F48!(1.0).into(),
        asset_weight_maint: I80F48!(0.9).into(),
    }];

    let res = test_f
        .marginfi_group
        .try_lending_pool_configure_bank_emode(&bank, emode_tag, &emode_entries)
        .await;
    assert!(res.is_err());

    // Try to set an emode config with duplicate entries -> should fail
    let emode_tag = 2u16;
    let emode_entries = vec![
        EmodeEntry {
            collateral_bank_emode_tag: emode_tag,
            flags: 1,
            pad0: [0, 0, 0, 0, 0],
            asset_weight_init: I80F48!(0.9).into(),
            asset_weight_maint: I80F48!(1.0).into(),
        },
        EmodeEntry {
            collateral_bank_emode_tag: emode_tag,
            flags: 0,
            pad0: [0, 0, 0, 0, 0],
            asset_weight_init: I80F48!(0.5).into(),
            asset_weight_maint: I80F48!(0.9).into(),
        },
    ];

    let res = test_f
        .marginfi_group
        .try_lending_pool_configure_bank_emode(&bank, emode_tag, &emode_entries)
        .await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn configure_bank_interest_only_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let bank = test_f.get_bank(&BankMint::Usdc);
    let old_bank = bank.load().await;

    let exp_points = make_points(&vec![
        RatePoint::new(1234, 56789),
        RatePoint::new(2345, 67890),
    ]);

    let ir_config = InterestRateConfigOpt {
        // TODO deprecate in 1.7
        // optimal_utilization_rate: Some(I80F48::from_num(0.9).into()),
        // plateau_interest_rate: Some(I80F48::from_num(0.5).into()),
        // max_interest_rate: Some(I80F48::from_num(1.5).into()),
        insurance_fee_fixed_apr: Some(I80F48::from_num(0.01).into()),
        insurance_ir_fee: Some(I80F48::from_num(0.02).into()),
        protocol_fixed_fee_apr: Some(I80F48::from_num(0.03).into()),
        protocol_ir_fee: Some(I80F48::from_num(0.04).into()),
        protocol_origination_fee: Some(I80F48::from_num(0.05).into()),
        zero_util_rate: Some(123),
        hundred_util_rate: Some(1234567),
        points: Some(exp_points),
    };

    test_f
        .marginfi_group
        .try_lending_pool_configure_bank_interest_only(&bank, ir_config.clone())
        .await?;

    let bank_after: Bank = test_f.load_and_deserialize(&bank.key).await;

    // TODO deprecate in 1.7
    assert_eq!(
        bank_after
            .config
            .interest_rate_config
            .optimal_utilization_rate,
        I80F48::ZERO.into()
    );
    // TODO deprecate in 1.7
    assert_eq!(
        bank_after.config.interest_rate_config.plateau_interest_rate,
        I80F48::ZERO.into()
    );
    // TODO deprecate in 1.7
    assert_eq!(
        bank_after.config.interest_rate_config.max_interest_rate,
        I80F48::ZERO.into()
    );
    assert_eq!(
        bank_after
            .config
            .interest_rate_config
            .insurance_fee_fixed_apr,
        ir_config.insurance_fee_fixed_apr.unwrap()
    );
    assert_eq!(
        bank_after.config.interest_rate_config.insurance_ir_fee,
        ir_config.insurance_ir_fee.unwrap()
    );
    assert_eq!(
        bank_after
            .config
            .interest_rate_config
            .protocol_fixed_fee_apr,
        ir_config.protocol_fixed_fee_apr.unwrap()
    );
    assert_eq!(
        bank_after.config.interest_rate_config.protocol_ir_fee,
        ir_config.protocol_ir_fee.unwrap()
    );
    assert_eq!(
        bank_after
            .config
            .interest_rate_config
            .protocol_origination_fee,
        ir_config.protocol_origination_fee.unwrap()
    );
    assert_eq!(
        bank_after.config.interest_rate_config.zero_util_rate,
        ir_config.zero_util_rate.unwrap()
    );
    assert_eq!(
        bank_after.config.interest_rate_config.hundred_util_rate,
        ir_config.hundred_util_rate.unwrap()
    );
    assert_eq!(bank_after.config.interest_rate_config.points, exp_points);
    assert_eq!(
        bank_after.config.interest_rate_config.curve_type,
        INTEREST_CURVE_SEVEN_POINT
    );

    // No change
    assert_eq!(
        bank_after.config.deposit_limit,
        old_bank.config.deposit_limit
    );
    assert_eq!(bank_after.config.borrow_limit, old_bank.config.borrow_limit);
    assert_eq!(
        bank_after.config.total_asset_value_init_limit,
        old_bank.config.total_asset_value_init_limit
    );

    Ok(())
}

#[tokio::test]
async fn configure_bank_interest_only_not_admin() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let bank = test_f.get_bank(&BankMint::Usdc);
    let group_before = test_f.marginfi_group.load().await;
    test_f
        .marginfi_group
        .try_update(
            group_before.admin,
            group_before.emode_admin,
            Pubkey::new_unique(),
            group_before.delegate_limit_admin,
            group_before.delegate_emissions_admin,
            false,
        )
        .await?;

    let ir_config = InterestRateConfigOpt {
        hundred_util_rate: Some(1234567),
        ..Default::default()
    };

    let res = test_f
        .marginfi_group
        .try_lending_pool_configure_bank_interest_only(&bank, ir_config)
        .await;
    assert!(res.is_err());
    assert_anchor_error!(res.unwrap_err(), ErrorCode::ConstraintHasOne);

    Ok(())
}

#[tokio::test]
async fn configure_bank_limits_only_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let bank = test_f.get_bank(&BankMint::Usdc);
    let old_bank = bank.load().await;

    let new_deposit_limit = old_bank.config.deposit_limit + 100;
    let new_borrow_limit = old_bank.config.borrow_limit + 200;
    let new_tavl = old_bank.config.total_asset_value_init_limit + 50;

    test_f
        .marginfi_group
        .try_lending_pool_configure_bank_limits_only(
            &bank,
            Some(new_deposit_limit),
            Some(new_borrow_limit),
            Some(new_tavl),
        )
        .await?;

    let bank_after: Bank = test_f.load_and_deserialize(&bank.key).await;

    assert_eq!(bank_after.config.deposit_limit, new_deposit_limit);
    assert_eq!(bank_after.config.borrow_limit, new_borrow_limit);
    assert_eq!(bank_after.config.total_asset_value_init_limit, new_tavl);
    assert_eq!(
        bank_after.config.interest_rate_config,
        old_bank.config.interest_rate_config
    );

    Ok(())
}

#[tokio::test]
async fn configure_bank_limits_only_not_admin() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let bank = test_f.get_bank(&BankMint::Usdc);
    let group_before = test_f.marginfi_group.load().await;
    test_f
        .marginfi_group
        .try_update(
            group_before.admin,
            group_before.emode_admin,
            group_before.delegate_curve_admin,
            Pubkey::new_unique(),
            group_before.delegate_emissions_admin,
            false,
        )
        .await?;

    let res = test_f
        .marginfi_group
        .try_lending_pool_configure_bank_limits_only(&bank, Some(1), Some(1), Some(1))
        .await;
    assert!(res.is_err());
    assert_anchor_error!(res.unwrap_err(), ErrorCode::ConstraintHasOne);

    Ok(())
}
