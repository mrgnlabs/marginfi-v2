use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use fixtures::{assert_custom_error, assert_eq_noise, native, prelude::*};
use marginfi::state::bank::BankImpl;
use marginfi::{
    constants::LIQUIDATION_FLAT_FEE_DEFAULT, prelude::*,
    state::marginfi_account::MarginfiAccountImpl,
};
use marginfi_type_crate::{
    constants::LIQUIDATION_RECORD_SEED,
    types::{BankConfigOpt, ACCOUNT_IN_RECEIVERSHIP},
};
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signer::Signer, transaction::Transaction};

#[tokio::test]
async fn deleverage_happy_path() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let risk_admin = test_f.payer().clone();
    assert_eq!(risk_admin, test_f.marginfi_group.load().await.risk_admin);

    let lp = test_f.create_marginfi_account().await;
    let deleveragee = test_f.create_marginfi_account().await;

    // Note: Sol is $10, USDC is $1
    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let usdc_bank = test_f.get_bank(&BankMint::Usdc);

    // LP provides initial liquidity
    let lp_usdc_acc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    lp.try_bank_deposit(lp_usdc_acc.key, usdc_bank, 100, None)
        .await?;

    // Setup deleveragee (after bank has liquidity for them to borrow)
    let user_token_sol = test_f.sol_mint.create_token_account_and_mint_to(10).await;
    let user_token_usdc = test_f.usdc_mint.create_empty_token_account().await;

    // * Note: Deposited $20 in SOL, borrowed $10 in USDC
    // * Note: all asset/liab weights in testing are 1, e.g. $20 in SOL = $20 exactly in value
    deleveragee
        .try_bank_deposit(user_token_sol.key, sol_bank, 2.0, None)
        .await?;
    deleveragee
        .try_bank_borrow(user_token_usdc.key, usdc_bank, 10.0)
        .await?;

    let (record_pk, _bump) = Pubkey::find_program_address(
        &[LIQUIDATION_RECORD_SEED.as_bytes(), deleveragee.key.as_ref()],
        &marginfi::ID,
    );

    // make deleveragee unhealthy ($20 of SOL now worth $8)
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


    // Note: deleveraging also (like liquidation) uses liquidation record - to ensure we do not worsen the account health.
    // What's differnt though is that the init ix CAN be part of the "deleverage" tx - no instrospection checks!
    let init_ix = deleveragee
        .make_init_liquidation_record_ix(record_pk, risk_admin)
        .await;

    let start_ix = deleveragee
        .make_start_deleverage_ix(record_pk, risk_admin)
        .await;

    // Risk admin withdraws some sol and repays some usdc
    let risk_admin_usdc_acc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    let risk_admin_sol_acc = test_f.sol_mint.create_empty_token_account().await;

    // Seize .210 * 10 = $2.10
    let withdraw_ix = deleveragee
        .make_bank_withdraw_ix(risk_admin_sol_acc.key, sol_bank, 0.210, None, true)
        .await;

    // Repay $2
    let repay_ix = deleveragee
        .make_bank_repay_ix(risk_admin_usdc_acc.key, usdc_bank, 2.0, None)
        .await;

    let end_ix = deleveragee
        .make_end_deleverage_ix(record_pk, risk_admin, vec![])
        .await;

    // record sol balances before liquidation
    let (payer_pre, fee_pre) = {
        let ctx = test_f.context.borrow_mut();
        let payer_bal = ctx.banks_client.get_balance(risk_admin).await?;
        let fee_bal = ctx
            .banks_client
            .get_balance(test_f.marginfi_group.fee_wallet)
            .await?;
        (payer_bal, fee_bal)
    };

    // send the tx
    {
        let ctx = test_f.context.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[init_ix, start_ix, withdraw_ix, repay_ix, end_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction_with_preflight(tx)
            .await?;
    } // release borrow of test_f via ctx

    let risk_admin_sol_tokens = risk_admin_sol_acc.balance().await;
    assert_eq!(risk_admin_sol_tokens, native!(0.210, "SOL", f64));
    let risk_admin_usdc_tokens = risk_admin_usdc_acc.balance().await;
    assert_eq!(risk_admin_usdc_tokens, native!(198, "USDC"));

    let deleveragee_ma = deleveragee.load().await;
    let sol_bank_state = sol_bank.load().await;
    let usdc_bank_state = usdc_bank.load().await;
    let sol_index = deleveragee_ma
        .lending_account
        .balances
        .iter()
        .position(|b| b.bank_pk == sol_bank.key)
        .unwrap();
    let usdc_index = deleveragee_ma
        .lending_account
        .balances
        .iter()
        .position(|b| b.bank_pk == usdc_bank.key)
        .unwrap();
    let sol_amount = sol_bank_state.get_asset_amount(
        deleveragee_ma.lending_account.balances[sol_index]
            .asset_shares
            .into(),
    )?;
    let usdc_liab = usdc_bank_state.get_liability_amount(
        deleveragee_ma.lending_account.balances[usdc_index]
            .liability_shares
            .into(),
    )?;
    // 20 - 2.10, in native sol decimals
    assert_eq_noise!(sol_amount, I80F48!(1790000000));
    // 10 - 2, in native usdc decimals
    assert_eq_noise!(usdc_liab, I80F48!(8000000));
    // receivership ends at the end of the tx, we never see the flag enabled
    assert_eq!(deleveragee_ma.get_flag(ACCOUNT_IN_RECEIVERSHIP), false);

    let (payer_post, fee_post) = {
        let ctx = test_f.context.borrow_mut();
        let payer_bal = ctx.banks_client.get_balance(risk_admin).await?;
        let fee_bal = ctx
            .banks_client
            .get_balance(test_f.marginfi_group.fee_wallet)
            .await?;
        (payer_bal, fee_bal)
    };

    // Note: no fees charged for deleveraging!
    assert_eq!(fee_pre, fee_post);

    // // make deleveragee unhealthy ($20 of SOL now worth $8)
    // sol_bank
    //     .update_config(
    //         BankConfigOpt {
    //             asset_weight_init: Some(I80F48!(0.25).into()),
    //             asset_weight_maint: Some(I80F48!(0.4).into()),
    //             ..Default::default()
    //         },
    //         None,
    //     )
    //     .await?;

    Ok(())
}

