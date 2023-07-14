use crate::{marginfi_group::*, native, spl::*, utils::*};
use anchor_lang::prelude::*;
use bincode::deserialize;
use solana_sdk::account::AccountSharedData;

use super::marginfi_account::MarginfiAccountFixture;
use crate::bank::BankFixture;
use fixed_macro::types::I80F48;
use lazy_static::lazy_static;
use marginfi::state::marginfi_group::{BankConfigOpt, BankOperationalState};
use marginfi::{
    constants::MAX_ORACLE_KEYS,
    state::{
        marginfi_group::{BankConfig, GroupConfig, InterestRateConfig, RiskTier},
        price::OracleSetup,
    },
};
use solana_program::{hash::Hash, sysvar};
use solana_program_test::*;
use solana_sdk::{account::Account, pubkey, signature::Keypair, signer::Signer};
use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};

#[derive(Default, Debug, Clone)]
pub struct TestSettings {
    pub group_config: Option<GroupConfig>,
    pub banks: Vec<TestBankSetting>,
}

impl TestSettings {
    pub fn all_banks_payer_not_admin() -> Self {
        Self {
            banks: vec![
                TestBankSetting {
                    mint: BankMint::USDC,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SOL,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SolEquivalent,
                    ..TestBankSetting::default()
                },
            ],
            group_config: Some(GroupConfig { admin: None }),
        }
    }

    /// All banks with the same config, but USDC and SOL are using switchboard price oracls
    pub fn all_banks_swb_payer_not_admin() -> Self {
        Self {
            banks: vec![
                TestBankSetting {
                    mint: BankMint::USDC,
                    config: Some(*DEFAULT_USDC_TEST_SW_BANK_CONFIG),
                },
                TestBankSetting {
                    mint: BankMint::SOL,
                    config: Some(*DEFAULT_SOL_TEST_SW_BANK_CONFIG),
                },
            ],
            group_config: Some(GroupConfig { admin: None }),
        }
    }

    pub fn all_banks_one_isolated() -> Self {
        Self {
            banks: vec![
                TestBankSetting {
                    mint: BankMint::USDC,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SOL,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SolEquivalent,
                    config: Some(BankConfig {
                        risk_tier: RiskTier::Isolated,
                        asset_weight_maint: I80F48!(0).into(),
                        asset_weight_init: I80F48!(0).into(),
                        ..*DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG
                    }),
                },
            ],
            group_config: Some(GroupConfig { admin: None }),
        }
    }

