use anchor_lang::{
    prelude::{
        Account, AccountInfo, AccountLoader, Clock, Context, Program, ProgramError, Pubkey, Rent,
        Signer, SolanaSysvar, Sysvar,
    },
    Discriminator, Key,
};
use arbitrary::Arbitrary;
use bumpalo::Bump;

use fixed_macro::types::I80F48;
use lazy_static::lazy_static;
use marginfi::{
    constants::PYTH_ID,
    prelude::MarginfiGroup,
    state::{
        marginfi_account::MarginfiAccount,
        marginfi_group::{Bank, BankConfig, BankVaultType, InterestRateConfig, WrappedI80F48},
    },
};
use pyth_sdk_solana::state::{
    AccountType, PriceAccount, PriceInfo, PriceStatus, Rational, MAGIC, VERSION_2,
};
use safe_transmute::{transmute_to_bytes, transmute_to_bytes_mut};
use solana_program::{
    bpf_loader,
    entrypoint::ProgramResult,
    instruction::Instruction,
    program_pack::Pack,
    program_stubs::{self},
    stake_history::Epoch,
    system_program, sysvar,
};
use spl_token::state::Mint;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    mem::size_of,
    ops::{Add, AddAssign},
    sync::RwLock,
    time::{SystemTime, UNIX_EPOCH},
};

type SplAccount = spl_token::state::Account;

#[derive(Default, Debug)]
pub struct Metrics {
    deposit_s: u64,
    deposit_e: u64,
    withdraw_s: u64,
    withdraw_e: u64,
    borrow_s: u64,
    borrow_e: u64,
    repay_s: u64,
    repay_e: u64,
    liquidate_e: u64,
    liquidate_s: u64,
    handle_bankruptcy_s: u64,
    handle_bankruptcy_e: u64,
    price_update: u64,
}

pub enum MetricAction {
    Deposit,
    Withdraw,
    Borrow,
    Repay,
    Liquidate,
    Bankruptcy,
}

impl Metrics {
    pub fn update_metric(&mut self, metric: MetricAction, success: bool) {
        let metric = match (metric, success) {
            (MetricAction::Deposit, true) => &mut self.deposit_s,
            (MetricAction::Deposit, false) => &mut self.deposit_e,
            (MetricAction::Withdraw, true) => &mut self.withdraw_s,
            (MetricAction::Withdraw, false) => &mut self.withdraw_e,
            (MetricAction::Borrow, true) => &mut self.borrow_s,
            (MetricAction::Borrow, false) => &mut self.borrow_e,
            (MetricAction::Repay, true) => &mut self.repay_s,
            (MetricAction::Repay, false) => &mut self.repay_e,
            (MetricAction::Liquidate, true) => &mut self.liquidate_s,
            (MetricAction::Liquidate, false) => &mut self.liquidate_e,
            (MetricAction::Bankruptcy, true) => &mut self.handle_bankruptcy_s,
            (MetricAction::Bankruptcy, false) => &mut self.handle_bankruptcy_e,
        };

        *metric += 1;
    }

    pub fn print(&self) {
        log!("\nDeposit\t{}\t{}\nWithd\t{}\t{}\nBorrow\t{}\t{}\nRepay\t{}\t{}\nLiq\t{}\t{}\nBank\t{}\t{}\nUpdate\t{}\n",
            self.deposit_s,
            self.deposit_e,
            self.withdraw_s,
            self.withdraw_e,
            self.borrow_s,
            self.borrow_e,
            self.repay_s,
            self.repay_e,
            self.liquidate_s,
            self.liquidate_e,
            self.handle_bankruptcy_s,
            self.handle_bankruptcy_e,
            self.price_update,
        );
    }
}

pub struct MarginfiGroupAccounts<'info> {
    pub marginfi_group: AccountInfo<'info>,
    pub banks: Vec<BankAccounts<'info>>,
    pub marginfi_accounts: Vec<UserAccount<'info>>,
    pub owner: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent_sysvar: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub last_sysvar_current_timestamp: RwLock<u64>,
    pub metrics: RwLock<Metrics>,
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        #[cfg(feature = "capture_log")]
        log::info!($($arg)*);
    }
}

impl<'bump> MarginfiGroupAccounts<'bump> {
    pub fn setup(bump: &'bump Bump, bank_configs: &[BankAndOracleConfig], n_users: usize) -> Self {
        let marginfi_program = new_marginfi_program(bump);
        let system_program = new_system_program(bump);
        let token_program = new_spl_token_program(bump);
        let admin = new_sol_account(1_000_000, bump);
        let rent_sysvar = new_rent_sysvar_account(0, Rent::free(), bump);
        let marginfi_group = initialize_marginfi_group(
            bump,
            marginfi_program.key,
            admin.clone(),
            system_program.clone(),
        );

        let mut mga = MarginfiGroupAccounts {
            marginfi_group,
            banks: vec![],
            owner: admin,
            system_program,
            rent_sysvar,
            token_program,
            marginfi_accounts: vec![],
            last_sysvar_current_timestamp: RwLock::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
            metrics: RwLock::new(Metrics::default()),
        };

        let banks = bank_configs
            .iter()
            .map(|config| mga.setup_bank(&bump, Rent::free(), config))
            .collect();

        mga.banks = banks;

        let token_vec = mga.banks.iter().map(|b| *b.mint.key).collect();

        mga.marginfi_accounts = (0..n_users)
            .into_iter()
            .map(|_| {
                mga.create_marginfi_account(bump, Rent::free(), &token_vec)
                    .unwrap()
            })
            .collect();

        // Fund initial banks to streamline deposits
        mga.banks.iter().enumerate().for_each(|(idx, _)| {
            mga.process_action_deposit(
                &AccountIdx((n_users - 1) as u8),
                &BankIdx(idx as u8),
                &AssetAmount(bank_configs[idx].deposit_limit / 2),
            )
            .unwrap();
        });

        mga
    }

    fn get_bank_map(&'bump self) -> HashMap<Pubkey, &'bump BankAccounts<'bump>> {
        HashMap::from_iter(
            self.banks
                .iter()
                .map(|bank| (bank.bank.key(), bank.clone())),
        )
    }