#[tokio::test]
async fn deleverage_cannot_worsen_health() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let risk_admin = test_f.payer().clone();
    assert_eq!(risk_admin, test_f.marginfi_group.load().await.risk_admin);

    let lp = test_f.create_marginfi_account().await;
    let deleveragee = test_f.create_marginfi_account().await;

    // Note: Sol is $10, USDC is $1
    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let usdc_bank = test_f.get_bank(&BankMint::Usdc);

    // LP provides initial liquidity
    let lp_usdc_acc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    lp.try_bank_deposit(lp_usdc_acc.key, usdc_bank, 100, None)
        .await?;

    // Setup deleveragee (after bank has liquidity for them to borrow)
    let user_token_sol = test_f.sol_mint.create_token_account_and_mint_to(10).await;
    let user_token_usdc = test_f.usdc_mint.create_empty_token_account().await;

    // * Note: Deposited $20 in SOL, borrowed $10 in USDC
    // * Note: all asset/liab weights in testing are 1, e.g. $20 in SOL = $20 exactly in value
    deleveragee
        .try_bank_deposit(user_token_sol.key, sol_bank, 2.0, None)
        .await?;
    deleveragee
        .try_bank_borrow(user_token_usdc.key, usdc_bank, 10.0)
        .await?;

    let (record_pk, _bump) = Pubkey::find_program_address(
        &[LIQUIDATION_RECORD_SEED.as_bytes(), deleveragee.key.as_ref()],
        &marginfi::ID,
    );

    // Note: deleveraging also (like liquidation) uses liquidation record - to ensure we do not worsen the account health.
    // What's differnt though is that the init ix CAN be part of the "deleverage" tx - no instrospection checks!
    let init_ix = deleveragee
        .make_init_liquidation_record_ix(record_pk, risk_admin)
        .await;

    let start_ix = deleveragee
        .make_start_deleverage_ix(record_pk, risk_admin)
        .await;

    // Risk admin withdraws some sol and repays some usdc
    let risk_admin_usdc_acc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
    let risk_admin_sol_acc = test_f.sol_mint.create_empty_token_account().await;

    // Seize .210 * 10 = $2.10
    let withdraw_ix = deleveragee
        .make_bank_withdraw_ix(risk_admin_sol_acc.key, sol_bank, 0.210, None, true)
        .await;

    // Repay $2
    let repay_ix = deleveragee
        .make_bank_repay_ix(risk_admin_usdc_acc.key, usdc_bank, 2.0, None)
        .await;

    let end_ix = deleveragee
        .make_end_deleverage_ix(record_pk, risk_admin, vec![])
        .await;

        let ctx = test_f.context.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[init_ix, start_ix, withdraw_ix, repay_ix, end_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );
        
    let res = ctx
        .banks_client
        .process_transaction_with_preflight(tx)
        .await;

    assert!(res.is_err());
    assert_custom_error!(res.unwrap_err(), MarginfiError::WorseHealthPostLiquidation);

    Ok(())
}