    pub fn many_banks_10() -> Self {
        Self {
            banks: vec![
                TestBankSetting {
                    mint: BankMint::USDC,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SOL,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SolEquivalent,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SolEquivalent1,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SolEquivalent2,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SolEquivalent3,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SolEquivalent4,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SolEquivalent5,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SolEquivalent6,
                    ..TestBankSetting::default()
                },
                TestBankSetting {
                    mint: BankMint::SolEquivalent7,
                    ..TestBankSetting::default()
                },
            ],
            group_config: Some(GroupConfig { admin: None }),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct TestBankSetting {
    pub mint: BankMint,
    pub config: Option<BankConfig>,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum BankMint {
    USDC,
    SOL,
    SolEquivalent,
    SolEquivalent1,
    SolEquivalent2,
    SolEquivalent3,
    SolEquivalent4,
    SolEquivalent5,
    SolEquivalent6,
    SolEquivalent7,
    SolEquivalent8,
    SolEquivalent9,
}

impl Default for BankMint {
    fn default() -> Self {
        Self::USDC
    }
}

pub struct TestFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub marginfi_group: MarginfiGroupFixture,
    pub banks: HashMap<BankMint, BankFixture>,
    pub usdc_mint: MintFixture,
    pub sol_mint: MintFixture,
    pub sol_equivalent_mint: MintFixture,
    pub mnde_mint: MintFixture,
}

pub const PYTH_USDC_FEED: Pubkey = pubkey!("PythUsdcPrice111111111111111111111111111111");
pub const SWITCHBOARD_USDC_FEED: Pubkey = pubkey!("SwchUsdcPrice111111111111111111111111111111");
pub const PYTH_SOL_FEED: Pubkey = pubkey!("PythSo1Price1111111111111111111111111111111");
pub const SWITCHBOARD_SOL_FEED: Pubkey = pubkey!("SwchSo1Price1111111111111111111111111111111");
pub const PYTH_SOL_EQUIVALENT_FEED: Pubkey = pubkey!("PythSo1Equiva1entPrice111111111111111111111");
pub const PYTH_MNDE_FEED: Pubkey = pubkey!("PythMndePrice111111111111111111111111111111");
pub const FAKE_PYTH_USDC_FEED: Pubkey = pubkey!("FakePythUsdcPrice11111111111111111111111111");

pub fn create_oracle_key_array(pyth_oracle: Pubkey) -> [Pubkey; MAX_ORACLE_KEYS] {
    let mut keys = [Pubkey::default(); MAX_ORACLE_KEYS];
    keys[0] = pyth_oracle;

    keys
}

lazy_static! {
    pub static ref DEFAULT_TEST_BANK_INTEREST_RATE_CONFIG: InterestRateConfig =
        InterestRateConfig {
            insurance_fee_fixed_apr: I80F48!(0).into(),
            insurance_ir_fee: I80F48!(0).into(),
            protocol_ir_fee: I80F48!(0).into(),
            protocol_fixed_fee_apr: I80F48!(0).into(),

            optimal_utilization_rate: I80F48!(0.5).into(),
            plateau_interest_rate: I80F48!(0.6).into(),
            max_interest_rate: I80F48!(3).into(),
            ..Default::default()
        };
    pub static ref DEFAULT_TEST_BANK_CONFIG: BankConfig = BankConfig {
        oracle_setup: OracleSetup::PythEma,
        asset_weight_maint: I80F48!(1).into(),
        asset_weight_init: I80F48!(1).into(),
        liability_weight_init: I80F48!(1).into(),
        liability_weight_maint: I80F48!(1).into(),

        operational_state: BankOperationalState::Operational,
        risk_tier: RiskTier::Collateral,

        interest_rate_config: InterestRateConfig {
            insurance_fee_fixed_apr: I80F48!(0).into(),
            insurance_ir_fee: I80F48!(0).into(),
            protocol_ir_fee: I80F48!(0).into(),
            protocol_fixed_fee_apr: I80F48!(0).into(),

            optimal_utilization_rate: I80F48!(0.5).into(),
            plateau_interest_rate: I80F48!(0.6).into(),
            max_interest_rate: I80F48!(3).into(),
            ..Default::default()
        },
        ..Default::default()
    };
    pub static ref DEFAULT_USDC_TEST_BANK_CONFIG: BankConfig = BankConfig {
        deposit_limit: native!(1_000_000_000, "USDC"),
        borrow_limit: native!(1_000_000_000, "USDC"),
        oracle_keys: create_oracle_key_array(PYTH_USDC_FEED),
        ..*DEFAULT_TEST_BANK_CONFIG
    };
    pub static ref DEFAULT_SOL_TEST_BANK_CONFIG: BankConfig = BankConfig {
        deposit_limit: native!(1_000_000, "SOL"),
        borrow_limit: native!(1_000_000, "SOL"),
        oracle_keys: create_oracle_key_array(PYTH_SOL_FEED),
        ..*DEFAULT_TEST_BANK_CONFIG
    };
    pub static ref DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG: BankConfig = BankConfig {
        deposit_limit: native!(1_000_000, "SOL_EQ"),
        borrow_limit: native!(1_000_000, "SOL_EQ"),
        oracle_keys: create_oracle_key_array(PYTH_SOL_EQUIVALENT_FEED),
        ..*DEFAULT_TEST_BANK_CONFIG
    };
    pub static ref DEFAULT_MNDE_TEST_BANK_CONFIG: BankConfig = BankConfig {
        deposit_limit: native!(1_000_000, "MNDE"),
        borrow_limit: native!(1_000_000, "MNDE"),
        oracle_keys: create_oracle_key_array(PYTH_MNDE_FEED),
        ..*DEFAULT_TEST_BANK_CONFIG
    };
    pub static ref DEFAULT_USDC_TEST_SW_BANK_CONFIG: BankConfig = BankConfig {
        oracle_setup: OracleSetup::SwitchboardV2,
        deposit_limit: native!(1_000_000_000, "USDC"),
        borrow_limit: native!(1_000_000_000, "USDC"),
        oracle_keys: create_oracle_key_array(SWITCHBOARD_USDC_FEED),
        ..*DEFAULT_TEST_BANK_CONFIG
    };
    pub static ref DEFAULT_SOL_TEST_SW_BANK_CONFIG: BankConfig = BankConfig {
        oracle_setup: OracleSetup::SwitchboardV2,
        deposit_limit: native!(1_000_000, "SOL"),
        borrow_limit: native!(1_000_000, "SOL"),
        oracle_keys: create_oracle_key_array(SWITCHBOARD_SOL_FEED),
        ..*DEFAULT_TEST_BANK_CONFIG
    };
}

pub const USDC_MINT_DECIMALS: u8 = 6;
pub const SOL_MINT_DECIMALS: u8 = 9;
pub const MNDE_MINT_DECIMALS: u8 = 9;

impl TestFixture {
    pub async fn new(test_settings: Option<TestSettings>) -> TestFixture {
        let mut program = ProgramTest::new("marginfi", marginfi::ID, processor!(marginfi::entry));

        #[cfg(feature = "lip")]
        program.add_program(
            "liquidity_incentive_program",
            liquidity_incentive_program::ID,
            processor!(liquidity_incentive_program::entry),
        );

        let usdc_keypair = Keypair::new();
        let sol_keypair = Keypair::new();
        let sol_equivalent_keypair = Keypair::new();
        let mnde_keypair = Keypair::new();

        program.add_account(
            PYTH_USDC_FEED,
            create_pyth_price_account(usdc_keypair.pubkey(), 1, USDC_MINT_DECIMALS.into(), None),
        );
        program.add_account(
            PYTH_SOL_FEED,
            create_pyth_price_account(sol_keypair.pubkey(), 10, SOL_MINT_DECIMALS.into(), None),
        );
        program.add_account(
            PYTH_SOL_EQUIVALENT_FEED,
            create_pyth_price_account(
                sol_equivalent_keypair.pubkey(),
                10,
                SOL_MINT_DECIMALS.into(),
                None,
            ),
        );
        program.add_account(
            PYTH_MNDE_FEED,
            create_pyth_price_account(mnde_keypair.pubkey(), 10, MNDE_MINT_DECIMALS.into(), None),
        );

        program.add_account(
            SWITCHBOARD_USDC_FEED,
            create_switchboard_price_feed(1, USDC_MINT_DECIMALS.into()),
        );

        program.add_account(
            SWITCHBOARD_SOL_FEED,
            create_switchboard_price_feed(10, SOL_MINT_DECIMALS.into()),
        );

        let context = Rc::new(RefCell::new(program.start_with_context().await));

        {
            let mut ctx = context.borrow_mut();
            let mut clock: Clock = ctx.banks_client.get_sysvar().await.unwrap();
            clock.unix_timestamp = 0;
            ctx.set_sysvar(&clock);
        }

        solana_logger::setup_with_default(RUST_LOG_DEFAULT);

        let usdc_mint_f = MintFixture::new(
            Rc::clone(&context),
            Some(usdc_keypair),
            Some(USDC_MINT_DECIMALS),
        )
        .await;
        let sol_mint_f = MintFixture::new(
            Rc::clone(&context),
            Some(sol_keypair),
            Some(SOL_MINT_DECIMALS),
        )
        .await;
        let sol_equivalent_mint_f = MintFixture::new(
            Rc::clone(&context),
            Some(sol_equivalent_keypair),
            Some(SOL_MINT_DECIMALS),
        )
        .await;
        let mnde_mint_f = MintFixture::new(
            Rc::clone(&context),
            Some(mnde_keypair),
            Some(MNDE_MINT_DECIMALS),
        )
        .await;

        let tester_group = MarginfiGroupFixture::new(
            Rc::clone(&context),
            test_settings
                .clone()
                .map(|ts| ts.group_config.unwrap_or(GroupConfig { admin: None }))
                .unwrap_or(GroupConfig { admin: None }),
        )
        .await;

        let mut banks = HashMap::new();
        if let Some(test_settings) = test_settings.clone() {
            for bank in test_settings.banks.iter() {
                let (bank_mint, default_config) = match bank.mint {
                    BankMint::USDC => (&usdc_mint_f, *DEFAULT_USDC_TEST_BANK_CONFIG),
                    BankMint::SOL => (&sol_mint_f, *DEFAULT_SOL_TEST_BANK_CONFIG),
                    BankMint::SolEquivalent => (
                        &sol_equivalent_mint_f,
                        *DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG,
                    ),
                    BankMint::SolEquivalent1 => (
                        &sol_equivalent_mint_f,
                        *DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG,
                    ),
                    BankMint::SolEquivalent2 => (
                        &sol_equivalent_mint_f,
                        *DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG,
                    ),
                    BankMint::SolEquivalent3 => (
                        &sol_equivalent_mint_f,
                        *DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG,
                    ),
                    BankMint::SolEquivalent4 => (
                        &sol_equivalent_mint_f,
                        *DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG,
                    ),
                    BankMint::SolEquivalent5 => (
                        &sol_equivalent_mint_f,
                        *DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG,
                    ),
                    BankMint::SolEquivalent6 => (
                        &sol_equivalent_mint_f,
                        *DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG,
                    ),
                    BankMint::SolEquivalent7 => (
                        &sol_equivalent_mint_f,
                        *DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG,
                    ),
                    BankMint::SolEquivalent8 => (
                        &sol_equivalent_mint_f,
                        *DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG,
                    ),
                    BankMint::SolEquivalent9 => (
                        &sol_equivalent_mint_f,
                        *DEFAULT_SOL_EQUIVALENT_TEST_BANK_CONFIG,
                    ),
                };

                banks.insert(
                    bank.mint.clone(),
                    tester_group
                        .try_lending_pool_add_bank(bank_mint, bank.config.unwrap_or(default_config))
                        .await
                        .unwrap(),
                );
            }
        };

        TestFixture {
            context: Rc::clone(&context),
            marginfi_group: tester_group,
            banks,
            usdc_mint: usdc_mint_f,
            sol_mint: sol_mint_f,
            sol_equivalent_mint: sol_equivalent_mint_f,
            mnde_mint: mnde_mint_f,
        }
    }

    pub async fn create_marginfi_account(&self) -> MarginfiAccountFixture {
        MarginfiAccountFixture::new(Rc::clone(&self.context), &self.marginfi_group.key).await
    }

    pub async fn set_bank_operational_state(
        &self,
        bank_fixture: &BankFixture,
        state: BankOperationalState,
    ) -> anyhow::Result<(), BanksClientError> {
        self.marginfi_group
            .try_lending_pool_configure_bank(
                bank_fixture,
                BankConfigOpt {
                    operational_state: Some(state),
                    ..BankConfigOpt::default()
                },
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

    pub fn get_bank(&self, bank_mint: &BankMint) -> &BankFixture {
        self.banks.get(bank_mint).unwrap()
    }

    pub fn get_bank_mut(&mut self, bank_mint: &BankMint) -> &mut BankFixture {
        self.banks.get_mut(bank_mint).unwrap()
    }

    pub fn set_time(&self, timestamp: i64) {
        let clock = Clock {
            unix_timestamp: timestamp,
            ..Default::default()
        };
        self.context.borrow_mut().set_sysvar(&clock);
    }

    pub async fn set_pyth_oracle_timestamp(&self, address: Pubkey, timestamp: i64) {
        let mut ctx = self.context.borrow_mut();

        let mut account = ctx
            .banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap();

        let data = account.data.as_mut_slice();
        let mut data = *pyth_sdk_solana::state::load_price_account(data).unwrap();

        data.timestamp = timestamp;
        data.prev_timestamp = timestamp;

        let bytes = bytemuck::bytes_of(&data);

        let mut aso = AccountSharedData::from(account);

        aso.set_data_from_slice(bytes);

        ctx.set_account(&address, &aso);
    }

    pub async fn advance_time(&self, seconds: i64) {
        let mut clock: Clock = self
            .context
            .borrow_mut()
            .banks_client
            .get_sysvar()
            .await
            .unwrap();
        clock.unix_timestamp += seconds;
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
