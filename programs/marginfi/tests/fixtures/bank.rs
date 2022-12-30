#![cfg(feature = "test-bpf")]
#![allow(unused)]

use anchor_lang::{prelude::Pubkey, AccountDeserialize};
use marginfi::state::marginfi_group::{Bank, BankVaultType};
use solana_program_test::ProgramTestContext;
use std::{cell::RefCell, rc::Rc};

use super::utils::{find_bank_vault_authority_pda, find_bank_vault_pda, load_and_deserialize};

pub struct BankFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
}

impl BankFixture {
    pub fn new(ctx: Rc<RefCell<ProgramTestContext>>, key: Pubkey) -> Self {
        Self { ctx, key }
    }

    pub fn get_vault(&self, vault_type: BankVaultType) -> (Pubkey, u8) {
        find_bank_vault_pda(&self.key, vault_type)
    }

    pub fn get_vault_authority(&self, vault_type: BankVaultType) -> (Pubkey, u8) {
        find_bank_vault_authority_pda(&self.key, vault_type)
    }

    pub async fn load(&self) -> Bank {
        load_and_deserialize::<Bank>(self.ctx.clone(), &self.key).await
    }
}
