use anchor_lang::{InstructionData, ToAccountMetas};
use fixed::types::I80F48;
use fixed_macro::types::I80F48 as fp;
use fixtures::{
    assert_anchor_error, assert_custom_error, bank::BankFixture,
    marginfi_account::MarginfiAccountFixture, prelude::*, ui_to_native,
};
use marginfi::{prelude::MarginfiError, state::bank::BankVaultType};
use marginfi_type_crate::types::{OrderTrigger, WrappedI80F48};
use solana_program_test::tokio;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction::SystemError,
    system_program, sysvar,
    transaction::Transaction,
};
use test_case::test_case;

/// Helper to create an OrderTrigger with a stop-loss threshold.
fn stop_loss_trigger(threshold: I80F48) -> OrderTrigger {
    OrderTrigger::StopLoss {
        threshold: WrappedI80F48::from(threshold),
    }
}

/// Helper to create an OrderTrigger with a take-profit threshold.
#[allow(dead_code)]
fn take_profit_trigger(threshold: I80F48) -> OrderTrigger {
    OrderTrigger::TakeProfit {
        threshold: WrappedI80F48::from(threshold),
    }
}

fn both_trigger(stop_loss: I80F48, take_profit: I80F48) -> OrderTrigger {
    OrderTrigger::Both {
        stop_loss: WrappedI80F48::from(stop_loss),
        take_profit: WrappedI80F48::from(take_profit),
    }
}

async fn setup_execution_fixture_with_params(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    uninvolved_mint: BankMint,
    trigger: OrderTrigger,
) -> anyhow::Result<(
    TestFixture,
    MarginfiAccountFixture,
    BankMint, // asset mint
    BankMint, // liability mint
    BankMint, // uninvolved mint
    Pubkey,   // order PDA
    Keypair,  // keeper
    Pubkey,   // keeper usdc token account
    Pubkey,   // keeper asset token account
    Pubkey,   // keeper uninvolved token account
)> {
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);
    let uninvolved_bank_f = test_f.get_bank(&uninvolved_mint);

    // borrower positions
    let borrower_mfi_account_f = create_borrower_with_positions(
        &test_f,
        asset_bank_f,
        asset_deposit,
        liability_bank_f,
        liability_borrow,
    )
    .await?;

    // add an uninvolved asset balance
    let uninvolved_account = uninvolved_bank_f
        .mint
        .create_token_account_and_mint_to(0.5)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(uninvolved_account.key, uninvolved_bank_f, 0.5, None)
        .await?;

    // set emissions destination to the authority before placing order
    let authority = borrower_mfi_account_f.load().await.authority;
    borrower_mfi_account_f
        .try_set_emissions_destination(authority)
        .await?;

    // place the order with the provided trigger
    let bank_keys = vec![asset_bank_f.key, liability_bank_f.key];
    let order_pda = borrower_mfi_account_f
        .try_place_order(bank_keys, trigger)
        .await?;

    // keeper setup
    let keeper = Keypair::new();
    fund_keeper_for_fees(&test_f, &keeper).await?;

    let keeper_liab_account = liability_bank_f
        .mint
        .create_token_account_and_mint_to_with_owner(&keeper.pubkey(), 100_000.0)
        .await
        .key;
    let keeper_asset_account = asset_bank_f
        .mint
        .create_empty_token_account_with_owner(&keeper.pubkey())
        .await
        .key;
    let keeper_uninvolved_account = uninvolved_bank_f
        .mint
        .create_empty_token_account_with_owner(&keeper.pubkey())
        .await
        .key;

    Ok((
        test_f,
        borrower_mfi_account_f,
        asset_mint,
        liability_mint,
        uninvolved_mint,
        order_pda,
        keeper,
        keeper_liab_account,
        keeper_asset_account,
        keeper_uninvolved_account,
    ))
}

#[allow(dead_code)]
fn estimate_withdraw_amount(liability_ui: f64, asset_price: f64) -> f64 {
    liability_ui / asset_price
}

fn default_price_for_mint(mint: &BankMint) -> f64 {
    match mint {
        BankMint::Usdc => 1.0,
        BankMint::Sol => 10.0,
        BankMint::Fixed => 2.0,
        other => panic!("unknown mint: {:?}", other),
    }
}

