#![no_main]

use std::{collections::BTreeMap, error::Error};

use anchor_lang::{
    prelude::{AccountLoader, Context, Pubkey, Rent},
    Key,
};
use anyhow::Result;
use arbitrary::Arbitrary;
use fixed::types::I80F48;
use libfuzzer_sys::fuzz_target;
use marginfi::{
    prelude::MarginfiGroup,
    state::marginfi_group::{Bank, WrappedI80F48},
};
use marginfi_fuzz::{
    setup_marginfi_group, AccountIdx, AssetAmount, BankAndOracleConfig, BankIdx,
    MarginfiGroupAccounts, PriceChange, N_BANKS, N_USERS,
};
use solana_program::program_pack::Pack;

#[derive(Debug, Arbitrary)]
enum Action {
    Deposit {
        account: AccountIdx,
        bank: BankIdx,
        asset_amount: AssetAmount,
    },
    Withdraw {
        account: AccountIdx,
        bank: BankIdx,
        asset_amount: AssetAmount,
    },
    UpdateOracle {
        bank: BankIdx,
        price: PriceChange,
    },
    AccrueInterest {
        bank: BankIdx,
        time_delta: u8,
    },
    Liquidate {
        liquidator: AccountIdx,
        liquidatee: AccountIdx,
        asset_bank: BankIdx,
        liability_bank: BankIdx,
        asset_amount: AssetAmount,
    },
    HandleBankruptcy {
        account: AccountIdx,
        bank: BankIdx,
    },
}

#[derive(Debug)]
pub struct ActionSequence(Vec<Action>);

impl<'a> Arbitrary<'a> for ActionSequence {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let n_actions = u.int_in_range(0..=200)?;
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
    let bump = bumpalo::Bump::new();
    let mut mga = MarginfiGroupAccounts::setup(&bump);

    mga.setup_banks(&bump, Rent::free(), N_BANKS, &ctx.initial_bank_configs);
    mga.setup_users(&bump, Rent::free(), N_USERS as usize);

    let al =
        AccountLoader::<MarginfiGroup>::try_from_unchecked(&marginfi::id(), &mga.marginfi_group)
            .unwrap();

    assert_eq!(al.load()?.admin, mga.owner.key());

    for action in ctx.action_sequence.0.iter() {
        process_action(action, &mga)?;
    }

    verify_end_state(&mga)?;

    Ok(())
}

fn verify_end_state(mga: &MarginfiGroupAccounts) -> anyhow::Result<()> {
    mga.banks.iter().try_for_each(|bank| {
        let bank_loader = AccountLoader::<Bank>::try_from(&bank.bank)?;
        let bank_data = bank_loader.load()?;

        let total_deposits = bank_data.get_deposit_amount(bank_data.total_deposit_shares.into())?;
        let total_liabilities =
            bank_data.get_liability_amount(bank_data.total_liability_shares.into())?;

        let net_accounted_balance = total_deposits - total_liabilities;

        let liquidity_vault_token_account =
            spl_token::state::Account::unpack(&bank.liquidity_vault.data.borrow())?;

        assert_eq!(
            liquidity_vault_token_account.amount,
            net_accounted_balance.to_num::<u64>(),
        );

        // println!(
        //     "bank: {:?} total_deposits: {:?} total_liabilities: {:?} net_accounted_balance: {:?}",
        //     bank.bank.key(),
        //     total_deposits,
        //     total_liabilities,
        //     net_accounted_balance,
        // );

        Ok::<_, anyhow::Error>(())
    })?;

    Ok(())
}

fn process_action(action: &Action, mga: &MarginfiGroupAccounts) -> Result<()> {
    match action {
        Action::Deposit {
            account,
            bank,
            asset_amount,
        } => mga.process_action_deposits(account, bank, asset_amount)?,
        Action::Withdraw {
            account,
            bank,
            asset_amount,
        } => mga.process_action_withdraw(account, bank, asset_amount)?,
        Action::AccrueInterest { bank, time_delta } => {
            mga.process_accrue_interest(bank, *time_delta)?
        }
        _ => (),
    };

    Ok(())
}