    fn refresh_oracle_accounts(&self) {
        self.banks.iter().for_each(|bank| {
            refresh_oracle_account(
                bank.oracle.clone(),
                self.last_sysvar_current_timestamp
                    .read()
                    .unwrap()
                    .to_owned() as i64,
            );
        });
    }

    pub fn advance_time(&self, time: u64) {
        self.last_sysvar_current_timestamp
            .write()
            .unwrap()
            .add_assign(time);
    }

    pub fn setup_bank(
        &self,
        bump: &'bump Bump,
        rent: Rent,
        initial_bank_config: &BankAndOracleConfig,
    ) -> BankAccounts<'bump> {
        let bank = new_owned_account(size_of::<Bank>(), &marginfi::ID, bump, rent);

        let mint = new_token_mint(bump, rent, initial_bank_config.mint_decimals);
        let (liquidity_vault_authority, liquidity_vault_authority_bump) =
            new_vault_authority(BankVaultType::Liquidity, bank.key, bump);
        let (liquidity_vault, liquidity_vault_bump) = new_vault_account(
            BankVaultType::Liquidity,
            mint.key,
            liquidity_vault_authority.key,
            bank.key,
            bump,
        );

        let (insurance_vault_authority, insurance_vault_authority_bump) =
            new_vault_authority(BankVaultType::Insurance, bank.key, bump);
        let (insurance_vault, insurance_vault_bump) = new_vault_account(
            BankVaultType::Insurance,
            mint.key,
            insurance_vault_authority.key,
            bank.key,
            bump,
        );

        let (fee_vault_authority, fee_vault_authority_bump) =
            new_vault_authority(BankVaultType::Fee, bank.key, bump);
        let (fee_vault, fee_vault_bump) = new_vault_account(
            BankVaultType::Fee,
            mint.key,
            fee_vault_authority.key,
            bank.key,
            bump,
        );

        let oracle = new_oracle_account(
            bump,
            rent,
            initial_bank_config.oracle_native_price as i64,
            *mint.key,
            initial_bank_config.mint_decimals as i32,
        );

        let mut seed_bump_map = BTreeMap::new();

        seed_bump_map.insert("liquidity_vault".to_owned(), liquidity_vault_bump);
        seed_bump_map.insert(
            "liquidity_vault_authority".to_owned(),
            liquidity_vault_authority_bump,
        );
        seed_bump_map.insert("insurance_vault".to_owned(), insurance_vault_bump);
        seed_bump_map.insert(
            "insurance_vault_authority".to_owned(),
            insurance_vault_authority_bump,
        );
        seed_bump_map.insert("fee_vault".to_owned(), fee_vault_bump);
        seed_bump_map.insert("fee_vault_authority".to_owned(), fee_vault_authority_bump);

        test_syscall_stubs(Some(
            *self.last_sysvar_current_timestamp.read().unwrap() as i64
        ));

        marginfi::instructions::marginfi_group::lending_pool_add_bank(
            Context::new(
                &marginfi::id(),
                &mut marginfi::instructions::LendingPoolAddBank {
                    marginfi_group: AccountLoader::try_from(&self.marginfi_group).unwrap(),
                    admin: Signer::try_from(&self.owner).unwrap(),
                    bank_mint: Box::new(Account::try_from(&mint).unwrap()),
                    bank: AccountLoader::try_from_unchecked(&marginfi::ID, &bank).unwrap(),
                    liquidity_vault_authority: liquidity_vault_authority.clone(),
                    liquidity_vault: Box::new(Account::try_from(&liquidity_vault).unwrap()),
                    insurance_vault_authority: insurance_vault_authority.clone(),
                    insurance_vault: Box::new(Account::try_from(&insurance_vault).unwrap()),
                    fee_vault_authority: fee_vault_authority.clone(),
                    fee_vault: Box::new(Account::try_from(&fee_vault).unwrap()),
                    rent: Sysvar::from_account_info(&self.rent_sysvar).unwrap(),
                    token_program: Program::try_from(&self.token_program).unwrap(),
                    system_program: Program::try_from(&self.system_program).unwrap(),
                },
                &[oracle.clone()],
                seed_bump_map,
            ),
            BankConfig {
                asset_weight_init: initial_bank_config.asset_weight_init,
                asset_weight_maint: initial_bank_config.asset_weight_maint,
                liability_weight_init: initial_bank_config.liability_weight_init,
                liability_weight_maint: initial_bank_config.liability_weight_maint,
                deposit_limit: initial_bank_config.deposit_limit,
                borrow_limit: initial_bank_config.borrow_limit,
                interest_rate_config: InterestRateConfig {
                    optimal_utilization_rate: I80F48!(0.5).into(),
                    plateau_interest_rate: I80F48!(0.5).into(),
                    max_interest_rate: I80F48!(4).into(),
                    insurance_fee_fixed_apr: I80F48!(0.01).into(),
                    insurance_ir_fee: I80F48!(0.05).into(),
                    protocol_fixed_fee_apr: I80F48!(0.01).into(),
                    protocol_ir_fee: I80F48!(0.1).into(),
                    ..Default::default()
                },
                oracle_setup: marginfi::state::marginfi_group::OracleSetup::Pyth,
                oracle_keys: [
                    oracle.key(),
                    Pubkey::default(),
                    Pubkey::default(),
                    Pubkey::default(),
                    Pubkey::default(),
                ],
                operational_state:
                    marginfi::state::marginfi_group::BankOperationalState::Operational,
                ..Default::default()
            },
        )
        .unwrap();

        set_discriminator::<Bank>(bank.clone());

