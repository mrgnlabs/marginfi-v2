use crate::SplAccount;
use anchor_lang::{
    prelude::{AccountInfo, Pubkey, Rent, SolanaSysvar},
    Discriminator,
};
use bumpalo::Bump;
use marginfi::{constants::PYTH_ID, state::marginfi_group::BankVaultType};
use pyth_sdk_solana::state::{
    AccountType, PriceAccount, PriceInfo, PriceStatus, Rational, MAGIC, VERSION_2,
};
use safe_transmute::{transmute_to_bytes, transmute_to_bytes_mut};
use solana_program::{
    bpf_loader, program_pack::Pack, stake_history::Epoch, system_program, sysvar,
};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token::state::Mint;
use std::mem::size_of;

pub struct AccountsState {
    pub bump: Bump,
}

impl AccountsState {
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }

    fn random_pubkey<'bump>(&'bump self) -> &Pubkey {
        self.bump
            .alloc(Pubkey::new(transmute_to_bytes(&rand::random::<[u64; 4]>())))
    }

    pub fn new_sol_account<'bump>(&'bump self, lamports: u64) -> AccountInfo<'bump> {
        self.new_sol_account_with_pubkey(self.random_pubkey(), lamports)
    }

    pub fn new_sol_account_with_pubkey<'bump>(
        &'bump self,
        pubkey: &'bump Pubkey,
        lamports: u64,
    ) -> AccountInfo<'bump> {
        AccountInfo::new(
            pubkey,
            true,
            false,
            self.bump.alloc(lamports),
            &mut [],
            &system_program::ID,
            false,
            Epoch::default(),
        )
    }

    pub fn new_token_mint<'bump>(&'bump self, rent: Rent, decimals: u8) -> AccountInfo<'bump> {
        let data = self.bump.alloc_slice_fill_copy(Mint::LEN, 0u8);
        let mut mint = Mint::default();
        mint.is_initialized = true;
        mint.decimals = decimals;
        Mint::pack(mint, data).unwrap();
        AccountInfo::new(
            self.random_pubkey(),
            false,
            true,
            self.bump.alloc(rent.minimum_balance(data.len())),
            data,
            &spl_token::ID,
            false,
            Epoch::default(),
        )
    }

    pub fn new_token_account<'bump, 'a, 'b>(
        &'bump self,
        mint_pubkey: &'a Pubkey,
        owner_pubkey: &'b Pubkey,
        balance: u64,
        rent: Rent,
    ) -> AccountInfo<'bump> {
        self.new_token_account_with_pubkey(
            Keypair::new().pubkey(),
            mint_pubkey,
            owner_pubkey,
            balance,
            rent,
        )
    }

    pub fn new_token_account_with_pubkey<'bump, 'a, 'b>(
        &'bump self,
        account_pubkey: Pubkey,
        mint_pubkey: &'a Pubkey,
        owner_pubkey: &'b Pubkey,
        balance: u64,
        rent: Rent,
    ) -> AccountInfo<'bump> {
        let data = self.bump.alloc_slice_fill_copy(SplAccount::LEN, 0u8);
        let mut account = SplAccount::default();
        account.state = spl_token::state::AccountState::Initialized;
        account.mint = *mint_pubkey;
        account.owner = *owner_pubkey;
        account.amount = balance;
        SplAccount::pack(account, data).unwrap();
        AccountInfo::new(
            self.bump.alloc(account_pubkey),
            false,
            true,
            self.bump.alloc(rent.minimum_balance(data.len())),
            data,
            &spl_token::ID,
            false,
            Epoch::default(),
        )
    }

    pub fn new_owned_account<'bump>(
        &'bump self,
        unpadded_len: usize,
        owner_pubkey: Pubkey,
        rent: Rent,
    ) -> AccountInfo<'bump> {
        let data_len = unpadded_len + 12;
        self.new_dex_owned_account_with_lamports(
            unpadded_len,
            rent.minimum_balance(data_len),
            self.bump.alloc(owner_pubkey),
        )
    }

    pub fn new_dex_owned_account_with_lamports<'bump>(
        &'bump self,
        unpadded_len: usize,
        lamports: u64,
        program_id: &'bump Pubkey,
    ) -> AccountInfo<'bump> {
        AccountInfo::new(
            self.random_pubkey(),
            false,
            true,
            self.bump.alloc(lamports),
            self.allocate_dex_owned_account(unpadded_len),
            program_id,
            false,
            Epoch::default(),
        )
    }

    fn allocate_dex_owned_account<'bump>(&'bump self, unpadded_size: usize) -> &mut [u8] {
        assert_eq!(unpadded_size % 8, 0);
        let padded_size = unpadded_size + 12;
        let u64_data = self.bump.alloc_slice_fill_copy(padded_size / 8 + 1, 0u64);

        transmute_to_bytes_mut(u64_data) as _
    }

    pub fn new_spl_token_program(&self) -> AccountInfo {
        self.new_program(spl_token::id())
    }

    pub fn new_system_program(&self) -> AccountInfo {
        self.new_program(system_program::id())
    }

    pub fn new_marginfi_program(&self) -> AccountInfo {
        self.new_program(marginfi::id())
    }

    pub fn new_program(&self, pubkey: Pubkey) -> AccountInfo {
        AccountInfo::new(
            self.bump.alloc(pubkey),
            false,
            false,
            self.bump.alloc(0),
            &mut [],
            &bpf_loader::ID,
            true,
            Epoch::default(),
        )
    }

    pub fn new_oracle_account(
        &self,
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

        let data = bytemuck::bytes_of(&price_account);
        let data_len = data.len();
        let lamports = self.bump.alloc(rent.minimum_balance(data_len));
        let data_ptr = self.bump.alloc_slice_fill_copy(data_len, 0u8);
        data_ptr.copy_from_slice(data);

        AccountInfo::new(
            self.random_pubkey(),
            false,
            true,
            lamports,
            data_ptr,
            &PYTH_ID,
            false,
            Epoch::default(),
        )
    }

    pub fn new_rent_sysvar_account(&self, rent: Rent) -> AccountInfo {
        let data = self.bump.alloc_slice_fill_copy(size_of::<Rent>(), 0u8);
        let lamports = rent.minimum_balance(data.len());

        let mut account_info = AccountInfo::new(
            &sysvar::rent::ID,
            false,
            false,
            self.bump.alloc(lamports),
            data,
            &sysvar::ID,
            false,
            Epoch::default(),
        );

        rent.to_account_info(&mut account_info).unwrap();

        account_info
    }

    pub fn new_vault_account<'bump>(
        &'bump self,
        vault_type: BankVaultType,
        mint_pubkey: &'bump Pubkey,
        owner: &'bump Pubkey,
        bank: &'bump Pubkey,
    ) -> (AccountInfo<'bump>, u8) {
        let (vault_address, seed_bump) = get_vault_address(bank, vault_type);

        (
            self.new_token_account_with_pubkey(vault_address, mint_pubkey, owner, 0, Rent::free()),
            seed_bump,
        )
    }

    pub fn new_vault_authority<'bump>(
        &'bump self,
        vault_type: BankVaultType,
        bank: &'bump Pubkey,
    ) -> (AccountInfo<'bump>, u8) {
        let (vault_address, seed_bump) = get_vault_authority(bank, vault_type);

        (
            AccountInfo::new(
                self.bump.alloc(vault_address),
                false,
                false,
                self.bump.alloc(0),
                &mut [],
                &system_program::ID,
                false,
                Epoch::default(),
            ),
            seed_bump,
        )
    }

    pub fn reset(&mut self) {
        self.bump.reset();
    }
}

pub struct AccountInfoCache<'bump> {
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

pub fn get_vault_address(bank: &Pubkey, vault_type: BankVaultType) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[vault_type.get_seed(), &bank.to_bytes()], &marginfi::ID)
}

pub fn get_vault_authority(bank: &Pubkey, vault_type: BankVaultType) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[vault_type.get_authority_seed(), &bank.to_bytes()],
        &marginfi::ID,
    )
}

pub fn set_discriminator<T: Discriminator>(ai: AccountInfo) {
    let mut data = ai.try_borrow_mut_data().unwrap();

    if data[..8].ne(&[0u8; 8]) {
        panic!("Account discriminator is already set");
    }

    data[..8].copy_from_slice(&T::DISCRIMINATOR);
}