async fn make_start_execute_ix(
    marginfi_account_f: &MarginfiAccountFixture,
    order: Pubkey,
    executor: Pubkey,
) -> anyhow::Result<(Instruction, Pubkey)> {
    let marginfi_account = marginfi_account_f.load().await;
    let (execute_record, _) = find_execute_order_pda(&order);

    let mut ix = Instruction {
        program_id: marginfi::ID,
        accounts: marginfi::accounts::StartExecuteOrder {
            group: marginfi_account.group,
            marginfi_account: marginfi_account_f.key,
            fee_payer: executor,
            executor,
            order,
            execute_record,
            instruction_sysvar: sysvar::instructions::id(),
            system_program: system_program::ID,
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::MarginfiAccountStartExecuteOrder {}.data(),
    };

    ix.accounts.extend_from_slice(
        &marginfi_account_f
            .load_observation_account_metas(vec![], vec![])
            .await,
    );

    Ok((ix, execute_record))
}

async fn make_end_execute_ix(
    marginfi_account_f: &MarginfiAccountFixture,
    order: Pubkey,
    execute_record: Pubkey,
    executor: Pubkey,
    fee_recipient: Pubkey,
    exclude_banks: Vec<Pubkey>,
) -> anyhow::Result<Instruction> {
    let marginfi_account = marginfi_account_f.load().await;

    let mut ix = Instruction {
        program_id: marginfi::ID,
        accounts: marginfi::accounts::EndExecuteOrder {
            group: marginfi_account.group,
            marginfi_account: marginfi_account_f.key,
            executor,
            fee_recipient,
            order,
            execute_record,
        }
        .to_account_metas(Some(true)),
        data: marginfi::instruction::MarginfiAccountEndExecuteOrder {}.data(),
    };

    ix.accounts.extend_from_slice(
        &marginfi_account_f
            .load_observation_account_metas(vec![], exclude_banks)
            .await,
    );

    Ok(ix)
}

async fn make_repay_ix(
    marginfi_account_f: &MarginfiAccountFixture,
    bank_f: &BankFixture,
    authority: Pubkey,
    signer_token_account: Pubkey,
    ui_amount: f64,
    repay_all: Option<bool>,
) -> anyhow::Result<Instruction> {
    let marginfi_account = marginfi_account_f.load().await;

    let mut accounts = marginfi::accounts::LendingAccountRepay {
        group: marginfi_account.group,
        marginfi_account: marginfi_account_f.key,
        authority,
        bank: bank_f.key,
        signer_token_account,
        liquidity_vault: bank_f.get_vault(BankVaultType::Liquidity).0,
        token_program: bank_f.get_token_program(),
    }
    .to_account_metas(Some(true));

    if bank_f.mint.token_program == anchor_spl::token_2022::ID {
        accounts.push(AccountMeta::new_readonly(bank_f.mint.key, false));
    }

    let ix = Instruction {
        program_id: marginfi::ID,
        accounts,
        data: marginfi::instruction::LendingAccountRepay {
            amount: ui_to_native!(ui_amount, bank_f.mint.mint.decimals),
            repay_all,
        }
        .data(),
    };

    Ok(ix)
}

async fn make_withdraw_ix(
    marginfi_account_f: &MarginfiAccountFixture,
    bank_f: &BankFixture,
    authority: Pubkey,
    destination: Pubkey,
    ui_amount: f64,
    withdraw_all: Option<bool>,
) -> anyhow::Result<Instruction> {
    let marginfi_account = marginfi_account_f.load().await;

    let mut accounts = marginfi::accounts::LendingAccountWithdraw {
        group: marginfi_account.group,
        marginfi_account: marginfi_account_f.key,
        authority,
        bank: bank_f.key,
        destination_token_account: destination,
        bank_liquidity_vault_authority: bank_f.get_vault_authority(BankVaultType::Liquidity).0,
        liquidity_vault: bank_f.get_vault(BankVaultType::Liquidity).0,
        token_program: bank_f.get_token_program(),
    }
    .to_account_metas(Some(true));

    if bank_f.mint.token_program == anchor_spl::token_2022::ID {
        accounts.push(AccountMeta::new_readonly(bank_f.mint.key, false));
    }

    let mut ix = Instruction {
        program_id: marginfi::ID,
        accounts,
        data: marginfi::instruction::LendingAccountWithdraw {
            amount: ui_to_native!(ui_amount, bank_f.mint.mint.decimals),
            withdraw_all,
        }
        .data(),
    };

    ix.accounts.extend_from_slice(
        &marginfi_account_f
            .load_observation_account_metas(vec![], vec![])
            .await,
    );

    Ok(ix)
}

async fn fund_keeper_for_fees(test_f: &TestFixture, keeper: &Keypair) -> anyhow::Result<()> {
    let mut ctx = test_f.context.borrow_mut();
    let rent = ctx.banks_client.get_rent().await?;
    let min_balance = rent.minimum_balance(0);
    let account = Account {
        lamports: min_balance + 1_000_000_000,
        data: vec![],
        owner: solana_sdk::system_program::ID,
        executable: false,
        rent_epoch: 0,
    };
    ctx.set_account(&keeper.pubkey(), &account.into());
    Ok(())
}

async fn create_borrower_with_positions(
    test_f: &TestFixture,
    asset_bank_f: &BankFixture,
    asset_deposit: f64,
    liability_bank_f: &BankFixture,
    liability_borrow: f64,
) -> anyhow::Result<MarginfiAccountFixture> {
    let liquidity_seed = (liability_borrow * 10.0).max(1_000.0);

    // Seed liquidity for the liability borrow
    let lender_mfi_account_f = test_f.create_marginfi_account().await;
    let lender_token_account = liability_bank_f
        .mint
        .create_token_account_and_mint_to(liquidity_seed)
        .await;
    lender_mfi_account_f
        .try_bank_deposit(
            lender_token_account.key,
            liability_bank_f,
            liquidity_seed,
            None,
        )
        .await?;

    // Borrower positions
    let borrower_mfi_account_f = test_f.create_marginfi_account().await;
    let borrower_asset_account = asset_bank_f
        .mint
        .create_token_account_and_mint_to(asset_deposit)
        .await;
    borrower_mfi_account_f
        .try_bank_deposit(
            borrower_asset_account.key,
            asset_bank_f,
            asset_deposit,
            None,
        )
        .await?;

    let borrower_liability_account = liability_bank_f.mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_borrow(
            borrower_liability_account.key,
            liability_bank_f,
            liability_borrow,
        )
        .await?;

    Ok(borrower_mfi_account_f)
}

async fn create_dual_asset_account(
    test_f: &TestFixture,
    first_bank: &BankFixture,
    first_deposit: f64,
    second_bank: &BankFixture,
    second_deposit: f64,
) -> anyhow::Result<MarginfiAccountFixture> {
    let mfi_account_f = test_f.create_marginfi_account().await;

    let first_asset_account = first_bank
        .mint
        .create_token_account_and_mint_to(first_deposit)
        .await;
    mfi_account_f
        .try_bank_deposit(first_asset_account.key, first_bank, first_deposit, None)
        .await?;

    let second_asset_account = second_bank
        .mint
        .create_token_account_and_mint_to(second_deposit)
        .await;
    mfi_account_f
        .try_bank_deposit(second_asset_account.key, second_bank, second_deposit, None)
        .await?;

    Ok(mfi_account_f)
}

#[test_case(BankMint::Usdc, 111.5, BankMint::Fixed, 50.0, BankMint::Sol, take_profit_trigger(fp!(12.5)))]
#[test_case(BankMint::Fixed, 5.45, BankMint::Usdc, 9.0, BankMint::Sol, take_profit_trigger(fp!(2)))]
#[test_case(BankMint::Fixed, 5.0, BankMint::Usdc, 8.0, BankMint::Sol, take_profit_trigger(fp!(3)))]
#[test_case(BankMint::Usdc, 111.5, BankMint::Fixed, 50.0, BankMint::Sol, both_trigger(fp!(5), fp!(12.5)))]
#[test_case(BankMint::Fixed, 100.0, BankMint::Usdc, 150.0, BankMint::Sol, both_trigger(fp!(40), fp!(100)))]
#[test_case(BankMint::Usdc, 150.0, BankMint::Fixed, 70.0, BankMint::Sol, stop_loss_trigger(fp!(5)))]
#[test_case(BankMint::Fixed, 100.0, BankMint::Usdc, 150.0, BankMint::Sol, stop_loss_trigger(fp!(40)))]
#[test_case(BankMint::Sol, 200.0, BankMint::Usdc, 50.0, BankMint::Fixed, stop_loss_trigger(fp!(1945)))]
#[tokio::test]
async fn execute_order_fails_pre_trigger_not_met(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    uninvolved_mint: BankMint,
    trigger: OrderTrigger,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let (
        test_f,
        borrower_mfi_account_f,
        asset_mint,
        liability_mint,
        _uninvolved_mint,
        order_pda,
        keeper,
        keeper_liab_account,
        keeper_asset_account,
        _keeper_uninvolved_account,
    ) = setup_execution_fixture_with_params(
        asset_mint,
        asset_deposit,
        liability_mint,
        liability_borrow,
        uninvolved_mint,
        trigger,
    )
    .await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    let price = default_price_for_mint(&asset_mint);

    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);

    let (start_ix, execute_record) =
        make_start_execute_ix(&borrower_mfi_account_f, order_pda, keeper.pubkey()).await?;

    let repay_ix = make_repay_ix(
        &borrower_mfi_account_f,
        &liability_bank_f,
        keeper.pubkey(),
        keeper_liab_account,
        0.0,
        Some(true),
    )
    .await?;

    let withdraw_amt = estimate_withdraw_amount(
        default_price_for_mint(&liability_mint) * liability_borrow,
        price,
    );

    let withdraw_ix = make_withdraw_ix(
        &borrower_mfi_account_f,
        &asset_bank_f,
        keeper.pubkey(),
        keeper_asset_account,
        withdraw_amt,
        None,
    )
    .await?;

    let end_ix = make_end_execute_ix(
        &borrower_mfi_account_f,
        order_pda,
        execute_record,
        keeper.pubkey(),
        keeper.pubkey(),
        vec![liability_bank_f.key],
    )
    .await?;

    let ctx = test_f.context.borrow_mut();
    let tx = Transaction::new_signed_with_payer(
        &[start_ix, repay_ix, withdraw_ix, end_ix],
        Some(&keeper.pubkey()),
        &[&keeper],
        ctx.last_blockhash,
    );

    let result = ctx.banks_client.process_transaction(tx).await;
    assert_custom_error!(result.unwrap_err(), MarginfiError::OrderTriggerNotMet);
    Ok(())
}

