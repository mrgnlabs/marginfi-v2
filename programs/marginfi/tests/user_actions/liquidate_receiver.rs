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
    let usdc_bank = test_f.get_bank(&BankMint::Usdc);
    let user_token_account = test_f.usdc_mint.create_token_account_and_mint_to(100).await;
    user.try_bank_deposit(user_token_account.key, usdc_bank, 100, None)
        .await?;
    let payer = test_f.context.borrow().payer.pubkey();

    let (record_pk, _bump) = Pubkey::find_program_address(
        &[user.key.as_ref(), LIQUIDATION_RECORD_SEED.as_bytes()],
        &marginfi::ID,
    );

    let init_ix = user.make_init_liquidation_record_ix(record_pk, payer).await;
    let start_ix = user.make_start_liquidation_ix(record_pk, payer).await;
    let end_ix = user
        .make_end_liquidation_ix(
            record_pk,
            payer,
            test_f.marginfi_group.fee_state,
            test_f.marginfi_group.fee_wallet,
        )
        .await;

    let ctx = test_f.context.borrow_mut();
    let init_tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(init_tx).await?;

    // Liquidation on a healthy account should fail
    let start_tx = Transaction::new_signed_with_payer(
        &[start_ix, end_ix],
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

    // liquidator setup doesn't really matter here, but we demonstrate that you cannot do any ix
    // before start_liquidate, even something innocuous as here with deposit.
    let liquidator = test_f.create_marginfi_account().await;
    let liquidatee = test_f.create_marginfi_account().await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let usdc_bank = test_f.get_bank(&BankMint::Usdc);

    // A pointless deposit to the liquidator...
    let liq_token_account = test_f.usdc_mint.create_token_account_and_mint_to(100).await;
    liquidator
        .try_bank_deposit(liq_token_account.key, usdc_bank, 99.0, None)
        .await?;

    // Set up an unhealthy liquidatee...
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
                asset_weight_init: Some(I80F48!(0.001).into()),
                asset_weight_maint: Some(I80F48!(0.002).into()),
                ..Default::default()
            },
            None,
        )
        .await?;

    let (record_pk, _bump) = Pubkey::find_program_address(
        &[liquidatee.key.as_ref(), LIQUIDATION_RECORD_SEED.as_bytes()],
        &marginfi::ID,
    );

    let init_ix = liquidatee
        .make_init_liquidation_record_ix(record_pk, test_f.payer())
        .await;
    // Sneaky Sneaky...
    let deposit_ix = liquidator
        .make_bank_deposit_ix(liq_token_account.key, usdc_bank, 1.0, None)
        .await;
    let start_ix = liquidatee
        .make_start_liquidation_ix(record_pk, liquidator.key)
        .await;
    let end_ix = liquidatee
        .make_end_liquidation_ix(
            record_pk,
            liquidator.key,
            test_f.marginfi_group.fee_state,
            test_f.marginfi_group.fee_wallet,
        )
        .await;

    let ctx = test_f.context.borrow_mut();
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
    // TODO repeat but with compute ix as the first to show that compute IS ALLOWED
}

#[tokio::test]
async fn liquidate_receiver_happy_path() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let liquidator = test_f.create_marginfi_account().await;
    let liquidatee = test_f.create_marginfi_account().await;

    // Note: Sol is $10, USDC is $10
    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let usdc_bank = test_f.get_bank(&BankMint::Usdc);

    // liquidator provides initial liquidity and keeps some for repayment
    let liquidator_usdc_acc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    liquidator
        .try_bank_deposit(liquidator_usdc_acc.key, usdc_bank, 100, None)
        .await?;

    // setup liquidatee (after bank has liquidity for them to borrow)
    let user_token_sol = test_f.sol_mint.create_token_account_and_mint_to(10).await;
    let user_token_usdc = test_f.usdc_mint.create_empty_token_account().await;
    // * Note: Deposited $20 in SOL, borrowed $10 in USDC
    // * Note: all asset/liab weights in testing are 1, e.g. $20 in SOL = $20 exactly in value
    liquidatee
        .try_bank_deposit(user_token_sol.key, sol_bank, 2.0, None)
        .await?;
    liquidatee
        .try_bank_borrow(user_token_usdc.key, usdc_bank, 10.0)
        .await?;

    // make liquidatee unhealthy ($20 of SOL now worth $8)
    sol_bank
        .update_config(
            BankConfigOpt {
                asset_weight_init: Some(I80F48!(0.25).into()),
                asset_weight_maint: Some(I80F48!(0.4).into()),
                ..Default::default()
            },
            None,
        )
        .await?;

    let (record_pk, _bump) = Pubkey::find_program_address(
        &[liquidatee.key.as_ref(), LIQUIDATION_RECORD_SEED.as_bytes()],
        &marginfi::ID,
    );

    {
        let ctx = test_f.context.borrow_mut();
        let init_ix = liquidatee
            .make_init_liquidation_record_ix(record_pk, ctx.payer.pubkey())
            .await;
        let init_tx = Transaction::new_signed_with_payer(
            &[init_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(init_tx).await?;
    } // release borrow of test_f via ctx

    let payer = test_f.payer().clone();
    println!("liquidator key {:?}", liquidator.key);
    println!("payer key {:?}", payer);
    let start_ix = liquidatee.make_start_liquidation_ix(record_pk, payer).await;
    // withdraw some sol to the liquidator and repay some usdc
    let liquidator_sol_acc = test_f.sol_mint.create_empty_token_account().await;
    // Seize .105 * 20 = $2.1
    let withdraw_ix = liquidatee
        .make_bank_withdraw_ix(liquidator_sol_acc.key, sol_bank, 0.105, None)
        .await;
    // Repay $2
    let repay_ix = liquidatee
        .make_bank_repay_ix(liquidator_usdc_acc.key, usdc_bank, 2.0, None)
        .await;
    let end_ix = liquidatee
        .make_end_liquidation_ix(
            record_pk,
            payer, // Note: payer must sign to pay the sol fee
            test_f.marginfi_group.fee_state,
            test_f.marginfi_group.fee_wallet,
        )
        .await;

    {
        let ctx = test_f.context.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[start_ix, withdraw_ix, repay_ix, end_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    } // release borrow of test_f via ctx

    // TODO assert taking more than 5% fails

    // TODO assert non-profitable possible?

    let liquidatee_ma = liquidatee.load().await;
    // receivership ends at the end of the tx, we never see the flag enabled
    assert_eq!(liquidatee_ma.get_flag(ACCOUNT_IN_RECEIVERSHIP), false);

    // TODO assert balances changed

    // TODO assert fee debited

    Ok(())
}