        BankAccounts {
            bank,
            oracle,
            liquidity_vault,
            insurance_vault,
            fee_vault,
            mint,
            liquidity_vault_authority,
            insurance_vault_authority,
            fee_vault_authority,
            mint_decimals: initial_bank_config.mint_decimals,
        }
    }

    fn create_marginfi_account(
        &self,
        bump: &'bump Bump,
        rent: Rent,
        token_mints: &Vec<Pubkey>,
    ) -> anyhow::Result<UserAccount<'bump>> {
        let marginfi_account =
            new_owned_account(size_of::<MarginfiAccount>(), &marginfi::ID, bump, rent);

        marginfi::instructions::marginfi_account::initialize(Context::new(
            &marginfi::id(),
            &mut marginfi::instructions::marginfi_account::MarginfiAccountInitialize {
                marginfi_group: AccountLoader::try_from(&self.marginfi_group.clone())?,
                marginfi_account: AccountLoader::try_from_unchecked(
                    &marginfi::ID,
                    &marginfi_account,
                )?,
                authority: Signer::try_from(&self.owner)?,
                fee_payer: Signer::try_from(&self.owner)?,
                system_program: Program::try_from(&self.system_program)?,
            },
            &[],
            BTreeMap::new(),
        ))
        .unwrap();

        let token_accounts = token_mints
            .iter()
            .map(|token| {
                new_token_account(token, self.owner.key, 100_000_000_000_000_000, bump, rent)
            })
            .collect();

        set_discriminator::<MarginfiAccount>(marginfi_account.clone());

        Ok(UserAccount::new(marginfi_account, token_accounts))
    }

    pub fn process_action_deposit(
        &self,
        account_idx: &AccountIdx,
        bank_idx: &BankIdx,
        asset_amount: &AssetAmount,
    ) -> anyhow::Result<()> {
        let marginfi_account = &self.marginfi_accounts[account_idx.0 as usize];
        let bank = &self.banks[bank_idx.0 as usize];

        let cache = AccountInfoCache::new(&[
            marginfi_account.margin_account.clone(),
            bank.bank.clone(),
            marginfi_account.token_accounts[bank_idx.0 as usize].clone(),
            bank.liquidity_vault.clone(),
        ]);

        let res = marginfi::instructions::marginfi_account::lending_account_deposit(
            Context::new(
                &marginfi::ID,
                &mut marginfi::instructions::LendingAccountDeposit {
                    marginfi_group: AccountLoader::try_from(&self.marginfi_group.clone())?,
                    marginfi_account: AccountLoader::try_from(&marginfi_account.margin_account)?,
                    signer: Signer::try_from(&self.owner)?,
                    bank: AccountLoader::try_from(&bank.bank)?,
                    signer_token_account: marginfi_account.token_accounts[bank_idx.0 as usize]
                        .clone(),
                    bank_liquidity_vault: bank.liquidity_vault.clone(),
                    token_program: Program::try_from(&self.token_program)?,
                },
                &[],
                BTreeMap::new(),
            ),
            asset_amount.0,
        );

        if res.is_err() {
            cache.revert();
        }

        self.metrics
            .write()
            .unwrap()
            .update_metric(MetricAction::Deposit, res.is_ok());

        Ok(())
    }

    pub fn process_action_repay(
        &self,
        account_idx: &AccountIdx,
        bank_idx: &BankIdx,
        asset_amount: &AssetAmount,
        repay_all: bool,
    ) -> anyhow::Result<()> {
        let marginfi_account = &self.marginfi_accounts[account_idx.0 as usize];
        let bank = &self.banks[bank_idx.0 as usize];

        let cache = AccountInfoCache::new(&[
            marginfi_account.margin_account.clone(),
            bank.bank.clone(),
            marginfi_account.token_accounts[bank_idx.0 as usize].clone(),
            bank.liquidity_vault.clone(),
        ]);

        let res = marginfi::instructions::marginfi_account::lending_account_repay(
            Context::new(
                &marginfi::ID,
                &mut marginfi::instructions::LendingAccountRepay {
                    marginfi_group: AccountLoader::try_from(&self.marginfi_group.clone())?,
                    marginfi_account: AccountLoader::try_from(&marginfi_account.margin_account)?,
                    signer: Signer::try_from(&self.owner)?,
                    bank: AccountLoader::try_from(&bank.bank)?,
                    signer_token_account: marginfi_account.token_accounts[bank_idx.0 as usize]
                        .clone(),
                    bank_liquidity_vault: bank.liquidity_vault.clone(),
                    token_program: Program::try_from(&self.token_program)?,
                },
                &[],
                BTreeMap::new(),
            ),
            asset_amount.0,
            Some(repay_all),
        );

        if res.is_err() {
            cache.revert();
        }

        self.metrics
            .write()
            .unwrap()
            .update_metric(MetricAction::Repay, res.is_ok());

        Ok(())
    }

    pub fn process_action_withdraw(
        &'bump self,
        account_idx: &AccountIdx,
        bank_idx: &BankIdx,
        asset_amount: &AssetAmount,
        withdraw_all: Option<bool>,
    ) -> anyhow::Result<()> {
        self.refresh_oracle_accounts();
        let marginfi_account = &self.marginfi_accounts[account_idx.0 as usize];
        let bank = &self.banks[bank_idx.0 as usize];

        let cache = AccountInfoCache::new(&[
            marginfi_account.margin_account.clone(),
            bank.bank.clone(),
            marginfi_account.token_accounts[bank_idx.0 as usize].clone(),
            bank.liquidity_vault.clone(),
        ]);

        let remove_all_bank = if let Some(withdraw_all) = withdraw_all {
            if withdraw_all {
                vec![bank.bank.key()]
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let res = marginfi::instructions::marginfi_account::lending_account_withdraw(
            Context::new(
                &marginfi::ID,
                &mut marginfi::instructions::LendingAccountWithdraw {
                    marginfi_group: AccountLoader::try_from(&self.marginfi_group.clone())?,
                    marginfi_account: AccountLoader::try_from(&marginfi_account.margin_account)?,
                    signer: Signer::try_from(&self.owner)?,
                    bank: AccountLoader::try_from(&bank.bank)?,
                    token_program: Program::try_from(&self.token_program)?,
                    destination_token_account: Account::try_from(
                        &marginfi_account.token_accounts[bank_idx.0 as usize].clone(),
                    )?,
                    bank_liquidity_vault_authority: bank.liquidity_vault_authority.clone(),
                    bank_liquidity_vault: Account::try_from(&bank.liquidity_vault)?,
                },
                &marginfi_account.get_remaining_accounts(
                    &self.get_bank_map(),
                    vec![],
                    remove_all_bank,
                ),
                BTreeMap::new(),
            ),
            asset_amount.0,
            withdraw_all,
        );

        if res.is_err() {
            cache.revert();
        }

        self.metrics
            .write()
            .unwrap()
            .update_metric(MetricAction::Withdraw, res.is_ok());

        Ok(())
    }

    pub fn process_action_borrow(
        &'bump self,
        account_idx: &AccountIdx,
        bank_idx: &BankIdx,
        asset_amount: &AssetAmount,
    ) -> anyhow::Result<()> {
        self.refresh_oracle_accounts();
        let marginfi_account = &self.marginfi_accounts[account_idx.0 as usize];
        let bank = &self.banks[bank_idx.0 as usize];

        let cache = AccountInfoCache::new(&[
            marginfi_account.margin_account.clone(),
            bank.bank.clone(),
            marginfi_account.token_accounts[bank_idx.0 as usize].clone(),
            bank.liquidity_vault.clone(),
        ]);

        let res = marginfi::instructions::marginfi_account::lending_account_borrow(
            Context::new(
                &marginfi::ID,
                &mut marginfi::instructions::LendingAccountBorrow {
                    marginfi_group: AccountLoader::try_from(&self.marginfi_group.clone())?,
                    marginfi_account: AccountLoader::try_from(&marginfi_account.margin_account)?,
                    signer: Signer::try_from(&self.owner)?,
                    bank: AccountLoader::try_from(&bank.bank)?,
                    token_program: Program::try_from(&self.token_program)?,
                    destination_token_account: Account::try_from(
                        &marginfi_account.token_accounts[bank_idx.0 as usize].clone(),
                    )?,
                    bank_liquidity_vault_authority: bank.liquidity_vault_authority.clone(),
                    bank_liquidity_vault: Account::try_from(&bank.liquidity_vault)?,
                },
                &marginfi_account.get_remaining_accounts(
                    &self.get_bank_map(),
                    vec![bank.bank.key()],
                    vec![],
                ),
                BTreeMap::new(),
            ),
            asset_amount.0,
        );

        if res.is_err() {
            cache.revert();
        }

        self.metrics
            .write()
            .unwrap()
            .update_metric(MetricAction::Borrow, res.is_ok());

        Ok(())
    }

    pub fn process_liquidate_account(
        &'bump self,
        liquidator_idx: &AccountIdx,
        liquidatee_idx: &AccountIdx,
        asset_bank_idx: &BankIdx,
        liab_bank_idx: &BankIdx,
        asset_amount: &AssetAmount,
    ) -> anyhow::Result<()> {
        self.refresh_oracle_accounts();
        let liquidator_account = &self.marginfi_accounts[liquidator_idx.0 as usize];
        let liquidatee_account = &self.marginfi_accounts[liquidatee_idx.0 as usize];
        let asset_bank = &self.banks[asset_bank_idx.0 as usize];
        let liab_bank = &self.banks[liab_bank_idx.0 as usize];

        let account_cache = AccountInfoCache::new(&[
            liquidator_account.margin_account.clone(),
            liquidatee_account.margin_account.clone(),
            asset_bank.bank.clone(),
            asset_bank.liquidity_vault.clone(),
            liab_bank.bank.clone(),
            liab_bank.liquidity_vault.clone(),
            liab_bank.insurance_vault.clone(),
        ]);

        let mut remaining_accounts = vec![asset_bank.oracle.clone(), liab_bank.oracle.clone()];

        let mut liquidator_remaining_accounts = liquidator_account.get_remaining_accounts(
            &self.get_bank_map(),
            vec![asset_bank.bank.key(), liab_bank.bank.key()],
            vec![],
        );
        let mut liquidatee_remaining_accounts =
            liquidatee_account.get_remaining_accounts(&self.get_bank_map(), vec![], vec![]);

        remaining_accounts.append(&mut liquidator_remaining_accounts);
        remaining_accounts.append(&mut liquidatee_remaining_accounts);

        let res = marginfi::instructions::lending_account_liquidate(
            Context::new(
                &marginfi::id(),
                &mut marginfi::instructions::LendingAccountLiquidate {
                    marginfi_group: AccountLoader::try_from(&self.marginfi_group.clone())?,
                    asset_bank: AccountLoader::try_from(&asset_bank.bank.clone())?,
                    liab_bank: AccountLoader::try_from(&liab_bank.bank.clone())?,
                    liquidator_marginfi_account: AccountLoader::try_from(
                        &liquidator_account.margin_account.clone(),
                    )?,
                    signer: Signer::try_from(&self.owner)?,
                    liquidatee_marginfi_account: AccountLoader::try_from(
                        &liquidatee_account.margin_account.clone(),
                    )?,
                    bank_liquidity_vault_authority: liab_bank.liquidity_vault_authority.clone(),
                    bank_liquidity_vault: Box::new(Account::try_from(
                        &liab_bank.liquidity_vault.clone(),
                    )?),
                    bank_insurance_vault: liab_bank.insurance_vault.clone(),
                    token_program: Program::try_from(&self.token_program)?,
                },
                &remaining_accounts,
                BTreeMap::new(),
            ),
            asset_amount.0,
        );

        if res.is_err() {
            account_cache.revert();
        }

        if res.is_ok() {
            self.process_handle_bankruptcy(liquidatee_idx, liab_bank_idx)?;
        }

        self.metrics
            .write()
            .unwrap()
            .update_metric(MetricAction::Liquidate, res.is_ok());

        Ok(())
    }

    pub fn process_handle_bankruptcy(
        &'bump self,
        account_idx: &AccountIdx,
        bank_idx: &BankIdx,
    ) -> anyhow::Result<()> {
        let marginfi_account = &self.marginfi_accounts[account_idx.0 as usize];
        let bank = &self.banks[bank_idx.0 as usize];

        let cache = AccountInfoCache::new(&[
            bank.bank.clone(),
            marginfi_account.margin_account.clone(),
            bank.liquidity_vault.clone(),
            bank.insurance_vault.clone(),
        ]);

        let res = marginfi::instructions::lending_pool_handle_bankruptcy(Context::new(
            &marginfi::ID,
            &mut marginfi::instructions::LendingPoolHandleBankruptcy {
                marginfi_group: AccountLoader::try_from(&self.marginfi_group.clone())?,
                admin: Signer::try_from(&self.owner)?,
                bank: AccountLoader::try_from(&bank.bank.clone())?,
                marginfi_account: AccountLoader::try_from(
                    &marginfi_account.margin_account.clone(),
                )?,
                liquidity_vault: bank.liquidity_vault.clone(),
                insurance_vault: Box::new(Account::try_from(&bank.insurance_vault.clone())?),
                insurance_vault_authority: bank.insurance_vault_authority.clone(),
                token_program: Program::try_from(&self.token_program)?,
            },
            &marginfi_account.get_remaining_accounts(&self.get_bank_map(), vec![], vec![]),
            BTreeMap::new(),
        ));

        if res.is_err() {
            cache.revert();
        }

        self.metrics
            .write()
            .unwrap()
            .update_metric(MetricAction::Bankruptcy, res.is_ok());

        Ok(())
    }

    pub fn process_update_oracle(
        &self,
        bank_idx: &BankIdx,
        price_change: &PriceChange,
    ) -> anyhow::Result<()> {
        let bank = &self.banks[bank_idx.0 as usize];
        let price_oracle = bank.oracle.clone();

        update_oracle_account(
            price_oracle,
            price_change.0 * 10_i64.pow(bank.mint_decimals as u32),
        )?;

        self.metrics.write().unwrap().price_update += 1;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PriceChange(i64);

impl<'a> Arbitrary<'a> for PriceChange {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self(u.int_in_range(-10..=10)? * 1_000_000))
    }
}

struct AccountInfoCache<'bump> {
    account_data: Vec<Vec<u8>>,
    account_info: Vec<AccountInfo<'bump>>,
}