#[test_case(BankMint::Fixed, 7.0, BankMint::Usdc, 5.0, BankMint::Sol, take_profit_trigger(fp!(5.5)))]
#[test_case(BankMint::Sol, 150.0, BankMint::Usdc, 10.0, BankMint::Fixed, take_profit_trigger(fp!(1490)))]
#[test_case(BankMint::Fixed, 7.0, BankMint::Sol, 0.8, BankMint::Usdc, take_profit_trigger(fp!(3)))]
#[test_case(BankMint::Fixed, 7.0, BankMint::Usdc, 5.0, BankMint::Sol, both_trigger(fp!(10), fp!(3.5)))]
#[test_case(BankMint::Sol, 150.0, BankMint::Usdc, 1000.0, BankMint::Fixed, both_trigger(fp!(600), fp!(1490)))]
#[test_case(BankMint::Fixed, 12.5, BankMint::Usdc, 20.0, BankMint::Sol, stop_loss_trigger(fp!(10)))]
#[test_case(BankMint::Sol, 150.0, BankMint::Usdc, 1000.0, BankMint::Fixed, stop_loss_trigger(fp!(600)))]
#[test_case(BankMint::Fixed, 5.5, BankMint::Sol, 0.8, BankMint::Usdc, stop_loss_trigger(fp!(5)))]
#[tokio::test]
async fn execute_order_fails_post_trigger_not_met(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    uninvolved_mint: BankMint,
    trigger: OrderTrigger,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let (
        test_f,
        borrower_mfi_account_f,
        asset_mint,
        liability_mint,
        _uninvolved_mint,
        order_pda,
        keeper,
        keeper_liab_account,
        keeper_asset_account,
        _keeper_uninvolved_account,
    ) = setup_execution_fixture_with_params(
        asset_mint,
        asset_deposit,
        liability_mint,
        liability_borrow,
        uninvolved_mint,
        trigger,
    )
    .await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    let price = default_price_for_mint(&asset_mint);

    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);

    let (start_ix, execute_record) =
        make_start_execute_ix(&borrower_mfi_account_f, order_pda, keeper.pubkey()).await?;

    let repay_ix = make_repay_ix(
        &borrower_mfi_account_f,
        &liability_bank_f,
        keeper.pubkey(),
        keeper_liab_account,
        0.0,
        Some(true),
    )
    .await?;

    let asset_value = price * asset_deposit;
    let liab_value = estimate_withdraw_amount(
        default_price_for_mint(&liability_mint) * liability_borrow,
        price,
    );
    let withdraw_amt = match trigger {
        OrderTrigger::TakeProfit { threshold } => {
            let threshold: I80F48 = threshold.into();
            let trigger_threshold = threshold.to_num::<f64>();

            ((asset_value - trigger_threshold) / price) * 1.05 // Make it go past the threshold
        }
        OrderTrigger::StopLoss { threshold: _ } => {
            liab_value * 1.05 // Make it go past the threshold
        }
        OrderTrigger::Both {
            stop_loss: _,
            take_profit,
        } => {
            if asset_value - liab_value >= Into::<I80F48>::into(take_profit).to_num::<f64>() {
                let threshold: I80F48 = take_profit.into();
                let trigger_threshold = threshold.to_num::<f64>();
                ((asset_value - trigger_threshold) / price) * 1.05 // Make it go past the threshold
            } else {
                liab_value * 1.05 // Make it go past the threshold
            }
        }
    };

    let withdraw_ix = make_withdraw_ix(
        &borrower_mfi_account_f,
        &asset_bank_f,
        keeper.pubkey(),
        keeper_asset_account,
        withdraw_amt,
        None,
    )
    .await?;

    let end_ix = make_end_execute_ix(
        &borrower_mfi_account_f,
        order_pda,
        execute_record,
        keeper.pubkey(),
        keeper.pubkey(),
        vec![liability_bank_f.key],
    )
    .await?;

    let ctx = test_f.context.borrow_mut();
    let tx = Transaction::new_signed_with_payer(
        &[start_ix, repay_ix, withdraw_ix, end_ix],
        Some(&keeper.pubkey()),
        &[&keeper],
        ctx.last_blockhash,
    );

    let result = ctx.banks_client.process_transaction(tx).await;
    assert_custom_error!(result.unwrap_err(), MarginfiError::OrderTriggerNotMet);
    Ok(())
}

