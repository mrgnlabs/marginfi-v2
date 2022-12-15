#![cfg(feature = "test-bpf")]
#![allow(dead_code)]

use crate::fixtures::{marginfi_group::*, spl::*, utils::*};
use anchor_lang::prelude::*;
use bincode::deserialize;

use marginfi::state::marginfi_group::GroupConfig;
use solana_program::sysvar;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use std::{cell::RefCell, rc::Rc};

pub struct TestFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub marginfi_group: MarginfiGroupFixture,
    pub collateral_mint: MintFixture,
}

impl TestFixture {
    pub async fn new(ix_arg: Option<GroupConfig>) -> TestFixture {
        let program = ProgramTest::new("marginfi", marginfi::ID, processor!(marginfi::entry));
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

    pub fn set_time(&self, timestamp: i64) {
        let clock = sysvar::clock::Clock {
            unix_timestamp: timestamp,
            ..Default::default()
        };
        self.context.borrow_mut().set_sysvar(&clock);
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
