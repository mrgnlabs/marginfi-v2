use std::{collections::BTreeMap, mem::size_of};

use anchor_lang::{
    accounts::program,
    prelude::{AccountInfo, AccountLoader, Context, Program, Pubkey, Rent, Signer, SolanaSysvar},
    Discriminator,
};
use bumpalo::Bump;
use marginfi::{prelude::MarginfiGroup, state::marginfi_group::BankVaultType};
use safe_transmute::{transmute_to_bytes, transmute_to_bytes_mut};
use solana_program::{
    bpf_loader, program_pack::Pack, stake_history::Epoch, system_program, sysvar,
};
use spl_token::state::Mint;

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

    pub fn setup_banks(&mut self, bump: &'bump Bump, n_banks: usize) {
        for _ in 0..n_banks {
            let bank = BankAccounts::setup(bump);
            self.banks.push(bank);
        }
    }
}

pub fn new_token_mint(bump: &Bump, rent: Rent) -> AccountInfo {
    let data = bump.alloc_slice_fill_copy(Mint::LEN, 0u8);
    let mut mint = Mint::default();
    mint.is_initialized = true;
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

pub fn get_vault_address(bank: &Pubkey, vault_type: BankVaultType, program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[vault_type.get_seed(), bank.as_ref()], program_id).0
}

pub fn get_vault_authority(
    bank: &Pubkey,
    vault_type: BankVaultType,
    program_id: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &[vault_type.get_authority_seed(), bank.as_ref()],
        program_id,
    )
    .0
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
    pub fn setup(bump: &'bump Bump, rent: Rent) -> Self {
        let bank = new_sol_account(1_000_000, bump);
        let mint = new_token_mint(bump, rent.clone());
        let liquidity_vault_authority =
            new_vault_authority(BankVaultType::Liquidity, bank.key, bump);
        let liquidity_vault = new_token_account(
            mint.key,
            liquidity_vault_authority.key,
            0,
            bump,
            rent.clone(),
        );

        let insurance_vault_authority =
            new_vault_authority(BankVaultType::Insurance, bank.key, bump);
        let insurance_vault = new_token_account(
            mint.key,
            insurance_vault_authority.key,
            0,
            bump,
            rent.clone(),
        );

        let fee_vault_authority = new_vault_authority(BankVaultType::Fee, bank.key, bump);
        let fee_vault = new_token_account(mint.key, fee_vault_authority.key, 0, bump, rent.clone());

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

fn random_pubkey(bump: &Bump) -> &Pubkey {
    bump.alloc(Pubkey::new(transmute_to_bytes(&rand::random::<[u64; 4]>())))
}

fn allocate_dex_owned_account(unpadded_size: usize, bump: &Bump) -> &mut [u8] {
    assert_eq!(unpadded_size % 8, 0);
    let padded_size = unpadded_size + 12;
    let u64_data = bump.alloc_slice_fill_copy(padded_size / 8 + 1, 0u64);
    &mut transmute_to_bytes_mut(u64_data)[3..padded_size + 3]
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
    bank: &'bump Pubkey,
    bump: &'bump Bump,
) -> AccountInfo<'bump> {
    AccountInfo::new(
        bump.alloc(get_vault_address(bank, vault_type, &marginfi::ID)),
        false,
        false,
        bump.alloc(0),
        &mut [],
        &system_program::ID,
        false,
        Epoch::default(),
    )
}

pub fn new_vault_authority<'bump>(
    vault_type: BankVaultType,
    bank: &'bump Pubkey,
    bump: &'bump Bump,
) -> AccountInfo<'bump> {
    AccountInfo::new(
        bump.alloc(get_vault_authority(bank, vault_type, &marginfi::ID)),
        false,
        false,
        bump.alloc(0),
        &mut [],
        &system_program::ID,
        false,
        Epoch::default(),
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

pub fn setup_marginfi_group(bump: &Bump) -> MarginfiGroupAccounts {
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
