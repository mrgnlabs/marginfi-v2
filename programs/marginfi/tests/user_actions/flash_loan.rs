use anchor_lang::{InstructionData, ToAccountMetas};
use fixtures::{assert_custom_error, prelude::*};
use marginfi::{prelude::*, state::marginfi_account::FLASHLOAN_ENABLED_FLAG};
use pretty_assertions::assert_eq;
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_program_test::*;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, signer::keypair::Keypair, signer::Signer,
    system_instruction, transaction::Transaction,
};

// Flashloan tests
// 1.  Flashloan success (1 action)
// 2.  Flashloan success (3 actions)
// 3.  Flashloan fails because of bad account health
// 4.  Flashloan fails because of non whitelisted account
// 5.  Flashloan fails because of missing `end_flashloan` ix
// 6.  Flashloan fails because of invalid instructions sysvar
// 7.  Flashloan fails because of invalid `end_flashloan` ix order
// 8.  Flashloan fails because `end_flashloan` ix is for another account
// 9.  Flashloan fails because account is already in a flashloan
// 10. Flashloan fails because payer has insufficient balance for flashloan fee

#[tokio::test]
async fn flashloan_success_1op() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;

    // Get initial fee wallet balance
    let fee_state = test_f.get_fee_state().await;
    let fee_wallet = fee_state.global_fee_wallet;
    let initial_fee_wallet_balance = test_f.get_sol_balance(fee_wallet).await;

    // Borrow SOL
    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let repay_ix = borrower_mfi_account_f
        .make_bank_repay_ix(
            borrower_token_account_f_sol.key,
            sol_bank,
            1_000,
            Some(true),
        )
        .await;

    let flash_loan_result = borrower_mfi_account_f
        .try_flashloan(vec![borrow_ix, repay_ix], vec![], vec![])
        .await;

    assert!(flash_loan_result.is_ok());

    // Verify flashloan fee was paid
    assert_eq!(
        test_f.get_sol_balance(fee_wallet).await - initial_fee_wallet_balance,
        fee_state.flashloan_flat_sol_fee as u64,
    );

    Ok(())
}

#[tokio::test]
async fn flashloan_success_3op() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;

    // Get initial fee wallet balance
    let fee_state = test_f.get_fee_state().await;
    let fee_wallet = fee_state.global_fee_wallet;
    let initial_fee_wallet_balance = test_f.get_sol_balance(fee_wallet).await;

    // Create borrow and repay instructions
    let mut ixs = Vec::new();
    for _ in 0..3 {
        let borrow_ix = borrower_mfi_account_f
            .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
            .await;
        ixs.push(borrow_ix);

        let repay_ix = borrower_mfi_account_f
            .make_bank_repay_ix(
                borrower_token_account_f_sol.key,
                sol_bank,
                1_000,
                Some(true),
            )
            .await;
        ixs.push(repay_ix);
    }

    ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));

    let flash_loan_result = borrower_mfi_account_f
        .try_flashloan(ixs, vec![], vec![])
        .await;

    assert!(flash_loan_result.is_ok());

    // Verify flashloan fee was paid
    assert_eq!(
        test_f.get_sol_balance(fee_wallet).await - initial_fee_wallet_balance,
        fee_state.flashloan_flat_sol_fee as u64,
    );

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_account_health() -> anyhow::Result<()> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let flash_loan_result = borrower_mfi_account_f
        .try_flashloan(vec![borrow_ix], vec![], vec![sol_bank.key])
        .await;

    assert_custom_error!(
        flash_loan_result.unwrap_err(),
        MarginfiError::RiskEngineInitRejected
    );

    Ok(())
}