#[test_case(BankMint::Fixed, 25.5, BankMint::Usdc, 46.0, BankMint::Sol, take_profit_trigger(fp!(5)))]
#[test_case(BankMint::Sol, 5.45, BankMint::Usdc, 50.0, BankMint::Fixed, take_profit_trigger(fp!(2)))]
#[test_case(BankMint::Fixed, 5.5, BankMint::Sol, 0.8, BankMint::Usdc, take_profit_trigger(fp!(2.5)))]
#[test_case(BankMint::Fixed, 25.5, BankMint::Usdc, 46.0, BankMint::Sol, both_trigger(fp!(25), fp!(5)))]
#[test_case(BankMint::Fixed, 100.0, BankMint::Usdc, 150.0, BankMint::Sol, both_trigger(fp!(60), fp!(2)))]
#[test_case(BankMint::Usdc, 150.0, BankMint::Fixed, 65.0, BankMint::Sol, stop_loss_trigger(fp!(25)))]
#[test_case(BankMint::Fixed, 100.0, BankMint::Usdc, 150.0, BankMint::Sol, stop_loss_trigger(fp!(60)))]
#[test_case(BankMint::Sol, 40.0, BankMint::Usdc, 50.0, BankMint::Fixed, stop_loss_trigger(fp!(360)))]
#[tokio::test]
async fn execute_order_fails_touch_uninvolved_balance(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    uninvolved_mint: BankMint,
    trigger: OrderTrigger,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let (
        test_f,
        borrower_mfi_account_f,
        asset_mint,
        liability_mint,
        uninvolved_mint,
        order_pda,
        keeper,
        keeper_liab_account,
        keeper_asset_account,
        keeper_uninvolved_account,
    ) = setup_execution_fixture_with_params(
        asset_mint,
        asset_deposit,
        liability_mint,
        liability_borrow,
        uninvolved_mint,
        trigger,
    )
    .await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    let price = default_price_for_mint(&asset_mint);

    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);
    let uninvolved_bank_f = test_f.get_bank(&uninvolved_mint);

    let (start_ix, execute_record) =
        make_start_execute_ix(&borrower_mfi_account_f, order_pda, keeper.pubkey()).await?;

    let repay_ix = make_repay_ix(
        &borrower_mfi_account_f,
        &liability_bank_f,
        keeper.pubkey(),
        keeper_liab_account,
        0.0,
        Some(true),
    )
    .await?;

    let withdraw_amt = estimate_withdraw_amount(
        default_price_for_mint(&liability_mint) * liability_borrow,
        price,
    );

    let withdraw_ix = make_withdraw_ix(
        &borrower_mfi_account_f,
        &asset_bank_f,
        keeper.pubkey(),
        keeper_asset_account,
        withdraw_amt,
        None,
    )
    .await?;

    // touch unrelated SOL balance
    let withdraw_sol_ix = make_withdraw_ix(
        &borrower_mfi_account_f,
        &uninvolved_bank_f,
        keeper.pubkey(),
        keeper_uninvolved_account,
        0.001,
        None,
    )
    .await?;

    let end_ix = make_end_execute_ix(
        &borrower_mfi_account_f,
        order_pda,
        execute_record,
        keeper.pubkey(),
        keeper.pubkey(),
        vec![liability_bank_f.key],
    )
    .await?;

    let ctx = test_f.context.borrow_mut();
    let tx = Transaction::new_signed_with_payer(
        &[start_ix, repay_ix, withdraw_ix, withdraw_sol_ix, end_ix],
        Some(&keeper.pubkey()),
        &[&keeper],
        ctx.last_blockhash,
    );

    let result = ctx.banks_client.process_transaction(tx).await;
    assert_custom_error!(result.unwrap_err(), MarginfiError::IllegalBalanceState);
    Ok(())
}

#[test_case(BankMint::Fixed, 625.5, BankMint::Usdc, 1245.0, BankMint::Sol, take_profit_trigger(fp!(5.5)))]
#[test_case(BankMint::Sol, 150.0, BankMint::Usdc, 10.0, BankMint::Fixed, take_profit_trigger(fp!(1490)))]
#[test_case(BankMint::Fixed, 5.5, BankMint::Sol, 0.8, BankMint::Usdc, take_profit_trigger(fp!(3)))]
#[test_case(BankMint::Fixed, 625.5, BankMint::Usdc, 1245.0, BankMint::Sol, both_trigger(fp!(10), fp!(5.5)))]
#[test_case(BankMint::Sol, 150.0, BankMint::Usdc, 1000.0, BankMint::Fixed, both_trigger(fp!(600), fp!(1490)))]
#[test_case(BankMint::Fixed, 625.5, BankMint::Usdc, 1245.0, BankMint::Sol, stop_loss_trigger(fp!(10)))]
#[test_case(BankMint::Sol, 150.0, BankMint::Usdc, 1000.0, BankMint::Fixed, stop_loss_trigger(fp!(600)))]
#[test_case(BankMint::Fixed, 5.5, BankMint::Sol, 0.8, BankMint::Usdc, stop_loss_trigger(fp!(5)))]
#[tokio::test]
async fn execute_order_fails_health_check(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    uninvolved_mint: BankMint,
    trigger: OrderTrigger,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let (
        test_f,
        borrower_mfi_account_f,
        asset_mint,
        liability_mint,
        uninvolved_mint,
        order_pda,
        keeper,
        keeper_liab_account,
        keeper_asset_account,
        keeper_uninvolved_account,
    ) = setup_execution_fixture_with_params(
        asset_mint,
        asset_deposit,
        liability_mint,
        liability_borrow,
        uninvolved_mint,
        trigger,
    )
    .await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);
    let uninvolved_bank_f = test_f.get_bank(&uninvolved_mint);

    // drain all SOL from the borrower
    borrower_mfi_account_f
        .try_bank_withdraw(
            keeper_uninvolved_account,
            uninvolved_bank_f,
            0.0,
            Some(true),
        )
        .await?;

    // seed SOL liquidity so the borrower can re-borrow after draining
    let sol_liquidity_provider = test_f.create_marginfi_account().await;
    let sol_liquidity_seed = 0.75;
    let sol_liquidity_account = uninvolved_bank_f
        .mint
        .create_token_account_and_mint_to(sol_liquidity_seed)
        .await;
    sol_liquidity_provider
        .try_bank_deposit(
            sol_liquidity_account.key,
            uninvolved_bank_f,
            sol_liquidity_seed,
            None,
        )
        .await?;

    // borrow a small amount of SOL to push post-execution health below zero
    let small_sol_borrow = 0.01;
    borrower_mfi_account_f
        .try_bank_borrow(
            keeper_uninvolved_account,
            uninvolved_bank_f,
            small_sol_borrow,
        )
        .await?;

    let (start_ix, execute_record) =
        make_start_execute_ix(&borrower_mfi_account_f, order_pda, keeper.pubkey()).await?;

    let repay_ix = make_repay_ix(
        &borrower_mfi_account_f,
        &liability_bank_f,
        keeper.pubkey(),
        keeper_liab_account,
        0.0,
        Some(true),
    )
    .await?;

    let withdraw_ix = make_withdraw_ix(
        &borrower_mfi_account_f,
        &asset_bank_f,
        keeper.pubkey(),
        keeper_asset_account,
        0.0,
        Some(true),
    )
    .await?;

    let end_ix = make_end_execute_ix(
        &borrower_mfi_account_f,
        order_pda,
        execute_record,
        keeper.pubkey(),
        keeper.pubkey(),
        vec![liability_bank_f.key, asset_bank_f.key],
    )
    .await?;

    let ctx = test_f.context.borrow_mut();
    let tx = Transaction::new_signed_with_payer(
        &[start_ix, repay_ix, withdraw_ix, end_ix],
        Some(&keeper.pubkey()),
        &[&keeper],
        ctx.last_blockhash,
    );

    let result = ctx.banks_client.process_transaction(tx).await;
    assert_custom_error!(result.unwrap_err(), MarginfiError::AccountNotHealthy);
    Ok(())
}

