use super::utils::load_and_deserialize;
use crate::prelude::MintFixture;
use crate::spl::TokenAccountFixture;
use anchor_lang::{
    prelude::{AccountMeta, Pubkey},
    InstructionData, ToAccountMetas,
};
use marginfi::{
    state::marginfi_group::{Bank, BankConfigOpt, BankVaultType},
    utils::{find_bank_vault_authority_pda, find_bank_vault_pda},
};
use solana_program::instruction::Instruction;
use solana_program_test::ProgramTestContext;
use solana_sdk::{signer::Signer, transaction::Transaction};
use std::{cell::RefCell, fmt::Debug, rc::Rc};

#[derive(Clone)]
pub struct BankFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
    pub mint: MintFixture,
}

impl BankFixture {
    pub fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        key: Pubkey,
        mint_fixture: &MintFixture,
    ) -> Self {
        Self {
            ctx,
            key,
            mint: mint_fixture.clone(),
        }
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
        let mut accounts = marginfi::accounts::LendingPoolConfigureBank {
            marginfi_group: self.load().await.group,
            admin: self.ctx.borrow().payer.pubkey(),
            bank: self.key,
        }
        .to_account_metas(Some(true));

        if let Some(oracle_config) = config.oracle {
            accounts.extend(
                oracle_config
                    .keys
                    .iter()
                    .map(|k| AccountMeta::new_readonly(*k, false)),
            );
        }

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts,
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

    pub async fn get_vault_token_account(&self, vault_type: BankVaultType) -> TokenAccountFixture {
        let (vault, _) = self.get_vault(vault_type);

        TokenAccountFixture::fetch(self.ctx.clone(), vault).await
    }
}

impl Debug for BankFixture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BankFixture")
            .field("key", &self.key)
            .finish()
    }
}
