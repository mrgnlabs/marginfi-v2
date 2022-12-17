#![cfg(feature = "test-bpf")]
#![allow(dead_code)]

use crate::{
    fixtures::{marginfi_group::*, spl::*, utils::*},
    native,
};
use anchor_lang::prelude::*;
use bincode::deserialize;

use lazy_static::lazy_static;
use marginfi::{
    constants::PYTH_ID,
    state::marginfi_group::{BankConfig, GroupConfig},
};
use solana_program::{hash::Hash, sysvar};
use solana_program_test::*;
use solana_sdk::{pubkey, signature::Keypair, signer::Signer};
use std::{cell::RefCell, rc::Rc};

use super::marginfi_account::MarginfiAccountFixture;

pub struct TestFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub marginfi_group: MarginfiGroupFixture,
    pub collateral_mint: MintFixture,
}

pub const PYTH_USDC_FEED: Pubkey = pubkey!("PythUsdcPrice111111111111111111111111111111");
pub const PYTH_SOL_FEED: Pubkey = pubkey!("PythSo1Price1111111111111111111111111111111");
pub const FAKE_PYTH_USDC_FEED: Pubkey = pubkey!("FakePythUsdcPrice11111111111111111111111111");

lazy_static! {
    pub static ref DEFAULT_TEST_BANK_CONFIG: BankConfig = BankConfig {
        pyth_oracle: PYTH_USDC_FEED,
        max_capacity: native!(1_000_000, "USDC"),
        ..BankConfig::default()
    };
}

impl TestFixture {
    pub async fn new(ix_arg: Option<GroupConfig>) -> TestFixture {
        let mut program = ProgramTest::new("marginfi", marginfi::ID, processor!(marginfi::entry));
        program.add_account_with_file_data(
            PYTH_USDC_FEED,
            1_000_000,
            PYTH_ID,
            "accounts/pyth_usdc_feed.bin",
        );
        program.add_account_with_file_data(
            PYTH_SOL_FEED,
            1_000_000,
            PYTH_ID,
            "accounts/pyth_sol_feed.bin",
        );
        let context = Rc::new(RefCell::new(program.start_with_context().await));
        solana_logger::setup_with_default(RUST_LOG_DEFAULT);

        let mint = MintFixture::new(Rc::clone(&context)).await;
        let tester_group = MarginfiGroupFixture::new(
            Rc::clone(&context),
            &mint.key,
            ix_arg.unwrap_or(GroupConfig { admin: None }),
        )
        .await;

        TestFixture {
            context: Rc::clone(&context),
            marginfi_group: tester_group,
            collateral_mint: mint,
        }
    }

    pub async fn create_marginfi_account(&self) -> MarginfiAccountFixture {
        let marfingi_account_f =
            MarginfiAccountFixture::new(Rc::clone(&self.context), &self.marginfi_group.key).await;

        marfingi_account_f
    }

    pub async fn load_and_deserialize<T: anchor_lang::AccountDeserialize>(
        &self,
        address: &Pubkey,
    ) -> T {
        let ai = self
            .context
            .borrow_mut()
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .unwrap();

        T::try_deserialize(&mut ai.data.as_slice()).unwrap()
    }

    pub fn payer(&self) -> Pubkey {
        self.context.borrow().payer.pubkey()
    }

    pub fn payer_keypair(&self) -> Keypair {
        clone_keypair(&self.context.borrow().payer)
    }

    pub fn set_time(&self, timestamp: i64) {
        let clock = sysvar::clock::Clock {
            unix_timestamp: timestamp,
            ..Default::default()
        };
        self.context.borrow_mut().set_sysvar(&clock);
    }

    pub async fn get_minimum_rent_for_size(&self, size: usize) -> u64 {
        self.context
            .borrow_mut()
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(size)
    }

    pub async fn get_latest_blockhash(&self) -> Hash {
        self.context
            .borrow_mut()
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap()
    }

    pub async fn get_slot(&self) -> u64 {
        self.context
            .borrow_mut()
            .banks_client
            .get_root_slot()
            .await
            .unwrap()
    }

    pub async fn get_clock(&self) -> Clock {
        deserialize::<Clock>(
            &self
                .context
                .borrow_mut()
                .banks_client
                .get_account(sysvar::clock::ID)
                .await
                .unwrap()
                .unwrap()
                .data,
        )
        .unwrap()
    }
}
