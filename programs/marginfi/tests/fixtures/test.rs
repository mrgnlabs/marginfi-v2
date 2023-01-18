#![cfg(feature = "test-bpf")]
#![allow(dead_code)]

use crate::{
    fixtures::{marginfi_group::*, spl::*, utils::*},
    native,
};
use anchor_lang::prelude::*;
use bincode::deserialize;

use super::marginfi_account::MarginfiAccountFixture;
use fixed_macro::types::I80F48;
use lazy_static::lazy_static;
use marginfi::{
    constants::MAX_ORACLE_KEYS,
    state::marginfi_group::{BankConfig, GroupConfig, InterestRateConfig, OracleSetup},
};
use solana_program::{hash::Hash, sysvar};
use solana_program_test::*;
use solana_sdk::{account::Account, pubkey, signature::Keypair, signer::Signer};
use std::{cell::RefCell, rc::Rc};

pub struct TestFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub marginfi_group: MarginfiGroupFixture,
    pub usdc_mint: MintFixture,
    pub sol_mint: MintFixture,
    pub sol_equivalent_mint: MintFixture,
    pub mnde_mint: MintFixture,
}

pub const PYTH_USDC_FEED: Pubkey = pubkey!("PythUsdcPrice111111111111111111111111111111");
pub const PYTH_SOL_FEED: Pubkey = pubkey!("PythSo1Price1111111111111111111111111111111");
pub const PYTH_SOL_EQUIVALENT_FEED: Pubkey = pubkey!("PythSo1Equiva1entPrice111111111111111111111");
pub const PYTH_MNDE_FEED: Pubkey = pubkey!("PythMndePrice111111111111111111111111111111");
pub const FAKE_PYTH_USDC_FEED: Pubkey = pubkey!("FakePythUsdcPrice11111111111111111111111111");

pub fn create_oracle_key_array(pyth_oracle: Pubkey) -> [Pubkey; MAX_ORACLE_KEYS] {
    let mut keys = [Pubkey::default(); MAX_ORACLE_KEYS];
    keys[0] = pyth_oracle;

    keys
}

lazy_static! {
    pub static ref DEFAULT_CONFIG: BankConfig = BankConfig {
        oracle_setup: OracleSetup::Pyth,
        deposit_weight_maint: I80F48!(1).into(),
        deposit_weight_init: I80F48!(1).into(),
        liability_weight_init: I80F48!(1).into(),
        liability_weight_maint: I80F48!(1).into(),

        interest_rate_config: InterestRateConfig {
            insurance_fee_fixed_apr: I80F48!(0).into(),
            insurance_ir_fee: I80F48!(0).into(),
            protocol_ir_fee: I80F48!(0).into(),
            protocol_fixed_fee_apr: I80F48!(0).into(),

            optimal_utilization_rate: I80F48!(0.5).into(),
            plateau_interest_rate: I80F48!(0.6).into(),
            max_interest_rate: I80F48!(3).into(),
        },
        ..Default::default()
    };
    pub static ref DEFAULT_USDC_TEST_BANK_CONFIG: BankConfig = BankConfig {
        max_capacity: native!(1_000_000_000, "USDC"),
        oracle_keys: create_oracle_key_array(PYTH_USDC_FEED),
        ..DEFAULT_CONFIG.clone()
    };
    pub static ref DEFAULT_SOL_TEST_BANK_CONFIG: BankConfig = BankConfig {
        max_capacity: native!(1_000_000, "SOL"),
        oracle_keys: create_oracle_key_array(PYTH_SOL_FEED),
        ..DEFAULT_CONFIG.clone()
    };
    pub static ref DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG: BankConfig = BankConfig {
        max_capacity: native!(1_000_000, "SOL_EQ"),
        oracle_keys: create_oracle_key_array(PYTH_SOL_EQUIVALENT_FEED),
        ..DEFAULT_CONFIG.clone()
    };
    pub static ref DEFAULT_MNDE_TEST_BANK_CONFIG: BankConfig = BankConfig {
        max_capacity: native!(1_000_000, "MNDE"),
        oracle_keys: create_oracle_key_array(PYTH_MNDE_FEED),
        ..DEFAULT_CONFIG.clone()
    };
}

impl TestFixture {
    pub async fn new(ix_arg: Option<GroupConfig>) -> TestFixture {
        let mut program = ProgramTest::new("marginfi", marginfi::ID, processor!(marginfi::entry));

        let usdc_keypair = Keypair::new();
        let sol_keypair = Keypair::new();
        let sol_equivalent_keypair = Keypair::new();
        let mnde_keypair = Keypair::new();

        program.add_account(
            PYTH_USDC_FEED,
            craft_pyth_price_account(usdc_keypair.pubkey(), 1, 6),
        );
        program.add_account(
            PYTH_SOL_FEED,
            craft_pyth_price_account(sol_keypair.pubkey(), 10, 9),
        );
        program.add_account(
            PYTH_SOL_EQUIVALENT_FEED,
            craft_pyth_price_account(sol_equivalent_keypair.pubkey(), 10, 9),
        );
        program.add_account(
            PYTH_MNDE_FEED,
            craft_pyth_price_account(mnde_keypair.pubkey(), 10, 9),
        );

        let context = Rc::new(RefCell::new(program.start_with_context().await));

        {
            let mut ctx = context.borrow_mut();
            let mut clock: Clock = ctx.banks_client.get_sysvar().await.unwrap();
            clock.unix_timestamp = 0;
            ctx.set_sysvar(&clock);
        }

        solana_logger::setup_with_default(RUST_LOG_DEFAULT);

        let usdc_mint_f = MintFixture::new(Rc::clone(&context), Some(usdc_keypair), None).await;
        let sol_mint_f = MintFixture::new(Rc::clone(&context), Some(sol_keypair), Some(9)).await;
        let sol_equivalent_mint_f =
            MintFixture::new(Rc::clone(&context), Some(sol_equivalent_keypair), Some(9)).await;
        let mnde_mint_f = MintFixture::new(Rc::clone(&context), Some(mnde_keypair), Some(9)).await;

        let tester_group = MarginfiGroupFixture::new(
            Rc::clone(&context),
            ix_arg.unwrap_or(GroupConfig { admin: None }),
        )
        .await;

        TestFixture {
            context: Rc::clone(&context),
            marginfi_group: tester_group,
            usdc_mint: usdc_mint_f,
            sol_mint: sol_mint_f,
            sol_equivalent_mint: sol_equivalent_mint_f,
            mnde_mint: mnde_mint_f,
        }
    }

    pub async fn create_marginfi_account(&self) -> MarginfiAccountFixture {
        MarginfiAccountFixture::new(
            Rc::clone(&self.context),
            &self.marginfi_group.key,
            &self.usdc_mint.key,
            &self.sol_mint.key,
            &self.sol_equivalent_mint.key,
        )
        .await
    }

    pub async fn try_load(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<Option<Account>, BanksClientError> {
        self.context
            .borrow_mut()
            .banks_client
            .get_account(*address)
            .await
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