#[test_case(BankMint::Fixed, 500.0, BankMint::Usdc, 985.0, BankMint::Sol, take_profit_trigger(fp!(10)))]
#[test_case(BankMint::Sol, 5.0, BankMint::Usdc, 10.0, BankMint::Fixed, take_profit_trigger(fp!(35)))]
#[test_case(BankMint::Fixed, 5.0, BankMint::Usdc, 9.0, BankMint::Sol, take_profit_trigger(fp!(0.5)))]
#[test_case(BankMint::Fixed, 500.0, BankMint::Usdc, 985.0, BankMint::Sol, both_trigger(fp!(1100), fp!(10)))]
#[test_case(BankMint::Sol, 150.0, BankMint::Usdc, 10.0, BankMint::Fixed, both_trigger(fp!(1495), fp!(35)))]
#[test_case(BankMint::Fixed, 1000.0, BankMint::Usdc, 985.0, BankMint::Sol, stop_loss_trigger(fp!(1100)))]
#[test_case(BankMint::Sol, 150.0, BankMint::Usdc, 10.0, BankMint::Fixed, stop_loss_trigger(fp!(1495)))]
#[test_case(BankMint::Fixed, 5.0, BankMint::Usdc, 9.0, BankMint::Sol, stop_loss_trigger(fp!(2)))]
#[tokio::test]
async fn execute_order_success(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    uninvolved_mint: BankMint,
    trigger: OrderTrigger,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let (
        test_f,
        borrower_mfi_account_f,
        asset_mint,
        liability_mint,
        uninvolved_mint,
        order_pda,
        keeper,
        keeper_liab_account,
        keeper_asset_account,
        _keeper_uninvolved_account,
    ) = setup_execution_fixture_with_params(
        asset_mint,
        asset_deposit,
        liability_mint,
        liability_borrow,
        uninvolved_mint,
        trigger,
    )
    .await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    let price = default_price_for_mint(&asset_mint);

    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);
    let uninvolved_bank_f = test_f.get_bank(&uninvolved_mint);

    let order_before = borrower_mfi_account_f.load_order(order_pda).await;
    let mfi_before = borrower_mfi_account_f.load().await;

    let (start_ix, execute_record) =
        make_start_execute_ix(&borrower_mfi_account_f, order_pda, keeper.pubkey()).await?;

    let repay_ix = make_repay_ix(
        &borrower_mfi_account_f,
        &liability_bank_f,
        keeper.pubkey(),
        keeper_liab_account,
        0.0,
        Some(true),
    )
    .await?;

    let withdraw_amt = estimate_withdraw_amount(
        default_price_for_mint(&liability_mint) * liability_borrow,
        price,
    );

    let withdraw_ix = make_withdraw_ix(
        &borrower_mfi_account_f,
        &asset_bank_f,
        keeper.pubkey(),
        keeper_asset_account,
        withdraw_amt,
        None,
    )
    .await?;

    let end_ix = make_end_execute_ix(
        &borrower_mfi_account_f,
        order_pda,
        execute_record,
        keeper.pubkey(),
        keeper.pubkey(),
        vec![liability_bank_f.key],
    )
    .await?;

    let ctx = test_f.context.borrow_mut();
    let tx = Transaction::new_signed_with_payer(
        &[start_ix, repay_ix, withdraw_ix, end_ix],
        Some(&keeper.pubkey()),
        &[&keeper],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    // order closed
    let order_after = test_f.try_load(&order_pda).await?;
    assert!(
        order_after.is_none(),
        "order should be closed after execution"
    );

    // verify balances: asset still present, liability removed, uninvolved remains
    let mfi_after = borrower_mfi_account_f.load().await;
    let asset_tag = order_before.tags[0];
    let liab_tag = order_before.tags[1];

    let pre_asset = mfi_before
        .lending_account
        .balances
        .iter()
        .find(|b| b.tag == asset_tag)
        .unwrap();
    let pre_liab = mfi_before
        .lending_account
        .balances
        .iter()
        .find(|b| b.tag == liab_tag)
        .unwrap();

    let post_asset = mfi_after
        .lending_account
        .balances
        .iter()
        .find(|b| b.bank_pk == pre_asset.bank_pk);
    assert!(post_asset.is_some(), "asset balance should remain");

    let post_liab = mfi_after
        .lending_account
        .balances
        .iter()
        .find(|b| b.bank_pk == pre_liab.bank_pk);
    assert!(post_liab.is_none(), "liability balance should be removed");

    // uninvolved SOL balance unchanged
    let pre_sol = mfi_before
        .lending_account
        .balances
        .iter()
        .find(|b| b.bank_pk == uninvolved_bank_f.key)
        .unwrap();
    let post_sol = mfi_after
        .lending_account
        .balances
        .iter()
        .find(|b| b.bank_pk == uninvolved_bank_f.key)
        .unwrap();
    assert_eq!(pre_sol.asset_shares, post_sol.asset_shares);
    assert_eq!(pre_sol.liability_shares, post_sol.liability_shares);

    // sanity: compare value to the trigger
    let post_asset_shares: I80F48 = post_asset.unwrap().asset_shares.into();
    let asset_native =
        post_asset_shares.to_num::<f64>() / 10f64.powi(asset_bank_f.mint.mint.decimals as i32);
    let asset_value = asset_native * price;
    match trigger {
        OrderTrigger::TakeProfit { threshold } => {
            let threshold: I80F48 = threshold.into();
            let trigger_threshold = threshold.to_num::<f64>();
            assert!(asset_value >= trigger_threshold);
        }
        OrderTrigger::StopLoss { threshold: _ } => {
            // For stop-loss ensure: new asset value >= (old asset value - old liability value)
            let pre_asset_shares: I80F48 = pre_asset.asset_shares.into();
            let pre_asset_native = pre_asset_shares.to_num::<f64>()
                / 10f64.powi(asset_bank_f.mint.mint.decimals as i32);
            let pre_asset_value = pre_asset_native * price;

            let pre_liab_shares: I80F48 = pre_liab.liability_shares.into();
            let pre_liab_native = pre_liab_shares.to_num::<f64>()
                / 10f64.powi(liability_bank_f.mint.mint.decimals as i32);
            let pre_liab_value = pre_liab_native * default_price_for_mint(&liability_mint);

            assert!(asset_value >= (pre_asset_value - pre_liab_value));
        }
        OrderTrigger::Both {
            stop_loss: _,
            take_profit,
        } => {
            // take-profit
            let threshold: I80F48 = take_profit.into();
            let trigger_threshold = threshold.to_num::<f64>();

            // stop-loss
            let pre_asset_shares: I80F48 = pre_asset.asset_shares.into();
            let pre_asset_native = pre_asset_shares.to_num::<f64>()
                / 10f64.powi(asset_bank_f.mint.mint.decimals as i32);
            let pre_asset_value = pre_asset_native * price;

            let pre_liab_shares: I80F48 = pre_liab.liability_shares.into();
            let pre_liab_native = pre_liab_shares.to_num::<f64>()
                / 10f64.powi(liability_bank_f.mint.mint.decimals as i32);
            let pre_liab_value = pre_liab_native * default_price_for_mint(&liability_mint);

            // any
            assert!(
                asset_value >= trigger_threshold
                    || asset_value >= (pre_asset_value - pre_liab_value)
            );
        }
    }

    Ok(())
}