impl<'info> AccountInfoCache<'info> {
    pub fn new(ais: &[AccountInfo<'info>]) -> Self {
        let account_data = ais.iter().map(|ai| ai.data.borrow().to_owned()).collect();
        Self {
            account_data,
            account_info: ais.to_vec(),
        }
    }

    pub fn revert(&self) {
        for (ai, data) in self.account_info.iter().zip(self.account_data.iter()) {
            ai.data.borrow_mut().copy_from_slice(data);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountIdx(pub u8);
pub const N_USERS: u8 = 4;
impl<'a> Arbitrary<'a> for AccountIdx {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let i: u8 = u.int_in_range(0..=N_USERS - 1)?;
        Ok(AccountIdx(i))
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (1, Some(1))
    }

    fn arbitrary_take_rest(mut u: arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Self::arbitrary(&mut u)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BankIdx(pub u8);
pub const N_BANKS: usize = 4;
impl<'a> Arbitrary<'a> for BankIdx {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(BankIdx(u.int_in_range(0..=N_BANKS - 1)? as u8))
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (1, Some(1))
    }

    fn arbitrary_take_rest(mut u: arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Self::arbitrary(&mut u)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetAmount(pub u64);

pub const MAX_ASSET_AMOUNT: u64 = 100_000;
impl<'a> Arbitrary<'a> for AssetAmount {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(AssetAmount(
            1_000 + u.int_in_range(1..=10)? * MAX_ASSET_AMOUNT,
        ))
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (8, Some(8))
    }

    fn arbitrary_take_rest(mut u: arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Self::arbitrary(&mut u)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BankAndOracleConfig {
    pub oracle_native_price: u64,
    pub mint_decimals: u8,

    pub asset_weight_init: WrappedI80F48,
    pub asset_weight_maint: WrappedI80F48,

    pub liability_weight_init: WrappedI80F48,
    pub liability_weight_maint: WrappedI80F48,

    pub deposit_limit: u64,
    pub borrow_limit: u64,
}

impl<'a> Arbitrary<'a> for BankAndOracleConfig {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mint_decimals = u.int_in_range(1..=3)? * 3;
        let top_limit = 1_000_000 * 10u64.pow(mint_decimals as u32);
        let borrow_limit = u.int_in_range(1..=10)? * top_limit;
        let deposit_limit = borrow_limit + u.int_in_range(1..=10)? * top_limit;

        let max_price = 100 * 10u64.pow(mint_decimals as u32);

        Ok(Self {
            oracle_native_price: u.int_in_range(1..=10)? * max_price,
            mint_decimals,
            asset_weight_init: I80F48!(0.5).into(),
            asset_weight_maint: I80F48!(0.75).into(),
            liability_weight_init: I80F48!(1.5).into(),
            liability_weight_maint: I80F48!(1.25).into(),
            deposit_limit,
            borrow_limit,
        })
    }
}

impl BankAndOracleConfig {
    pub fn dummy() -> Self {
        Self {
            oracle_native_price: 10 * 10u64.pow(6),
            mint_decimals: 6,
            asset_weight_init: I80F48!(0.75).into(),
            asset_weight_maint: I80F48!(0.8).into(),
            liability_weight_init: I80F48!(1.2).into(),
            liability_weight_maint: I80F48!(1.1).into(),
            deposit_limit: 1_000_000_000_000 * 10u64.pow(6),
            borrow_limit: 1_000_000_000_000 * 10u64.pow(6),
        }
    }
}

pub fn new_token_mint(bump: &Bump, rent: Rent, decimals: u8) -> AccountInfo {
    let data = bump.alloc_slice_fill_copy(Mint::LEN, 0u8);
    let mut mint = Mint::default();
    mint.is_initialized = true;
    mint.decimals = decimals;
    Mint::pack(mint, data).unwrap();
    AccountInfo::new(
        random_pubkey(bump),
        false,
        true,
        bump.alloc(rent.minimum_balance(data.len())),
        data,
        &spl_token::ID,
        false,
        Epoch::default(),
    )
}

pub fn new_token_account<'bump, 'a, 'b>(
    mint_pubkey: &'a Pubkey,
    owner_pubkey: &'b Pubkey,
    balance: u64,
    bump: &'bump Bump,
    rent: Rent,
) -> AccountInfo<'bump> {
    let data = bump.alloc_slice_fill_copy(SplAccount::LEN, 0u8);
    let mut account = SplAccount::default();
    account.state = spl_token::state::AccountState::Initialized;
    account.mint = *mint_pubkey;
    account.owner = *owner_pubkey;
    account.amount = balance;
    SplAccount::pack(account, data).unwrap();
    AccountInfo::new(
        random_pubkey(bump),
        false,
        true,
        bump.alloc(rent.minimum_balance(data.len())),
        data,
        &spl_token::ID,
        false,
        Epoch::default(),
    )
}

pub fn new_token_account_with_pubkey<'bump, 'a, 'b>(
    account_pubkey: Pubkey,
    mint_pubkey: &'a Pubkey,
    owner_pubkey: &'b Pubkey,
    balance: u64,
    bump: &'bump Bump,
    rent: Rent,
) -> AccountInfo<'bump> {
    let data = bump.alloc_slice_fill_copy(SplAccount::LEN, 0u8);
    let mut account = SplAccount::default();
    account.state = spl_token::state::AccountState::Initialized;
    account.mint = *mint_pubkey;
    account.owner = *owner_pubkey;
    account.amount = balance;
    SplAccount::pack(account, data).unwrap();
    AccountInfo::new(
        bump.alloc(account_pubkey),
        false,
        true,
        bump.alloc(rent.minimum_balance(data.len())),
        data,
        &spl_token::ID,
        false,
        Epoch::default(),
    )
}

pub fn get_vault_address(bank: &Pubkey, vault_type: BankVaultType) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[vault_type.get_seed(), &bank.to_bytes()], &marginfi::ID)
}