#[tokio::test]
// Note: The flashloan flag is now deprecated
async fn flashloan_ok_missing_flag() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let repay_ix = borrower_mfi_account_f
        .make_bank_repay_ix(
            borrower_token_account_f_sol.key,
            sol_bank,
            1_000,
            Some(true),
        )
        .await;

    let flash_loan_result = borrower_mfi_account_f
        .try_flashloan(vec![borrow_ix, repay_ix], vec![], vec![])
        .await;

    assert!(flash_loan_result.is_ok());

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_missing_fe_ix() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let repay_ix = borrower_mfi_account_f
        .make_bank_repay_ix(
            borrower_token_account_f_sol.key,
            sol_bank,
            1_000,
            Some(true),
        )
        .await;

    let mut ixs = vec![borrow_ix, repay_ix];

    let start_ix = borrower_mfi_account_f
        .make_lending_account_start_flashloan_ix(ixs.len() as u64)
        .await;

    ixs.insert(0, start_ix);

    let mut ctx = test_f.context.borrow_mut();

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey().clone()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalFlashloan);

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_missing_invalid_sysvar_ixs() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let repay_ix = borrower_mfi_account_f
        .make_bank_repay_ix(
            borrower_token_account_f_sol.key,
            sol_bank,
            1_000,
            Some(true),
        )
        .await;

    let mut ixs = vec![borrow_ix, repay_ix];

    let start_ix = Instruction {
        program_id: marginfi::id(),
        accounts: marginfi::accounts::LendingAccountStartFlashloan {
            marginfi_account: borrower_mfi_account_f.key,
            signer: test_f.context.borrow().payer.pubkey(),
            ixs_sysvar: Pubkey::default(),
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::LendingAccountStartFlashloan {
            end_index: ixs.len() as u64 + 1,
        }
        .data(),
    };

    let end_ix = borrower_mfi_account_f
        .make_lending_account_end_flashloan_ix(vec![], vec![])
        .await;

    ixs.insert(0, start_ix);
    ixs.push(end_ix);

    let mut ctx = test_f.context.borrow_mut();

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey().clone()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;

    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_invalid_end_fl_order() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let mut ixs = vec![borrow_ix];

    let start_ix = borrower_mfi_account_f
        .make_lending_account_start_flashloan_ix(0)
        .await;

    let end_ix = borrower_mfi_account_f
        .make_lending_account_end_flashloan_ix(vec![], vec![])
        .await;

    ixs.insert(0, start_ix);
    ixs.insert(0, end_ix);

    let mut ctx = test_f.context.borrow_mut();

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey().clone()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalFlashloan);

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_invalid_end_fl_different_m_account() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let mut ixs = vec![borrow_ix];

    let start_ix = borrower_mfi_account_f
        .make_lending_account_start_flashloan_ix(ixs.len() as u64 + 1)
        .await;

    let end_ix = lender_mfi_account_f
        .make_lending_account_end_flashloan_ix(vec![], vec![])
        .await;

    ixs.insert(0, start_ix);
    ixs.push(end_ix);

    let mut ctx = test_f.context.borrow_mut();

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey().clone()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalFlashloan);

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_already_in_flashloan() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;

    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Fund SOL borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;

    borrower_mfi_account_f
        .try_set_flag(FLASHLOAN_ENABLED_FLAG)
        .await?;

    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;
    // Borrow SOL

    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 1_000)
        .await;

    let mut ixs = vec![borrow_ix];

    let start_ix = borrower_mfi_account_f
        .make_lending_account_start_flashloan_ix(ixs.len() as u64 + 2)
        .await;

    let end_ix = borrower_mfi_account_f
        .make_lending_account_end_flashloan_ix(vec![], vec![])
        .await;

    ixs.insert(0, start_ix.clone());
    ixs.insert(0, start_ix.clone());
    ixs.push(end_ix);

    let mut ctx = test_f.context.borrow_mut();

    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&ctx.payer.pubkey().clone()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(tx).await;

    assert_custom_error!(res.unwrap_err(), MarginfiError::IllegalFlashloan);

    Ok(())
}

#[tokio::test]
async fn flashloan_fail_insufficient_balance_for_fee() -> anyhow::Result<()> {
    // Setup test executor with non-admin payer
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let sol_bank = test_f.get_bank(&BankMint::Sol);

    // Fund SOL lender
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account_f_sol = test_f
        .sol_mint
        .create_token_account_and_mint_to(1_000)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(lender_token_account_f_sol.key, sol_bank, 1_000)
        .await?;

    // Create borrower
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_token_account_f_sol = test_f.sol_mint.create_empty_token_account().await;

    // Increase the flashloan fee to be sure it causes the issue (and not the transaction fee)
    let payer = test_f.payer();
    let payer_keypair = test_f.payer_keypair();
    let fee_state = test_f.get_fee_state().await;
    let blockhash = test_f.get_latest_blockhash().await;
    let new_flashloan_fee = 10_000_000;

    let edit_fee_state_ix = Instruction {
        program_id: marginfi::id(),
        accounts: marginfi::accounts::EditFeeState {
            global_fee_admin: payer,
            fee_state: fee_state.key,
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::EditGlobalFeeState {
            fee_wallet: fee_state.global_fee_wallet,
            bank_init_flat_sol_fee: fee_state.bank_init_flat_sol_fee,
            flashloan_flat_sol_fee: new_flashloan_fee,
            program_fee_fixed: fee_state.program_fee_fixed,
            program_fee_rate: fee_state.program_fee_rate,
        }
        .data(),
    };
    {
        let mut ctx = test_f.context.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[edit_fee_state_ix],
            Some(&payer),
            &[&payer_keypair],
            blockhash,
        );
        ctx.banks_client.process_transaction(tx).await?;
    }

    let fee_state = test_f.get_fee_state().await;
    assert_eq!(fee_state.flashloan_flat_sol_fee, new_flashloan_fee);

    // Drain the payer to have enough for the transaction fee but not enough left for the flashloan fee
    let payer_keypair = test_f.payer_keypair();
    let current_balance = test_f.get_sol_balance(payer).await;
    {
        let mut ctx = test_f.context.borrow_mut();
        let drain_amount = current_balance - 1_000_000;
        let drain_account = Keypair::new().pubkey();
        let transfer_ix = system_instruction::transfer(&payer, &drain_account, drain_amount);
        let tx = Transaction::new_signed_with_payer(
            &[transfer_ix],
            Some(&payer),
            &[&payer_keypair],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await?;
    }

    // Try flashloan which should fail since payer doesn't have enough for flashloan fee
    let borrow_ix = borrower_mfi_account_f
        .make_bank_borrow_ix(borrower_token_account_f_sol.key, sol_bank, 500)
        .await;

    let repay_ix = borrower_mfi_account_f
        .make_bank_repay_ix(borrower_token_account_f_sol.key, sol_bank, 500, Some(true))
        .await;

    let res = borrower_mfi_account_f
        .try_flashloan(vec![borrow_ix, repay_ix], vec![], vec![])
        .await;

    assert!(res.is_err());

    Ok(())
}