#[test_case(BankMint::Usdc, 200.0, BankMint::Sol, 6.0, fp!(50))]
#[test_case(BankMint::Sol, 70.0, BankMint::Usdc, 500.0, fp!(100))]
#[test_case(BankMint::Fixed, 700.0, BankMint::Usdc, 500.0, fp!(100))]
#[tokio::test]
async fn place_order_success_one_asset_one_liability(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    trigger_threshold: I80F48,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);

    let borrower_mfi_account_f = create_borrower_with_positions(
        &test_f,
        asset_bank_f,
        asset_deposit,
        liability_bank_f,
        liability_borrow,
    )
    .await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    // set emissions destination to the authority before placing order
    let authority = borrower_mfi_account_f.load().await.authority;
    borrower_mfi_account_f
        .try_set_emissions_destination(authority)
        .await?;

    let bank_keys = vec![asset_bank_f.key, liability_bank_f.key];
    let trigger = stop_loss_trigger(trigger_threshold);

    let order_pda = borrower_mfi_account_f
        .try_place_order(bank_keys.clone(), trigger)
        .await?;

    // Verify order was created correctly
    let order = borrower_mfi_account_f.load_order(order_pda).await;
    assert_eq!(order.marginfi_account, borrower_mfi_account_f.key);
    assert!(order.tags[0] > 0, "tag 0 should be non-zero");
    assert!(order.tags[1] > 0, "tag 1 should be non-zero");
    assert_ne!(order.tags[0], order.tags[1], "tags should be different");

    // Verify tags are set on the marginfi account balances
    let marginfi_account = borrower_mfi_account_f.load().await;
    let has_tag_0 = marginfi_account
        .lending_account
        .balances
        .iter()
        .any(|b| b.tag == order.tags[0]);
    let has_tag_1 = marginfi_account
        .lending_account
        .balances
        .iter()
        .any(|b| b.tag == order.tags[1]);
    assert!(has_tag_0, "balance with tag 0 should exist");
    assert!(has_tag_1, "balance with tag 1 should exist");

    Ok(())
}

#[test_case(BankMint::Usdc, 1_000.0, BankMint::Sol, 5.0)]
#[test_case(BankMint::Sol, 5.0, BankMint::Usdc, 500.0)]
#[test_case(BankMint::Fixed, 20.0, BankMint::Usdc, 500.0)]
#[tokio::test]
async fn place_order_fails_both_assets(
    first_asset_mint: BankMint,
    first_deposit: f64,
    second_asset_mint: BankMint,
    second_deposit: f64,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let first_bank_f = test_f.get_bank(&first_asset_mint);
    let second_bank_f = test_f.get_bank(&second_asset_mint);

    let mfi_account_f = create_dual_asset_account(
        &test_f,
        first_bank_f,
        first_deposit,
        second_bank_f,
        second_deposit,
    )
    .await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    // set emissions destination to the authority before placing order
    let authority = mfi_account_f.load().await.authority;
    mfi_account_f
        .try_set_emissions_destination(authority)
        .await?;

    let bank_keys = vec![first_bank_f.key, second_bank_f.key];
    let trigger = stop_loss_trigger(fp!(100));

    let result = mfi_account_f.try_place_order(bank_keys, trigger).await;

    assert_custom_error!(
        result.unwrap_err(),
        MarginfiError::InvalidAssetOrLiabilitiesCount
    );

    Ok(())
}