pub fn get_vault_authority(bank: &Pubkey, vault_type: BankVaultType) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[vault_type.get_authority_seed(), &bank.to_bytes()],
        &marginfi::ID,
    )
}

pub struct UserAccount<'info> {
    pub margin_account: AccountInfo<'info>,
    pub token_accounts: Vec<AccountInfo<'info>>,
}

impl<'info> UserAccount<'info> {
    pub fn new(
        margin_account: AccountInfo<'info>,
        token_accounts: Vec<AccountInfo<'info>>,
    ) -> Self {
        Self {
            margin_account,
            token_accounts,
        }
    }

    pub fn get_remaining_accounts(
        &self,
        bank_map: &HashMap<Pubkey, &BankAccounts<'info>>,
        include_banks: Vec<Pubkey>,
        exclude_banks: Vec<Pubkey>,
    ) -> Vec<AccountInfo<'info>> {
        let marginfi_account_al =
            AccountLoader::<MarginfiAccount>::try_from(&self.margin_account).unwrap();
        let marginfi_account = marginfi_account_al.load().unwrap();

        let mut already_included_banks = HashSet::new();

        let mut ais = marginfi_account
            .lending_account
            .balances
            .iter()
            .filter(|a| a.active && !exclude_banks.contains(&a.bank_pk))
            .map(|balance| {
                let bank_accounts = bank_map.get(&balance.bank_pk).unwrap();

                already_included_banks.insert(bank_accounts.bank.key());

                vec![bank_accounts.bank.clone(), bank_accounts.oracle.clone()]
            })
            .flatten()
            .collect::<Vec<_>>();

