use anchor_lang::{
    prelude::{
        Account, AccountInfo, AccountLoader, Clock, Context, Program, ProgramError, Pubkey, Rent,
        Signer, SolanaSysvar, Sysvar,
    },
    Discriminator, Key,
};
use arbitrary::Arbitrary;
use bumpalo::Bump;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use lazy_static::lazy_static;
use marginfi::{
    constants::PYTH_ID,
    prelude::MarginfiGroup,
    state::marginfi_group::{Bank, BankConfig, BankVaultType, InterestRateConfig, WrappedI80F48},
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
    program_stubs::{self, SyscallStubs},
    stake_history::Epoch,
    system_program, sysvar,
};
use spl_token::state::Mint;
use std::{
    collections::BTreeMap,
    mem::{align_of, size_of},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

type SplAccount = spl_token::state::Account;

pub struct MarginfiGroupAccounts<'info> {
    pub marginfi_group: AccountInfo<'info>,
    pub banks: Vec<BankAccounts<'info>>,
    pub owner: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent_sysvar: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

impl<'bump> MarginfiGroupAccounts<'bump> {
    pub fn setup(bump: &'bump Bump) -> Self {
        let marginfi_program = new_marginfi_program(bump);
        let system_program = new_system_program(bump);
        let token_program = new_spl_token_program(bump);
        let admin = new_sol_account(1_000_000, bump);
        let rent_sysvar = new_rent_sysvar_account(0, Rent::free(), bump);
        let marginfi_group = initialize_marginfi_group(
            bump,
            &marginfi_program.key,
            admin.clone(),
            system_program.clone(),
        );

        MarginfiGroupAccounts {
            marginfi_group,
            banks: vec![],
            owner: admin,
            system_program,
            rent_sysvar,
            token_program,
        }
    }

    pub fn setup_banks(
        &mut self,
        bump: &'bump Bump,
        rent: Rent,
        n_banks: usize,
        initial_bank_configs: &[BankAndOracleConfig],
    ) {
        for i in 0..n_banks {
            let bank = self.setup_bank(bump, rent, initial_bank_configs[i]);
            self.banks.push(bank);
        }
    }

    fn setup_bank(
        &self,
        bump: &'bump Bump,
        rent: Rent,
        initial_bank_config: BankAndOracleConfig,
    ) -> BankAccounts<'bump> {
        let bank = new_owned_account(size_of::<Bank>(), &marginfi::ID, bump, rent);

        let mint = new_token_mint(bump, rent.clone(), initial_bank_config.mint_decimals);
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
            mint.key.clone(),
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

        marginfi::instructions::marginfi_group::lending_pool_add_bank(
            Context::new(
                &marginfi::id(),
                &mut marginfi::instructions::LendingPoolAddBank {
                    marginfi_group: AccountLoader::try_from(&self.marginfi_group).unwrap(),
                    admin: Signer::try_from(&self.owner).unwrap(),
                    bank_mint: Box::new(Account::try_from(&mint).unwrap()),
                    bank: AccountLoader::try_from_unchecked(&marginfi::ID, &bank).unwrap(),
                    liquidity_vault_authority,
                    liquidity_vault: Box::new(Account::try_from(&liquidity_vault).unwrap()),
                    insurance_vault_authority,
                    insurance_vault: Box::new(Account::try_from(&insurance_vault).unwrap()),
                    fee_vault_authority,
                    fee_vault: Box::new(Account::try_from(&fee_vault).unwrap()),
                    pyth_oracle: oracle.clone(),
                    rent: Sysvar::from_account_info(&self.rent_sysvar).unwrap(),
                    token_program: Program::try_from(&self.token_program).unwrap(),
                    system_program: Program::try_from(&self.system_program).unwrap(),
                },
                &[],
                seed_bump_map,
            ),
            BankConfig {
                deposit_weight_init: initial_bank_config.deposit_weight_init,
                deposit_weight_maint: initial_bank_config.deposit_weight_maint,
                liability_weight_init: initial_bank_config.liability_weight_init,
                liability_weight_maint: initial_bank_config.liability_weight_maint,
                max_capacity: initial_bank_config.max_capacity,
                pyth_oracle: oracle.key.clone(),
                interest_rate_config: InterestRateConfig {
                    optimal_utilization_rate: I80F48!(0.5).into(),
                    plateau_interest_rate: I80F48!(0.5).into(),
                    max_interest_rate: I80F48!(4).into(),
                    insurance_fee_fixed_apr: I80F48!(0.01).into(),
                    insurance_ir_fee: I80F48!(0.05).into(),
                    protocol_fixed_fee_apr: I80F48!(0.01).into(),
                    protocol_ir_fee: I80F48!(0.1).into(),
                },
            },
        )
        .unwrap();

        BankAccounts {
            bank,
            oracle,
            liquidity_vault,
            insurance_vault,
            fee_vault,
            mint,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BankAndOracleConfig {
    pub oracle_native_price: u64,
    pub mint_decimals: u8,

    pub deposit_weight_init: WrappedI80F48,
    pub deposit_weight_maint: WrappedI80F48,

    pub liability_weight_init: WrappedI80F48,
    pub liability_weight_maint: WrappedI80F48,

    pub max_capacity: u64,
}

impl<'a> Arbitrary<'a> for BankAndOracleConfig {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mint_decimals: u8 = u.arbitrary::<u8>()? % 10;

        // let deposit_weight_maint: f32 = u.arbitrary::<f32>()? % 1.0;
        // let deposit_weight_init: f32 = u.arbitrary::<f32>()? % deposit_weight_maint;

        // let liability_weight_maint: f32 = u.arbitrary::<f32>()? % 2.0 + 1.0;
        // let liability_weight_init: f32 = u.arbitrary::<f32>()? % liability_weight_maint + 1.0;

        Ok(Self {
            oracle_native_price: u.arbitrary::<u64>()? % 1_000 * 10u64.pow(mint_decimals as u32),
            mint_decimals,
            deposit_weight_init: I80F48!(1).into(),
            deposit_weight_maint: I80F48!(1).into(),
            liability_weight_init: I80F48!(1).into(),
            liability_weight_maint: I80F48!(1).into(),
            max_capacity: u.arbitrary::<u64>()? % (1_000_000_000 * 10u64.pow(mint_decimals as u32)),
        })
    }
}

impl BankAndOracleConfig {
    pub fn dummy() -> Self {
        Self {
            oracle_native_price: 20 * 10u64.pow(6),
            mint_decimals: 6,
            deposit_weight_init: I80F48!(1).into(),
            deposit_weight_maint: I80F48!(1).into(),
            liability_weight_init: I80F48!(1).into(),
            liability_weight_maint: I80F48!(1).into(),
            max_capacity: 1_000_000_000_000 * 10u64.pow(6),
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

pub fn get_vault_address(
    bank: &Pubkey,
    vault_type: BankVaultType,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[vault_type.get_seed(), bank.as_ref()], program_id)
}

pub fn get_vault_authority(
    bank: &Pubkey,
    vault_type: BankVaultType,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[vault_type.get_authority_seed(), bank.as_ref()],
        program_id,
    )
}

pub struct BankAccounts<'info> {
    pub bank: AccountInfo<'info>,
    pub oracle: AccountInfo<'info>,
    pub liquidity_vault: AccountInfo<'info>,
    pub insurance_vault: AccountInfo<'info>,
    pub fee_vault: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

impl<'bump> BankAccounts<'bump> {
    // pub fn setup();}
}
fn random_pubkey(bump: &Bump) -> &Pubkey {
    bump.alloc(Pubkey::new(transmute_to_bytes(&rand::random::<[u64; 4]>())))
}

fn allocate_dex_owned_account(unpadded_size: usize, bump: &Bump) -> &mut [u8] {
    assert_eq!(unpadded_size % 8, 0);
    let padded_size = unpadded_size + 12;
    let u64_data = bump.alloc_slice_fill_copy(padded_size / 8 + 1, 0u64);
    let data = transmute_to_bytes_mut(u64_data); //[3..padded_size + 3];

    // assert_eq!(
    //     data[8..padded_size + 8].as_ptr() as usize % align_of::<&[u8]>(),
    //     0
    // );

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
    let (vault_address, seed_bump) = get_vault_address(bank, vault_type, &marginfi::ID);

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
    let (vault_address, seed_bump) = get_vault_address(bank, vault_type, &marginfi::ID);

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
    let data = bytemuck::bytes_of(&PriceAccount {
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
    })
    .to_vec();

    AccountInfo::new(
        random_pubkey(bump),
        false,
        true,
        bump.alloc(rent.minimum_balance(data.len())),
        bump.alloc(data),
        &PYTH_ID,
        false,
        Epoch::default(),
    )
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
    unix_timestamp: Option<i64>,
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
        if *VERBOSE >= 1 {
            println!("{}", message);
        }
    }

    fn sol_invoke_signed(
        &self,
        instruction: &Instruction,
        account_infos: &[AccountInfo],
        signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        let mut new_account_infos = vec![];

        // mimic check for token program in accounts
        if !account_infos.iter().any(|x| *x.key == spl_token::id()) {
            return Err(ProgramError::InvalidAccountData);
        }

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

pub fn setup_marginfi_group(bump: &Bump) -> MarginfiGroupAccounts {
    test_syscall_stubs(Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    ));

    let marginfi_program = new_marginfi_program(bump);
    let system_program = new_system_program(bump);
    let token_program = new_spl_token_program(bump);
    let admin = new_sol_account(1_000_000, bump);
    let rent_sysvar = new_rent_sysvar_account(0, Rent::free(), bump);
    let marginfi_group = initialize_marginfi_group(
        bump,
        &marginfi_program.key,
        admin.clone(),
        system_program.clone(),
    );

    MarginfiGroupAccounts {
        marginfi_group,
        banks: vec![],
        owner: admin,
        system_program,
        rent_sysvar,
        token_program,
    }
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
        &mut marginfi::instructions::InitializeMarginfiGroup {
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
