#![cfg(feature = "test-bpf")]
#![allow(unused)]

use super::utils::load_and_deserialize;
use anchor_lang::{prelude::Pubkey, AccountDeserialize, InstructionData, ToAccountMetas};
use marginfi::{
    state::marginfi_group::{Bank, BankConfigOpt, BankVaultType},
    utils::{find_bank_vault_authority_pda, find_bank_vault_pda},
};
use solana_program::instruction::Instruction;
use solana_program_test::ProgramTestContext;
use solana_sdk::{signer::Signer, transaction::Transaction};
use std::{cell::RefCell, fmt::Debug, rc::Rc};

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

    pub async fn update_config(&self, config: BankConfigOpt) -> anyhow::Result<()> {
        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingPoolConfigureBank {
                marginfi_group: self.load().await.group,
                admin: self.ctx.borrow().payer.pubkey(),
                bank: self.key,
                pyth_oracle: if let Some(pyth_oracle) = config.pyth_oracle {
                    pyth_oracle
                } else {
                    Pubkey::default()
                },
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingPoolConfigureBank {
                bank_config_opt: config,
            }
            .data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.ctx.borrow().payer.pubkey()),
            &[&self.ctx.borrow().payer],
            self.ctx.borrow().last_blockhash,
        );

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?;

        Ok(())
    }
}

impl Debug for BankFixture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BankFixture")
            .field("key", &self.key)
            .finish()
    }
}