        let missing_banks = include_banks
            .iter()
            .filter(|key| !already_included_banks.contains(key))
            .collect::<Vec<_>>();

        let mut missing_bank_ais = missing_banks
            .iter()
            .map(|key| {
                let bank_accounts = bank_map.get(key).unwrap();
                vec![bank_accounts.bank.clone(), bank_accounts.oracle.clone()]
            })
            .flatten()
            .collect::<Vec<AccountInfo>>();

        ais.append(&mut missing_bank_ais);

        ais
    }
}

pub struct BankAccounts<'info> {
    pub bank: AccountInfo<'info>,
    pub oracle: AccountInfo<'info>,
    pub liquidity_vault: AccountInfo<'info>,
    pub liquidity_vault_authority: AccountInfo<'info>,
    pub insurance_vault: AccountInfo<'info>,
    pub insurance_vault_authority: AccountInfo<'info>,
    pub fee_vault: AccountInfo<'info>,
    pub fee_vault_authority: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub mint_decimals: u8,
}

fn random_pubkey(bump: &Bump) -> &Pubkey {
    bump.alloc(Pubkey::new(transmute_to_bytes(&rand::random::<[u64; 4]>())))
}

fn allocate_dex_owned_account(unpadded_size: usize, bump: &Bump) -> &mut [u8] {
    assert_eq!(unpadded_size % 8, 0);
    let padded_size = unpadded_size + 12;
    let u64_data = bump.alloc_slice_fill_copy(padded_size / 8 + 1, 0u64);
    let data = transmute_to_bytes_mut(u64_data); //[3..padded_size + 3];

    data
}

pub fn new_sol_account(lamports: u64, bump: &Bump) -> AccountInfo {
    new_sol_account_with_pubkey(random_pubkey(bump), lamports, bump)
}

pub fn new_sol_account_with_pubkey<'bump>(
    pubkey: &'bump Pubkey,
    lamports: u64,
    bump: &'bump Bump,
) -> AccountInfo<'bump> {
    AccountInfo::new(
        pubkey,
        true,
        false,
        bump.alloc(lamports),
        &mut [],
        &system_program::ID,
        false,
        Epoch::default(),
    )
}

pub fn new_vault_account<'bump>(
    vault_type: BankVaultType,
    mint_pubkey: &'bump Pubkey,
    owner: &'bump Pubkey,
    bank: &'bump Pubkey,
    bump: &'bump Bump,
) -> (AccountInfo<'bump>, u8) {
    let (vault_address, seed_bump) = get_vault_address(bank, vault_type);

    (
        new_token_account_with_pubkey(vault_address, mint_pubkey, owner, 0, bump, Rent::free()),
        seed_bump,
    )
}

pub fn new_vault_authority<'bump>(
    vault_type: BankVaultType,
    bank: &'bump Pubkey,
    bump: &'bump Bump,
) -> (AccountInfo<'bump>, u8) {
    let (vault_address, seed_bump) = get_vault_authority(bank, vault_type);

    (
        AccountInfo::new(
            bump.alloc(vault_address),
            false,
            false,
            bump.alloc(0),
            &mut [],
            &system_program::ID,
            false,
            Epoch::default(),
        ),
        seed_bump,
    )
}

pub fn new_owned_account<'bump>(
    unpadded_len: usize,
    program_id: &'bump Pubkey,
    bump: &'bump Bump,
    rent: Rent,
) -> AccountInfo<'bump> {
    let data_len = unpadded_len + 12;
    new_dex_owned_account_with_lamports(
        unpadded_len,
        rent.minimum_balance(data_len),
        program_id,
        bump,
    )
}

