#![cfg(feature = "test-bpf")]
#![allow(unused)]

use crate::fixtures::{spl::*, utils::*};
use anchor_lang::{prelude::*, solana_program::system_program, InstructionData};
use anchor_spl::token;
use anyhow::Result;
use marginfi::{
    constants::*,
    prelude::{MarginfiGroup, MarginfiResult},
    state::marginfi_group::{BankConfig, BankConfigOpt, BankVaultType, GroupConfig},
};
use solana_program::sysvar;
use solana_program_test::*;
use solana_sdk::{
    account::AccountSharedData, instruction::Instruction, signature::Keypair, signer::Signer,
    system_instruction, transaction::Transaction, transport::TransportError,
};
use std::{
    cell::{RefCell, RefMut},
    convert::TryInto,
    mem,
    rc::Rc,
};

use super::{bank::BankFixture, marginfi_account::MarginfiAccountFixture};

pub struct MarginfiGroupFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
}

impl MarginfiGroupFixture {
    pub async fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        config_arg: GroupConfig,
    ) -> MarginfiGroupFixture {
        let ctx_ref = ctx.clone();
        let group_key = Keypair::new();

        {
            let mut ctx = ctx.borrow_mut();

            let accounts = marginfi::accounts::InitializeMarginfiGroup {
                marginfi_group: group_key.pubkey(),
                admin: ctx.payer.pubkey(),
                system_program: system_program::id(),
            };
            let init_marginfi_group_ix = Instruction {
                program_id: marginfi::id(),
                accounts: accounts.to_account_metas(Some(true)),
                data: marginfi::instruction::InitializeMarginfiGroup {}.data(),
            };
            let rent = ctx.banks_client.get_rent().await.unwrap();
            let size = MarginfiGroupFixture::get_size();
            let create_marginfi_group_ix = system_instruction::create_account(
                &ctx.payer.pubkey(),
                &group_key.pubkey(),
                rent.minimum_balance(size),
                size as u64,
                &marginfi::id(),
            );

            let tx = Transaction::new_signed_with_payer(
                &[init_marginfi_group_ix],
                Some(&ctx.payer.pubkey().clone()),
                &[&ctx.payer, &group_key],
                ctx.last_blockhash,
            );
            ctx.banks_client.process_transaction(tx).await.unwrap();
        }

        MarginfiGroupFixture {
            ctx: ctx_ref.clone(),
            key: group_key.pubkey(),
        }
    }

    pub async fn try_lending_pool_add_bank(
        &self,
        bank_asset_mint: Pubkey,
        bank_config: BankConfig,
    ) -> Result<BankFixture, BanksClientError> {
        let mut ctx = self.ctx.borrow_mut();
        let rent = ctx.banks_client.get_rent().await.unwrap();

        let bank_key = Keypair::new();
        let bank_fixture = BankFixture::new(self.ctx.clone(), bank_key.pubkey());

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingPoolAddBank {
                marginfi_group: self.key,
                admin: ctx.payer.pubkey(),
                bank_mint: bank_asset_mint,
                bank: bank_key.pubkey(),
                liquidity_vault_authority: bank_fixture
                    .get_vault_authority(BankVaultType::Liquidity)
                    .0,
                liquidity_vault: bank_fixture.get_vault(BankVaultType::Liquidity).0,
                insurance_vault_authority: bank_fixture
                    .get_vault_authority(BankVaultType::Insurance)
                    .0,
                insurance_vault: bank_fixture.get_vault(BankVaultType::Insurance).0,
                fee_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Fee).0,
                fee_vault: bank_fixture.get_vault(BankVaultType::Fee).0,
                rent: sysvar::rent::id(),
                token_program: token::ID,
                system_program: system_program::id(),
                pyth_oracle: bank_config.pyth_oracle,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingPoolAddBank { bank_config }.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer, &bank_key],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(bank_fixture)
    }

    pub async fn try_lending_pool_configure_bank(
        &self,
        bank: BankFixture,
        bank_config_opt: BankConfigOpt,
    ) -> Result<()> {
        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingPoolConfigureBank {
                bank: bank.key,
                marginfi_group: self.key,
                admin: ctx.payer.pubkey(),
                pyth_oracle: bank_config_opt.pyth_oracle.unwrap_or_default(),
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingPoolConfigureBank {
                bank_config_opt: todo!(),
            }
            .data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(())
    }

    pub async fn try_accrue_interest(&self, bank: &BankFixture) -> Result<()> {
        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingPoolBankAccrueInterest {
                marginfi_group: self.key,
                bank: bank.key,
                liquidity_vault_authority: bank.get_vault_authority(BankVaultType::Liquidity).0,
                liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
                insurance_vault: bank.get_vault(BankVaultType::Insurance).0,
                fee_vault: bank.get_vault(BankVaultType::Fee).0,
                token_program: token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::BankAccrueInterest {}.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(())
    }

    pub async fn try_handle_bankruptcy(
        &self,
        bank: &BankFixture,
        marginfi_account: &MarginfiAccountFixture,
    ) -> Result<(), BanksClientError> {
        let mut accounts = marginfi::accounts::LendingPoolHandleBankruptcy {
            marginfi_group: self.key,
            admin: self.ctx.borrow().payer.pubkey(),
            bank: bank.key,
            marginfi_account: marginfi_account.key,
            liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            insurance_vault: bank.get_vault(BankVaultType::Insurance).0,
            insurance_vault_authority: bank.get_vault_authority(BankVaultType::Insurance).0,
            token_program: token::ID,
        }
        .to_account_metas(Some(true));

        accounts.append(
            &mut marginfi_account
                .load_observation_account_metas(vec![])
                .await,
        );

        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: accounts,
            data: marginfi::instruction::LendingPoolHandleBankruptcy {}.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await
    }

    pub fn get_size() -> usize {
        8 + mem::size_of::<MarginfiGroup>()
    }

    pub async fn load(&self) -> marginfi::state::marginfi_group::MarginfiGroup {
        load_and_deserialize::<marginfi::state::marginfi_group::MarginfiGroup>(
            self.ctx.clone(),
            &self.key,
        )
        .await
    }
}