// // Here liquidator tries to seize more than the permitted premium, and should fail
// #[tokio::test]
// async fn deleverage_cannot_worsen_health() -> anyhow::Result<()> {
//     let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

//     let liquidator = test_f.create_marginfi_account().await;
//     let deleveragee = test_f.create_marginfi_account().await;
//     let sol_bank = test_f.get_bank(&BankMint::Sol);
//     let usdc_bank = test_f.get_bank(&BankMint::Usdc);

//     let liquidator_usdc_acc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
//     liquidator
//         .try_bank_deposit(liquidator_usdc_acc.key, usdc_bank, 100, None)
//         .await?;

//     let user_token_sol = test_f.sol_mint.create_token_account_and_mint_to(10).await;
//     let user_token_usdc = test_f.usdc_mint.create_empty_token_account().await;
//     deleveragee
//         .try_bank_deposit(user_token_sol.key, sol_bank, 2.0, None)
//         .await?;
//     deleveragee
//         .try_bank_borrow(user_token_usdc.key, usdc_bank, 10.0)
//         .await?;
//     sol_bank
//         .update_config(
//             BankConfigOpt {
//                 asset_weight_init: Some(I80F48!(0.25).into()),
//                 asset_weight_maint: Some(I80F48!(0.4).into()),
//                 ..Default::default()
//             },
//             None,
//         )
//         .await?;

//     let (record_pk, _bump) = Pubkey::find_program_address(
//         &[LIQUIDATION_RECORD_SEED.as_bytes(), deleveragee.key.as_ref()],
//         &marginfi::ID,
//     );
//     {
//         let ctx = test_f.context.borrow_mut();
//         let init_ix = deleveragee
//             .make_init_liquidation_record_ix(record_pk, ctx.payer.pubkey())
//             .await;
//         let init_tx = Transaction::new_signed_with_payer(
//             &[init_ix],
//             Some(&ctx.payer.pubkey()),
//             &[&ctx.payer],
//             ctx.last_blockhash,
//         );
//         ctx.banks_client
//             .process_transaction_with_preflight(init_tx)
//             .await?;
//     }

//     let payer = test_f.payer().clone();
//     let start_ix = deleveragee
//         .make_start_liquidation_ix(record_pk, payer)
//         .await;
//     let liquidator_sol_acc = test_f.sol_mint.create_empty_token_account().await;
//     // .3 * 10 = $3
//     let withdraw_ix = deleveragee
//         .make_bank_withdraw_ix(liquidator_sol_acc.key, sol_bank, 0.3, None, true)
//         .await;
//     // $2
//     let repay_ix = deleveragee
//         .make_bank_repay_ix(liquidator_usdc_acc.key, usdc_bank, 2.0, None)
//         .await;
//     let end_ix = deleveragee
//         .make_end_liquidation_ix(
//             record_pk,
//             payer,
//             test_f.marginfi_group.fee_state,
//             test_f.marginfi_group.fee_wallet,
//             vec![],
//         )
//         .await;