#[test_case(BankMint::Fixed, 1_000.0, BankMint::Sol, 5.0)]
#[test_case(BankMint::Usdc, 500.0, BankMint::Fixed, 50.0)]
#[test_case(BankMint::Sol, 10.0, BankMint::Usdc, 50.0)]
#[tokio::test]
async fn place_order_fails_same_order_twice(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);

    let borrower_mfi_account_f = create_borrower_with_positions(
        &test_f,
        asset_bank_f,
        asset_deposit,
        liability_bank_f,
        liability_borrow,
    )
    .await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    // set emissions destination to the authority before placing order
    let authority = borrower_mfi_account_f.load().await.authority;
    borrower_mfi_account_f
        .try_set_emissions_destination(authority)
        .await?;

    let bank_keys = vec![asset_bank_f.key, liability_bank_f.key];
    let trigger = stop_loss_trigger(fp!(100));
    borrower_mfi_account_f
        .try_place_order(bank_keys.clone(), trigger)
        .await?;

    let trigger2 = both_trigger(fp!(50), fp!(200));
    let result = borrower_mfi_account_f
        .try_place_order(bank_keys, trigger2)
        .await;

    assert_anchor_error!(result.unwrap_err(), SystemError::AccountAlreadyInUse);

    Ok(())
}

#[test_case(BankMint::Usdc, 300.0, BankMint::Fixed, 50.0, fp!(100))]
#[test_case(BankMint::Fixed, 100.0, BankMint::Sol, 10.0, fp!(80))]
#[test_case(BankMint::Sol, 20.0, BankMint::Usdc, 75.0, fp!(50))]
#[tokio::test]
async fn close_order_success_authority(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    trigger_threshold: I80F48,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);

    let borrower_mfi_account_f = create_borrower_with_positions(
        &test_f,
        asset_bank_f,
        asset_deposit,
        liability_bank_f,
        liability_borrow,
    )
    .await?;

    // set emissions destination to the authority before placing order
    let authority = borrower_mfi_account_f.load().await.authority;
    borrower_mfi_account_f
        .try_set_emissions_destination(authority)
        .await?;

    let bank_keys = vec![asset_bank_f.key, liability_bank_f.key];
    let trigger = stop_loss_trigger(trigger_threshold);
    let order_pda = borrower_mfi_account_f
        .try_place_order(bank_keys.clone(), trigger)
        .await?;

    // Verify order exists
    let order_account = test_f.try_load(&order_pda).await?;
    assert!(order_account.is_some(), "order should exist before close");

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    let fee_recipient = test_f.payer();
    borrower_mfi_account_f
        .try_close_order(order_pda, fee_recipient)
        .await?;

    // Verify order is closed
    let order_account_after = test_f.try_load(&order_pda).await?;
    assert!(
        order_account_after.is_none(),
        "order should be closed after close_order"
    );

    Ok(())
}

#[test_case(BankMint::Usdc, 300.0, BankMint::Fixed, 50.0, fp!(100))]
#[test_case(BankMint::Fixed, 100.0, BankMint::Sol, 10.0, fp!(80))]
#[test_case(BankMint::Sol, 20.0, BankMint::Usdc, 75.0, fp!(50))]
#[tokio::test]
async fn keeper_close_order_success_after_clearing_side(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    trigger_threshold: I80F48,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);

    let borrower_mfi_account_f = create_borrower_with_positions(
        &test_f,
        asset_bank_f,
        asset_deposit,
        liability_bank_f,
        liability_borrow,
    )
    .await?;

    // set emissions destination to the authority before placing order
    let authority = borrower_mfi_account_f.load().await.authority;
    borrower_mfi_account_f
        .try_set_emissions_destination(authority)
        .await?;

    let bank_keys = vec![asset_bank_f.key, liability_bank_f.key];
    let trigger = stop_loss_trigger(trigger_threshold);
    let order_pda = borrower_mfi_account_f
        .try_place_order(bank_keys.clone(), trigger)
        .await?;

    // Clear the liability side by repaying fully
    let repay_amount = liability_borrow * 2.0;
    let repay_token_account = liability_bank_f
        .mint
        .create_token_account_and_mint_to(repay_amount)
        .await;
    borrower_mfi_account_f
        .try_bank_repay(repay_token_account.key, liability_bank_f, 0.0, Some(true))
        .await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    let keeper = Keypair::new();
    fund_keeper_for_fees(&test_f, &keeper).await?;

    let fee_recipient = keeper.pubkey();
    borrower_mfi_account_f
        .try_keeper_close_order(order_pda, &keeper, fee_recipient)
        .await?;

    // Verify order is closed
    let order_account_after = test_f.try_load(&order_pda).await?;
    assert!(
        order_account_after.is_none(),
        "order should be closed after keeper_close_order"
    );

    Ok(())
}

#[test_case(BankMint::Usdc, 300.0, BankMint::Fixed, 50.0, fp!(100))]
#[test_case(BankMint::Fixed, 100.0, BankMint::Sol, 10.0, fp!(80))]
#[test_case(BankMint::Sol, 20.0, BankMint::Usdc, 75.0, fp!(50))]
#[tokio::test]
async fn keeper_can_close_order_after_marginfi_account_closed(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    trigger_threshold: I80F48,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------
    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);

    let borrower_mfi_account_f = create_borrower_with_positions(
        &test_f,
        asset_bank_f,
        asset_deposit,
        liability_bank_f,
        liability_borrow,
    )
    .await?;

    // set emissions destination to the authority before placing order
    let authority = borrower_mfi_account_f.load().await.authority;
    borrower_mfi_account_f
        .try_set_emissions_destination(authority)
        .await?;

    let bank_keys = vec![asset_bank_f.key, liability_bank_f.key];
    let trigger = stop_loss_trigger(trigger_threshold);
    let order_pda = borrower_mfi_account_f
        .try_place_order(bank_keys.clone(), trigger)
        .await?;

    // Verify order exists
    let order_before = test_f.try_load(&order_pda).await?;
    assert!(order_before.is_some(), "order should exist before cleanup");

    // Clear balances

    let repay_account = liability_bank_f
        .mint
        .create_token_account_and_mint_to(liability_borrow * 2.0)
        .await;
    borrower_mfi_account_f
        .try_bank_repay(repay_account.key, liability_bank_f, 0.0, Some(true))
        .await?;

    let withdraw_destination = asset_bank_f.mint.create_empty_token_account().await;
    borrower_mfi_account_f
        .try_bank_withdraw(withdraw_destination.key, asset_bank_f, 0.0, Some(true))
        .await?;

    let marginfi_account_after = borrower_mfi_account_f.load().await;
    let active_balances = marginfi_account_after
        .lending_account
        .balances
        .iter()
        .filter(|b| b.is_active())
        .count();
    assert_eq!(active_balances, 0, "all balances should be closed");

    borrower_mfi_account_f.try_close_account(1).await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    let keeper = Keypair::new();
    fund_keeper_for_fees(&test_f, &keeper).await?;
    let fee_recipient = keeper.pubkey();

    borrower_mfi_account_f
        .try_keeper_close_order(order_pda, &keeper, fee_recipient)
        .await?;

    let order_after = test_f.try_load(&order_pda).await?;
    assert!(order_after.is_none(), "order should be closed by keeper");

    Ok(())
}

