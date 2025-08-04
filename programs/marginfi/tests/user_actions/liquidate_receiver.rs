use fixed_macro::types::I80F48;
use fixtures::{assert_custom_error, prelude::*};
use marginfi::{prelude::*, state::marginfi_account::MarginfiAccountImpl};
use marginfi_type_crate::{
    constants::LIQUIDATION_RECORD_SEED,
    types::{BankConfigOpt, ACCOUNT_IN_RECEIVERSHIP},
};
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signer::Signer, transaction::Transaction};

#[tokio::test]
async fn liquidate_start_fails_on_healthy_account() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let user = test_f.create_marginfi_account().await;

    let (record_pk, _bump) = Pubkey::find_program_address(
        &[user.key.as_ref(), LIQUIDATION_RECORD_SEED.as_bytes()],
        &marginfi::ID,
    );

    let init_ix = user.make_init_liquidation_record_ix(record_pk).await;
    let start_ix = user
        .make_start_liquidation_ix(record_pk, test_f.context.borrow().payer.pubkey())
        .await;

    let mut ctx = test_f.context.borrow_mut();
    // init the record first so Start can be sent alone
    let init_tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(init_tx).await?;

    // Start on a healthy account should fail
    let start_tx = Transaction::new_signed_with_payer(
        &[start_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    let res = ctx.banks_client.process_transaction(start_tx).await;
    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::HealthyAccount);
    Ok(())
}

#[tokio::test]
async fn liquidate_start_must_be_first() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let liquidator = test_f.create_marginfi_account().await;
    let liquidatee = test_f.create_marginfi_account().await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let usdc_bank = test_f.get_bank(&BankMint::Usdc);

    let liq_token_account = test_f.usdc_mint.create_token_account_and_mint_to(100).await;
    // leave 1 token for the later deposit in the failing tx
    liquidator
        .try_bank_deposit(liq_token_account.key, usdc_bank, 99.0, None)
        .await?;

    let user_token_sol = test_f.sol_mint.create_token_account_and_mint_to(1).await;
    liquidatee
        .try_bank_deposit(user_token_sol.key, sol_bank, 1.0, None)
        .await?;
    let user_token_usdc = test_f.usdc_mint.create_empty_token_account().await;
    liquidatee
        .try_bank_borrow(user_token_usdc.key, usdc_bank, 1.0)
        .await?;

    sol_bank
        .update_config(
            BankConfigOpt {
                asset_weight_init: Some(I80F48!(0.25).into()),
                asset_weight_maint: Some(I80F48!(0.5).into()),
                ..Default::default()
            },
            None,
        )
        .await?;

    let (record_pk, _bump) = Pubkey::find_program_address(
        &[liquidatee.key.as_ref(), LIQUIDATION_RECORD_SEED.as_bytes()],
        &marginfi::ID,
    );

    let init_ix = liquidatee.make_init_liquidation_record_ix(record_pk).await;
    let deposit_ix = liquidator
        .make_bank_deposit_ix(liq_token_account.key, usdc_bank, 1.0, None)
        .await;
    let start_ix = liquidatee
        .make_start_liquidation_ix(record_pk, test_f.context.borrow().payer.pubkey())
        .await;
    let end_ix = liquidatee
        .make_end_liquidation_ix(
            record_pk,
            test_f.context.borrow().payer.pubkey(),
            test_f.marginfi_group.fee_state,
            test_f.marginfi_group.fee_wallet,
        )
        .await;

    let mut ctx = test_f.context.borrow_mut();
    // init in a separate tx to isolate the StartNotFirst failure
    let init_tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(init_tx).await?;

    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix, start_ix, end_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;
    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::StartNotFirst);
    Ok(())
}

#[tokio::test]
async fn liquidate_receiver_happy_path() -> anyhow::Result<()> {
    let mut test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let liquidator = test_f.create_marginfi_account().await;
    let liquidatee = test_f.create_marginfi_account().await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let usdc_bank = test_f.get_bank(&BankMint::Usdc);

    // liquidator provides initial liquidity and keeps some for repayment
    let liq_usdc_account = test_f.usdc_mint.create_token_account_and_mint_to(7).await;
    liquidator
        .try_bank_deposit(liq_usdc_account.key, usdc_bank, 6.0, None)
        .await?;

    // setup liquidatee (after bank has liquidity)
    let user_token_sol = test_f.sol_mint.create_token_account_and_mint_to(10).await;
    let user_token_usdc = test_f.usdc_mint.create_empty_token_account().await;
    liquidatee
        .try_bank_deposit(user_token_sol.key, sol_bank, 10.0, None)
        .await?;
    liquidatee
        .try_bank_borrow(user_token_usdc.key, usdc_bank, 5.0)
        .await?;

    // make account unhealthy
    sol_bank
        .update_config(
            BankConfigOpt {
                asset_weight_init: Some(I80F48!(0.25).into()),
                asset_weight_maint: Some(I80F48!(0.5).into()),
                ..Default::default()
            },
            None,
        )
        .await?;

    let (record_pk, _bump) = Pubkey::find_program_address(
        &[liquidatee.key.as_ref(), LIQUIDATION_RECORD_SEED.as_bytes()],
        &marginfi::ID,
    );

    let init_ix = liquidatee.make_init_liquidation_record_ix(record_pk).await;
    let start_ix = liquidatee
        .make_start_liquidation_ix(record_pk, test_f.context.borrow().payer.pubkey())
        .await;

    // withdraw some sol to the liquidator and repay some usdc
    let liq_sol_account = test_f.sol_mint.create_empty_token_account().await;
    let withdraw_ix = liquidatee
        .make_bank_withdraw_ix(liq_sol_account.key, sol_bank, 1.0, None)
        .await;
    let repay_ix = liquidator
        .make_bank_repay_ix(liq_usdc_account.key, usdc_bank, 1.0, None)
        .await;
    let end_ix = liquidatee
        .make_end_liquidation_ix(
            record_pk,
            test_f.context.borrow().payer.pubkey(),
            test_f.marginfi_group.fee_state,
            test_f.marginfi_group.fee_wallet,
        )
        .await;

    let mut ctx = test_f.context.borrow_mut();
    // initialize record separately so Start is first
    let init_tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(init_tx).await?;

    let tx = Transaction::new_signed_with_payer(
        &[start_ix, withdraw_ix, repay_ix, end_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    let liquidatee_ma = liquidatee.load().await;
    assert_eq!(liquidatee_ma.get_flag(ACCOUNT_IN_RECEIVERSHIP), false);

    Ok(())
}