//     let ctx = test_f.context.borrow_mut();
//     let tx = Transaction::new_signed_with_payer(
//         &[start_ix, withdraw_ix, repay_ix, end_ix],
//         Some(&ctx.payer.pubkey()),
//         &[&ctx.payer],
//         ctx.last_blockhash,
//     );
//     let res = ctx
//         .banks_client
//         .process_transaction_with_preflight(tx)
//         .await;
//     assert!(res.is_err());
//     assert_custom_error!(res.unwrap_err(), MarginfiError::LiquidationPremiumTooHigh);
//     Ok(())
// }

// #[tokio::test]
// async fn liquidate_receiver_rejects_zero_weight_asset() -> anyhow::Result<()> {
//     let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

//     let liquidator = test_f.create_marginfi_account().await;
//     let deleveragee = test_f.create_marginfi_account().await;
//     let sol_bank = test_f.get_bank(&BankMint::Sol);
//     let usdc_bank = test_f.get_bank(&BankMint::Usdc);

//     let liquidator_usdc_acc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
//     liquidator
//         .try_bank_deposit(liquidator_usdc_acc.key, usdc_bank, 100, None)
//         .await?;

//     let user_token_sol = test_f.sol_mint.create_token_account_and_mint_to(10).await;
//     let user_token_usdc = test_f.usdc_mint.create_empty_token_account().await;
//     deleveragee
//         .try_bank_deposit(user_token_sol.key, sol_bank, 1.0, None)
//         .await?;
//     deleveragee
//         .try_bank_borrow(user_token_usdc.key, usdc_bank, 10.0)
//         .await?;

//     sol_bank
//         .update_config(
//             BankConfigOpt {
//                 asset_weight_init: Some(I80F48::ZERO.into()),
//                 asset_weight_maint: Some(I80F48::ZERO.into()),
//                 ..Default::default()
//             },
//             None,
//         )
//         .await?;

//     let (record_pk, _bump) = Pubkey::find_program_address(
//         &[LIQUIDATION_RECORD_SEED.as_bytes(), deleveragee.key.as_ref()],
//         &marginfi::ID,
//     );
//     {
//         let ctx = test_f.context.borrow_mut();
//         let init_ix = deleveragee
//             .make_init_liquidation_record_ix(record_pk, ctx.payer.pubkey())
//             .await;
//         let init_tx = Transaction::new_signed_with_payer(
//             &[init_ix],
//             Some(&ctx.payer.pubkey()),
//             &[&ctx.payer],
//             ctx.last_blockhash,
//         );
//         ctx.banks_client
//             .process_transaction_with_preflight(init_tx)
//             .await?;
//     }

//     let payer = test_f.payer().clone();
//     let start_ix = deleveragee
//         .make_start_liquidation_ix(record_pk, payer)
//         .await;
//     let liquidator_sol_acc = test_f.sol_mint.create_empty_token_account().await;
//     let withdraw_ix = deleveragee
//         .make_bank_withdraw_ix(liquidator_sol_acc.key, sol_bank, 0.1, None, true)
//         .await;
//     let repay_ix = deleveragee
//         .make_bank_repay_ix(liquidator_usdc_acc.key, usdc_bank, 2.0, None)
//         .await;
//     let end_ix = deleveragee
//         .make_end_liquidation_ix(
//             record_pk,
//             payer,
//             test_f.marginfi_group.fee_state,
//             test_f.marginfi_group.fee_wallet,
//             vec![],
//         )
//         .await;

//     let ctx = test_f.context.borrow_mut();
//     let tx = Transaction::new_signed_with_payer(
//         &[start_ix, withdraw_ix, repay_ix, end_ix],
//         Some(&ctx.payer.pubkey()),
//         &[&ctx.payer],
//         ctx.last_blockhash,
//     );
//     let res = ctx
//         .banks_client
//         .process_transaction_with_preflight(tx)
//         .await;
//     assert!(res.is_err());
//     assert_custom_error!(res.unwrap_err(), MarginfiError::LiquidationPremiumTooHigh);

//     Ok(())
// }