pub fn new_dex_owned_account_with_lamports<'bump>(
    unpadded_len: usize,
    lamports: u64,
    program_id: &'bump Pubkey,
    bump: &'bump Bump,
) -> AccountInfo<'bump> {
    AccountInfo::new(
        random_pubkey(bump),
        false,
        true,
        bump.alloc(lamports),
        allocate_dex_owned_account(unpadded_len, bump),
        program_id,
        false,
        Epoch::default(),
    )
}

pub fn new_spl_token_program(bump: &Bump) -> AccountInfo {
    AccountInfo::new(
        &spl_token::ID,
        false,
        false,
        bump.alloc(0),
        &mut [],
        &bpf_loader::ID,
        true,
        Epoch::default(),
    )
}

pub fn new_system_program(bump: &Bump) -> AccountInfo {
    AccountInfo::new(
        &system_program::ID,
        false,
        false,
        bump.alloc(0),
        &mut [],
        &bpf_loader::ID,
        true,
        Epoch::default(),
    )
}

pub fn new_marginfi_program(bump: &Bump) -> AccountInfo {
    AccountInfo::new(
        &marginfi::ID,
        false,
        false,
        bump.alloc(0),
        &mut [],
        &bpf_loader::ID,
        true,
        Epoch::default(),
    )
}

pub fn new_oracle_account(
    bump: &Bump,
    rent: Rent,
    native_price: i64,
    mint: Pubkey,
    mint_decimals: i32,
) -> AccountInfo {
    let price_account = PriceAccount {
        prod: mint,
        agg: PriceInfo {
            conf: 0,
            price: native_price,
            status: PriceStatus::Trading,
            ..Default::default()
        },
        expo: -mint_decimals,
        prev_price: native_price,
        magic: MAGIC,
        ver: VERSION_2,
        atype: AccountType::Price as u32,
        timestamp: 0,
        ema_price: Rational {
            val: native_price,
            numer: native_price,
            denom: 1,
        },
        ..Default::default()
    };

    let price_data = bytemuck::bytes_of(&price_account);

    let rent_amount = rent.minimum_balance(price_data.len());

    let data = bump.alloc_slice_fill_copy(size_of::<PriceAccount>(), 0);

    data.clone_from_slice(price_data);

    AccountInfo::new(
        random_pubkey(bump),
        false,
        true,
        bump.alloc(rent_amount),
        data,
        &PYTH_ID,
        false,
        Epoch::default(),
    )
}

pub fn update_oracle_account(ai: AccountInfo, price_change: i64) -> Result<(), ProgramError> {
    let mut data = ai.try_borrow_mut_data()?;
    let data = bytemuck::from_bytes_mut::<PriceAccount>(&mut data);

    data.agg.price += price_change;
    data.ema_price.val += price_change;
    data.ema_price.numer += price_change;

    Ok(())
}

pub fn refresh_oracle_account(ai: AccountInfo, timestamp: i64) -> Result<(), ProgramError> {
    let mut data = ai.try_borrow_mut_data()?;
    let data = bytemuck::from_bytes_mut::<PriceAccount>(&mut data);

    data.timestamp = timestamp;

    Ok(())
}

pub fn set_discriminator<T: Discriminator>(ai: AccountInfo) {
    let mut data = ai.try_borrow_mut_data().unwrap();

    if data[..8].ne(&[0u8; 8]) {
        panic!("Account discriminator is already set");
    }

    data[..8].copy_from_slice(&T::DISCRIMINATOR);
}

fn new_rent_sysvar_account(lamports: u64, rent: Rent, bump: &Bump) -> AccountInfo {
    let data = bump.alloc_slice_fill_copy(size_of::<Rent>(), 0u8);
    let mut account_info = AccountInfo::new(
        &sysvar::rent::ID,
        false,
        false,
        bump.alloc(lamports),
        data,
        &sysvar::ID,
        false,
        Epoch::default(),
    );
    rent.to_account_info(&mut account_info).unwrap();
    account_info
}

lazy_static! {
    static ref VERBOSE: u32 = std::env::var("FUZZ_VERBOSE")
        .map(|s| s.parse())
        .ok()
        .transpose()
        .ok()
        .flatten()
        .unwrap_or(0);
}

struct TestSyscallStubs {
    pub unix_timestamp: Option<i64>,
}

impl program_stubs::SyscallStubs for TestSyscallStubs {
    fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
        let clock: Option<i64> = self.unix_timestamp;
        unsafe {
            *(var_addr as *mut _ as *mut Clock) = Clock {
                unix_timestamp: clock.unwrap(),
                ..Clock::default()
            };
        }
        solana_program::entrypoint::SUCCESS
    }

    fn sol_log(&self, message: &str) {
        if *VERBOSE != 0 {
            println!("Program Log: {}", message);
        }
    }

    fn sol_invoke_signed(
        &self,
        instruction: &Instruction,
        account_infos: &[AccountInfo],
        signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        let mut new_account_infos = vec![];

        for meta in instruction.accounts.iter() {
            for account_info in account_infos.iter() {
                if meta.pubkey == *account_info.key {
                    let mut new_account_info = account_info.clone();
                    for seeds in signers_seeds.iter() {
                        let signer =
                            Pubkey::create_program_address(seeds, &marginfi::id()).unwrap();
                        if *account_info.key == signer {
                            new_account_info.is_signer = true;
                        }
                    }
                    new_account_infos.push(new_account_info);
                }
            }
        }

        spl_token::processor::Processor::process(
            &instruction.program_id,
            &new_account_infos,
            &instruction.data,
        )
    }
}

fn test_syscall_stubs(unix_timestamp: Option<i64>) {
    // only one test may run at a time
    program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs { unix_timestamp }));
}

fn initialize_marginfi_group<'bump>(
    bump: &'bump Bump,
    program_id: &'bump Pubkey,
    admin: AccountInfo<'bump>,
    system_program: AccountInfo<'bump>,
) -> AccountInfo<'bump> {
    let marginfi_group =
        new_owned_account(size_of::<MarginfiGroup>(), program_id, bump, Rent::free());

    marginfi::instructions::marginfi_group::initialize(Context::new(
        &marginfi::id(),
        &mut marginfi::instructions::MarginfiGroupInitialize {
            // Unchecked because we are initializing the account.
            marginfi_group: AccountLoader::try_from_unchecked(program_id, &marginfi_group).unwrap(),
            admin: Signer::try_from(&admin).unwrap(),
            system_program: Program::try_from(&system_program).unwrap(),
        },
        &[],
        BTreeMap::new(),
    ))
    .unwrap();

    set_discriminator::<MarginfiGroup>(marginfi_group.clone());

    marginfi_group
}

