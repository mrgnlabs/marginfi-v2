#![no_main]

use std::time::{Duration, SystemTime};

use anchor_lang::{prelude::AccountLoader, Key};
use anyhow::Result;
use arbitrary::Arbitrary;

use bumpalo::Bump;
use lazy_static::{lazy::Lazy, lazy_static};
use libfuzzer_sys::fuzz_target;
use marginfi::{prelude::MarginfiGroup, state::marginfi_group::Bank};
use marginfi_fuzz::{
    log, AccountIdx, AssetAmount, BankAndOracleConfig, BankIdx, MarginfiGroupAccounts, PriceChange,
    N_BANKS, N_USERS,
};
use solana_program::program_pack::Pack;

#[derive(Debug, Arbitrary)]
enum Action {
    Deposit {
        account: AccountIdx,
        bank: BankIdx,
        asset_amount: AssetAmount,
    },
    Borrow {
        account: AccountIdx,
        bank: BankIdx,
        asset_amount: AssetAmount,
    },
    UpdateOracle {
        bank: BankIdx,
        price: PriceChange,
    },
    Repay {
        account: AccountIdx,
        bank: BankIdx,
        asset_amount: AssetAmount,
        repay_all: bool,
    },
    Withdraw {
        account: AccountIdx,
        bank: BankIdx,
        asset_amount: AssetAmount,
        withdraw_all: bool,
    },
    Liquidate {
        liquidator: AccountIdx,
        liquidatee: AccountIdx,
        asset_bank: BankIdx,
        liability_bank: BankIdx,
        asset_amount: AssetAmount,
    },
}

#[derive(Debug)]
pub struct ActionSequence(Vec<Action>);

impl<'a> Arbitrary<'a> for ActionSequence {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let n_actions = 1_000;
        let mut actions = Vec::with_capacity(n_actions);

        for _ in 0..n_actions {
            let action = Action::arbitrary(u)?;
            actions.push(action);
        }

        Ok(ActionSequence(actions))
    }
}

#[derive(Debug, Arbitrary)]
pub struct FuzzerContext {
    pub action_sequence: ActionSequence,
    pub initial_bank_configs: [BankAndOracleConfig; N_BANKS],
}

fuzz_target!(|data: FuzzerContext| { process_actions(data).unwrap() });

fn process_actions(ctx: FuzzerContext) -> Result<()> {
    let mut bump = bumpalo::Bump::new();

    if !*GET_LOGGER {
        println!("Setting up logger");
    }

    let mga = MarginfiGroupAccounts::setup(&bump, &ctx.initial_bank_configs, N_USERS as usize);

    let al =
        AccountLoader::<MarginfiGroup>::try_from_unchecked(&marginfi::id(), &mga.marginfi_group)
            .unwrap();

    assert_eq!(al.load()?.admin, mga.owner.key());

    for action in ctx.action_sequence.0.iter() {
        process_action(action, &mga)?;
    }

    // log! ();
    //
    mga.metrics.read().unwrap().print();

    verify_end_state(&mga)?;

    bump.reset();

    Ok(())
}

static GET_LOGGER: once_cell::sync::Lazy<bool> = once_cell::sync::Lazy::new(|| {
    #[cfg(feature = "capture_log")]
    setup_logging().unwrap();
    true
});

#[cfg(feature = "capture_log")]
fn setup_logging() -> anyhow::Result<()> {
    let logfile = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{l} - {m}\n",
        )))
        .build("log/history.log")?;

    let config = log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            log4rs::config::Root::builder()
                .appender("logfile")
                .build(log::LevelFilter::Info),
        )?;

    log4rs::init_config(config)?;

    println!("Starging logger");

    log::info!("===========START========");

    Ok(())
}

fn verify_end_state(mga: &MarginfiGroupAccounts) -> anyhow::Result<()> {
    mga.banks.iter().try_for_each(|bank| {
        let bank_loader = AccountLoader::<Bank>::try_from(&bank.bank)?;
        let bank_data = bank_loader.load()?;

        let total_deposits = bank_data.get_asset_amount(bank_data.total_asset_shares.into())?;
        let total_liabilities =
            bank_data.get_liability_amount(bank_data.total_liability_shares.into())?;

        let net_accounted_balance = total_deposits - total_liabilities;

        let liquidity_vault_token_account =
            spl_token::state::Account::unpack(&bank.liquidity_vault.data.borrow())?;

        assert_eq!(
            liquidity_vault_token_account.amount,
            net_accounted_balance.to_num::<u64>(),
        );

        Ok::<_, anyhow::Error>(())
    })?;

    Ok(())
}

fn process_action<'bump>(action: &Action, mga: &'bump MarginfiGroupAccounts<'bump>) -> Result<()> {
    match action {
        Action::Deposit {
            account,
            bank,
            asset_amount,
        } => mga.process_action_deposit(account, bank, asset_amount)?,
        Action::Withdraw {
            account,
            bank,
            asset_amount,
            withdraw_all,
        } => mga.process_action_withdraw(account, bank, asset_amount, Some(*withdraw_all))?,
        Action::Borrow {
            account,
            bank,
            asset_amount,
        } => mga.process_action_borrow(account, bank, asset_amount)?,
        Action::Repay {
            account,
            bank,
            asset_amount,
            repay_all,
        } => mga.process_action_repay(account, bank, asset_amount, *repay_all)?,
        Action::UpdateOracle { bank, price } => mga.process_update_oracle(bank, price)?,
        Action::Liquidate {
            liquidator,
            liquidatee,
            asset_bank,
            liability_bank,
            asset_amount,
        } => mga.process_liquidate_account(
            liquidator,
            liquidatee,
            asset_bank,
            liability_bank,
            asset_amount,
        )?,
    };

    mga.advance_time();

    Ok(())
}
