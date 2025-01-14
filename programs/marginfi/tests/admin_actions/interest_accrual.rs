use anchor_lang::prelude::Clock;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{assert_eq_noise, native, prelude::*};
use marginfi::{
    prelude::GroupConfig,
    state::{
        bank::{Bank, BankConfig},
        interest_rate::InterestRateConfig,
        marginfi_group::BankVaultType,
    },
};
use pretty_assertions::assert_eq;
use solana_program_test::*;

#[tokio::test]
async fn marginfi_group_accrue_interest_rates_success_1() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        group_config: Some(GroupConfig { admin: None }),
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                config: Some(BankConfig {
                    interest_rate_config: InterestRateConfig {
                        optimal_utilization_rate: I80F48!(0.9).into(),
                        plateau_interest_rate: I80F48!(1).into(),
                        ..*DEFAULT_TEST_BANK_INTEREST_RATE_CONFIG
                    },
                    ..*DEFAULT_USDC_TEST_BANK_CONFIG
                }),
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        protocol_fees: false,
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f.usdc_mint.create_token_account_and_mint_to(100).await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 999)
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 90)
        .await?;

    {
        let mut ctx = test_f.context.borrow_mut();
        let mut clock: Clock = ctx.banks_client.get_sysvar().await?;
        // Advance clock by 1 year
        clock.unix_timestamp += 365 * 24 * 60 * 60;
        ctx.set_sysvar(&clock);
    }

    test_f
        .marginfi_group
        .try_accrue_interest(usdc_bank_f)
        .await?;

    let borrower_mfi_account = borrower_mfi_account_f.load().await;
    let borrower_bank_account = borrower_mfi_account.lending_account.balances[1];
    let usdc_bank: Bank = usdc_bank_f.load().await;
    let liabilities =
        usdc_bank.get_liability_amount(borrower_bank_account.liability_shares.into())?;

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let lender_bank_account = lender_mfi_account.lending_account.balances[0];
    let assets = usdc_bank.get_asset_amount(lender_bank_account.asset_shares.into())?;

    assert_eq_noise!(
        liabilities,
        I80F48::from(native!(180, "USDC")),
        I80F48!(100)
    );
    assert_eq_noise!(assets, I80F48::from(native!(190, "USDC")), I80F48!(100));

    Ok(())
}

#[tokio::test]
async fn marginfi_group_accrue_interest_rates_success_2() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                config: Some(BankConfig {
                    deposit_limit: native!(1_000_000_000, "USDC"),
                    interest_rate_config: InterestRateConfig {
                        optimal_utilization_rate: I80F48!(0.9).into(),
                        plateau_interest_rate: I80F48!(1).into(),
                        protocol_fixed_fee_apr: I80F48!(0.01).into(),
                        insurance_fee_fixed_apr: I80F48!(0.01).into(),
                        ..*DEFAULT_TEST_BANK_INTEREST_RATE_CONFIG
                    },
                    ..*DEFAULT_USDC_TEST_BANK_CONFIG
                }),
            },
            TestBankSetting {
                mint: BankMint::Sol,
                config: Some(BankConfig {
                    asset_weight_init: I80F48!(1).into(),
                    deposit_limit: native!(200_000_000, "SOL"),
                    ..*DEFAULT_SOL_TEST_BANK_CONFIG
                }),
            },
        ],
        group_config: Some(GroupConfig { admin: None }),
        protocol_fees: false,
    }))
    .await;

    let usdc_bank_f = test_f.get_bank(&BankMint::Usdc);
    let sol_bank_f = test_f.get_bank(&BankMint::Sol);

    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(100_000_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_usdc.key, usdc_bank_f, 100_000_000)
        .await?;

    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(10_000_000)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(borrower_token_account_sol.key, sol_bank_f, 10_000_000)
        .await?;
    let borrower_token_account_usdc = test_f.usdc_mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_borrow(borrower_token_account_usdc.key, usdc_bank_f, 90_000_000)
        .await?;

    // Advance clock by 1 minute
    {
        let mut ctx = test_f.context.borrow_mut();
        let mut clock: Clock = ctx.banks_client.get_sysvar().await?;
        clock.unix_timestamp += 60;
        ctx.set_sysvar(&clock);
    }

    test_f
        .marginfi_group
        .try_accrue_interest(usdc_bank_f)
        .await?;

    // The program fee ata needs to exist, but doesn't need any assets.
    {
        let ctx = test_f.context.clone();
        let ata = TokenAccountFixture::new_from_ata(
            ctx,
            &test_f.usdc_mint.key,
            &test_f.marginfi_group.fee_wallet,
            &test_f.usdc_mint.token_program,
        )
        .await;
        let ata_expected = get_associated_token_address_with_program_id(
            &test_f.marginfi_group.fee_wallet,
            &test_f.usdc_mint.key,
            &test_f.usdc_mint.token_program,
        );
        assert_eq!(ata.key, ata_expected);
    }

    test_f.marginfi_group.try_collect_fees(usdc_bank_f).await?;

    let borrower_mfi_account = borrower_mfi_account_f.load().await;
    let borrower_bank_account = borrower_mfi_account.lending_account.balances[1];
    let usdc_bank = usdc_bank_f.load().await;
    let liabilities =
        usdc_bank.get_liability_amount(borrower_bank_account.liability_shares.into())?;

    let lender_mfi_account = lender_mfi_account_f.load().await;
    let lender_bank_account = lender_mfi_account.lending_account.balances[0];
    let assets = usdc_bank.get_asset_amount(lender_bank_account.asset_shares.into())?;

    assert_eq_noise!(liabilities, I80F48!(90000174657530), I80F48!(10));
    assert_eq_noise!(assets, I80F48!(100000171232876), I80F48!(10));

    let protocol_fees = usdc_bank_f
        .get_vault_token_account(BankVaultType::Fee)
        .await;
    let insurance_fees = usdc_bank_f
        .get_vault_token_account(BankVaultType::Insurance)
        .await;

    assert_eq!(protocol_fees.balance().await, 1712328);
    assert_eq!(insurance_fees.balance().await, 1712328);

    Ok(())
}