#[test_case(BankMint::Usdc, 300.0, BankMint::Fixed, 50.0, fp!(100))]
#[test_case(BankMint::Fixed, 150.0, BankMint::Sol, 20.0, fp!(80))]
#[test_case(BankMint::Sol, 20.0, BankMint::Usdc, 75.0, fp!(50))]
#[tokio::test]
async fn keeper_close_order_fails_active_tags(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    trigger_threshold: I80F48,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);

    let borrower_mfi_account_f = create_borrower_with_positions(
        &test_f,
        asset_bank_f,
        asset_deposit,
        liability_bank_f,
        liability_borrow,
    )
    .await?;

    // set emissions destination to the authority before placing order
    let authority = borrower_mfi_account_f.load().await.authority;
    borrower_mfi_account_f
        .try_set_emissions_destination(authority)
        .await?;

    let bank_keys = vec![asset_bank_f.key, liability_bank_f.key];
    let trigger = stop_loss_trigger(trigger_threshold);
    let order_pda = borrower_mfi_account_f
        .try_place_order(bank_keys.clone(), trigger)
        .await?;

    let keeper = Keypair::new();
    fund_keeper_for_fees(&test_f, &keeper).await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    let fee_recipient = keeper.pubkey();
    let result = borrower_mfi_account_f
        .try_keeper_close_order(order_pda, &keeper, fee_recipient)
        .await;

    assert_custom_error!(
        result.unwrap_err(),
        MarginfiError::LiquidatorOrderCloseNotAllowed
    );

    Ok(())
}

#[test_case(BankMint::Usdc, 1_000.0, BankMint::Sol, 5.0, BankMint::Sol, fp!(900))]
#[test_case(BankMint::Usdc, 850.0, BankMint::Fixed, 50.0, BankMint::Usdc, fp!(600))]
#[test_case(BankMint::Sol, 100.0, BankMint::Fixed, 400.0, BankMint::Sol, fp!(50))]
#[tokio::test]
async fn set_liquidator_close_order_flags_success(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    flagged_bank_mint: BankMint,
    trigger_threshold: I80F48,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);
    let flagged_bank_f = test_f.get_bank(&flagged_bank_mint);

    let borrower_mfi_account_f = create_borrower_with_positions(
        &test_f,
        asset_bank_f,
        asset_deposit,
        liability_bank_f,
        liability_borrow,
    )
    .await?;

    // set emissions destination to the authority before placing order
    let authority = borrower_mfi_account_f.load().await.authority;
    borrower_mfi_account_f
        .try_set_emissions_destination(authority)
        .await?;

    let bank_keys = vec![asset_bank_f.key, liability_bank_f.key];
    let trigger = stop_loss_trigger(trigger_threshold);
    let order_pda = borrower_mfi_account_f
        .try_place_order(bank_keys.clone(), trigger)
        .await?;

    // Verify tags are non-zero before
    let order_before = borrower_mfi_account_f.load_order(order_pda).await;
    assert!(order_before.tags[0] > 0, "tag 0 should be non-zero before");
    assert!(order_before.tags[1] > 0, "tag 1 should be non-zero before");

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    borrower_mfi_account_f
        .try_set_keeper_close_flags(Some(vec![flagged_bank_f.key]))
        .await?;

    // Verify the flagged balance's tag is now zero
    let marginfi_account_after = borrower_mfi_account_f.load().await;
    let flagged_balance = marginfi_account_after
        .lending_account
        .balances
        .iter()
        .find(|b| b.is_active() && b.bank_pk == flagged_bank_f.key);

    assert!(
        flagged_balance.is_some(),
        "flagged balance should still exist"
    );
    assert_eq!(
        flagged_balance.unwrap().tag,
        0,
        "flagged balance tag should be zeroed after set_liquidator_close_flags"
    );

    Ok(())
}

#[test_case(BankMint::Usdc, 100.0, BankMint::Sol, 5.0, fp!(20))]
#[test_case(BankMint::Usdc, 850.0, BankMint::Fixed, 50.0, fp!(600))]
#[test_case(BankMint::Sol, 3.0, BankMint::Fixed, 10.0, fp!(5))]
#[tokio::test]
async fn keeper_close_order_success_after_set_flags(
    asset_mint: BankMint,
    asset_deposit: f64,
    liability_mint: BankMint,
    liability_borrow: f64,
    trigger_threshold: I80F48,
) -> anyhow::Result<()> {
    // ---------------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------------

    let test_f = TestFixture::new(Some(TestSettings::all_banks_payer_not_admin())).await;
    let asset_bank_f = test_f.get_bank(&asset_mint);
    let liability_bank_f = test_f.get_bank(&liability_mint);

    let borrower_mfi_account_f = create_borrower_with_positions(
        &test_f,
        asset_bank_f,
        asset_deposit,
        liability_bank_f,
        liability_borrow,
    )
    .await?;

    // set emissions destination to the authority before placing order
    let authority = borrower_mfi_account_f.load().await.authority;
    borrower_mfi_account_f
        .try_set_emissions_destination(authority)
        .await?;

    let bank_keys = vec![asset_bank_f.key, liability_bank_f.key];
    let trigger = stop_loss_trigger(trigger_threshold);
    let order_pda = borrower_mfi_account_f
        .try_place_order(bank_keys.clone(), trigger)
        .await?;

    borrower_mfi_account_f
        .try_set_keeper_close_flags(Some(vec![asset_bank_f.key, liability_bank_f.key]))
        .await?;

    let keeper = Keypair::new();
    fund_keeper_for_fees(&test_f, &keeper).await?;

    // ---------------------------------------------------------------------
    // Test
    // ---------------------------------------------------------------------

    let fee_recipient = keeper.pubkey();
    borrower_mfi_account_f
        .try_keeper_close_order(order_pda, &keeper, fee_recipient)
        .await?;

    // Verify order is closed
    let order_account_after = test_f.try_load(&order_pda).await?;
    assert!(
        order_account_after.is_none(),
        "order should be closed after keeper_close_order"
    );

    Ok(())
}