// // Here liquidator can zero-out the account because it falls below the minimum value threshold
// #[tokio::test]
// async fn liquidate_receiver_closes_out_low_value_acc() -> anyhow::Result<()> {
//     let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

//     let liquidator = test_f.create_marginfi_account().await;
//     let deleveragee = test_f.create_marginfi_account().await;
//     let sol_bank = test_f.get_bank(&BankMint::Sol);
//     let usdc_bank = test_f.get_bank(&BankMint::Usdc);

//     let liquidator_usdc_acc = test_f.usdc_mint.create_token_account_and_mint_to(200).await;
//     liquidator
//         .try_bank_deposit(liquidator_usdc_acc.key, usdc_bank, 100, None)
//         .await?;

//     let user_token_sol = test_f.sol_mint.create_token_account_and_mint_to(10).await;
//     let user_token_usdc = test_f.usdc_mint.create_empty_token_account().await;
//     //  .4 * 10 = $4, which is less than the minimum of $5
//     deleveragee
//         .try_bank_deposit(user_token_sol.key, sol_bank, 0.4, None)
//         .await?;
//     deleveragee
//         .try_bank_borrow(user_token_usdc.key, usdc_bank, 2.0)
//         .await?;
//     sol_bank
//         .update_config(
//             BankConfigOpt {
//                 asset_weight_init: Some(I80F48!(0.25).into()),
//                 asset_weight_maint: Some(I80F48!(0.4).into()),
//                 ..Default::default()
//             },
//             None,
//         )
//         .await?;

//     let (record_pk, _bump) = Pubkey::find_program_address(
//         &[LIQUIDATION_RECORD_SEED.as_bytes(), deleveragee.key.as_ref()],
//         &marginfi::ID,
//     );
//     {
//         let ctx = test_f.context.borrow_mut();
//         let init_ix = deleveragee
//             .make_init_liquidation_record_ix(record_pk, ctx.payer.pubkey())
//             .await;
//         let init_tx = Transaction::new_signed_with_payer(
//             &[init_ix],
//             Some(&ctx.payer.pubkey()),
//             &[&ctx.payer],
//             ctx.last_blockhash,
//         );
//         ctx.banks_client
//             .process_transaction_with_preflight(init_tx)
//             .await?;
//     }

//     let payer = test_f.payer().clone();
//     let start_ix = deleveragee
//         .make_start_liquidation_ix(record_pk, payer)
//         .await;
//     let liquidator_sol_acc = test_f.sol_mint.create_empty_token_account().await;
//     // NOTE: In receivership liquidation, you MUST PASS the oracle for the withdrawn asset even for
//     // a withdraw-all. The entire balance is still withdrawn!
//     let withdraw_ix = deleveragee
//         .make_bank_withdraw_ix(liquidator_sol_acc.key, sol_bank, 0.4, Some(true), true)
//         .await;
//     // The entire liability
//     let repay_ix = deleveragee
//         .make_bank_repay_ix(liquidator_usdc_acc.key, usdc_bank, 2.0, Some(true))
//         .await;
//     let end_ix = deleveragee
//         .make_end_liquidation_ix(
//             record_pk,
//             payer,
//             test_f.marginfi_group.fee_state,
//             test_f.marginfi_group.fee_wallet,
//             vec![usdc_bank.key],
//         )
//         .await;

//     {
//         let ctx = test_f.context.borrow_mut();
//         let tx = Transaction::new_signed_with_payer(
//             &[start_ix, withdraw_ix, repay_ix, end_ix],
//             Some(&ctx.payer.pubkey()),
//             &[&ctx.payer],
//             ctx.last_blockhash,
//         );
//         let res = ctx
//             .banks_client
//             .process_transaction_with_preflight(tx)
//             .await;
//         assert!(res.is_ok());
//     } // release borrow of ctx

//     // Account has been fully closed, all positions were seized and repaid.
//     let marginfi_account = deleveragee.load().await;
//     let active_balance_count = marginfi_account
//         .lending_account
//         .get_active_balances_iter()
//         .count();
//     // The lending position is closed.
//     assert_eq!(0, active_balance_count);

//     Ok(())
// }