#[cfg(test)]
mod tests {
    use fixed::types::I80F48;
    use marginfi::{instructions::marginfi_account, state::marginfi_account::RiskEngine};

    use super::*;
    #[test]
    fn deposit_test() {
        let bump = bumpalo::Bump::new();

        let a = MarginfiGroupAccounts::setup(&bump, &[BankAndOracleConfig::dummy(); 2], 2);

        let al =
            AccountLoader::<MarginfiGroup>::try_from_unchecked(&marginfi::id(), &a.marginfi_group)
                .unwrap();

        assert_eq!(al.load().unwrap().admin, a.owner.key());

        a.process_action_deposit(&AccountIdx(0), &BankIdx(0), &AssetAmount(1000))
            .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &marginfi::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();
        let marginfi_account = marginfi_account_ai.load().unwrap();

        assert_eq!(
            I80F48::from(marginfi_account.lending_account.balances[0].asset_shares),
            I80F48!(1000)
        );
    }

    #[test]
    fn borrow_test() {
        let bump = bumpalo::Bump::new();

        let a = MarginfiGroupAccounts::setup(&bump, &[BankAndOracleConfig::dummy(); 2], 2);

        a.process_action_deposit(&AccountIdx(1), &BankIdx(1), &AssetAmount(1000))
            .unwrap();
        a.process_action_deposit(&AccountIdx(0), &BankIdx(0), &AssetAmount(1000))
            .unwrap();
        a.process_action_borrow(&AccountIdx(0), &BankIdx(1), &AssetAmount(100))
            .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &marginfi::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();

        {
            let marginfi_account = marginfi_account_ai.load().unwrap();

            assert_eq!(
                I80F48::from(marginfi_account.lending_account.balances[0].asset_shares),
                I80F48!(1000)
            );
            assert_eq!(
                I80F48::from(marginfi_account.lending_account.balances[1].liability_shares),
                I80F48!(100)
            );
        }

        a.process_action_repay(&AccountIdx(0), &BankIdx(1), &AssetAmount(100), false)
            .unwrap();

        let marginfi_account = marginfi_account_ai.load().unwrap();

        assert_eq!(
            I80F48::from(marginfi_account.lending_account.balances[1].liability_shares),
            I80F48!(0)
        );
    }

    #[test]
    fn liquidation_test() {
        let bump = bumpalo::Bump::new();

        let a = MarginfiGroupAccounts::setup(&bump, &[BankAndOracleConfig::dummy(); 2], 3);

        a.process_action_deposit(&AccountIdx(1), &BankIdx(1), &AssetAmount(1000))
            .unwrap();
        a.process_action_deposit(&AccountIdx(0), &BankIdx(0), &AssetAmount(1000))
            .unwrap();
        a.process_action_borrow(&AccountIdx(0), &BankIdx(1), &AssetAmount(500))
            .unwrap();

        a.process_update_oracle(&BankIdx(1), &PriceChange(190))
            .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &marginfi::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();

        {
            let marginfi_account = marginfi_account_ai.load().unwrap();
            let margin_account = &a.marginfi_accounts[0];
            let bank_map = a.get_bank_map();
            let remaining_accounts =
                &margin_account.get_remaining_accounts(&bank_map, vec![], vec![]);

            let re = RiskEngine::new(&marginfi_account, &remaining_accounts).unwrap();

            let health = re
                .get_account_health(
                    marginfi::state::marginfi_account::RiskRequirementType::Maintenance,
                )
                .unwrap();

            println!("Health {health}");
        }

        a.process_action_deposit(&AccountIdx(2), &BankIdx(1), &AssetAmount(1000))
            .unwrap();

        a.process_liquidate_account(
            &AccountIdx(2),
            &AccountIdx(0),
            &BankIdx(0),
            &BankIdx(1),
            &AssetAmount(50),
        )
        .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &marginfi::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();

        let marginfi_account = marginfi_account_ai.load().unwrap();

        assert_eq!(
            I80F48::from(marginfi_account.lending_account.balances[0].asset_shares),
            I80F48!(950)
        );
    }

    #[test]
    fn liquidation_and_bankruptcy() {
        let bump = bumpalo::Bump::new();

        let a = MarginfiGroupAccounts::setup(&bump, &[BankAndOracleConfig::dummy(); 2], 3);

        a.process_action_deposit(&AccountIdx(1), &BankIdx(1), &AssetAmount(1000))
            .unwrap();
        a.process_action_deposit(&AccountIdx(0), &BankIdx(0), &AssetAmount(1000))
            .unwrap();
        a.process_action_borrow(&AccountIdx(0), &BankIdx(1), &AssetAmount(500))
            .unwrap();

        a.process_update_oracle(&BankIdx(1), &PriceChange(10000))
            .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &marginfi::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();

        {
            let marginfi_account = marginfi_account_ai.load().unwrap();
            let margin_account = &a.marginfi_accounts[0];
            let bank_map = a.get_bank_map();
            let remaining_accounts =
                &margin_account.get_remaining_accounts(&bank_map, vec![], vec![]);

            let re = RiskEngine::new(&marginfi_account, &remaining_accounts).unwrap();

            let health = re
                .get_account_health(
                    marginfi::state::marginfi_account::RiskRequirementType::Maintenance,
                )
                .unwrap();

            println!("Health {health}");
        }

        a.process_action_deposit(&AccountIdx(2), &BankIdx(1), &AssetAmount(1000))
            .unwrap();

        a.process_liquidate_account(
            &AccountIdx(2),
            &AccountIdx(0),
            &BankIdx(0),
            &BankIdx(1),
            &AssetAmount(1000),
        )
        .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &marginfi::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();

        let marginfi_account = marginfi_account_ai.load().unwrap();

        assert_eq!(
            I80F48::from(marginfi_account.lending_account.balances[0].asset_shares),
            I80F48!(0)
        );
        assert_eq!(
            I80F48::from(marginfi_account.lending_account.balances[0].liability_shares),
            I80F48!(0)
        );
    }
}
