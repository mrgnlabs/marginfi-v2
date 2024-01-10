use super::{bank::BankFixture, marginfi_account::MarginfiAccountFixture};
use crate::prelude::MintFixture;
use crate::utils::*;
use anchor_lang::{prelude::*, solana_program::system_program, InstructionData};
use anchor_spl::token;
use anyhow::Result;
use marginfi::{
    prelude::MarginfiGroup,
    state::marginfi_group::{BankConfig, BankConfigOpt, BankVaultType, GroupConfig},
};
use solana_program::sysvar;
use solana_program_test::*;
use solana_sdk::{
    instruction::Instruction, signature::Keypair, signer::Signer, transaction::Transaction,
};
use std::{cell::RefCell, mem, rc::Rc};

pub struct MarginfiGroupFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
}

impl MarginfiGroupFixture {
    pub async fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        config: GroupConfig,
    ) -> MarginfiGroupFixture {
        let ctx_ref = ctx.clone();

        let group_key = Keypair::new();

        {
            let mut ctx = ctx.borrow_mut();

            let initialize_marginfi_group_ix = Instruction {
                program_id: marginfi::id(),
                accounts: marginfi::accounts::MarginfiGroupInitialize {
                    marginfi_group: group_key.pubkey(),
                    admin: ctx.payer.pubkey(),
                    system_program: system_program::id(),
                }
                .to_account_metas(Some(true)),
                data: marginfi::instruction::MarginfiGroupInitialize {}.data(),
            };

            let configure_marginfi_group_ix = Instruction {
                program_id: marginfi::id(),
                accounts: marginfi::accounts::MarginfiGroupConfigure {
                    marginfi_group: group_key.pubkey(),
                    admin: ctx.payer.pubkey(),
                }
                .to_account_metas(Some(true)),
                data: marginfi::instruction::MarginfiGroupConfigure { config }.data(),
            };

            let tx = Transaction::new_signed_with_payer(
                &[initialize_marginfi_group_ix, configure_marginfi_group_ix],
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
        bank_asset_mint_fixture: &MintFixture,
        bank_config: BankConfig,
    ) -> Result<BankFixture, BanksClientError> {
        let bank_key = Keypair::new();
        let bank_mint = bank_asset_mint_fixture.key;
        let bank_fixture =
            BankFixture::new(self.ctx.clone(), bank_key.pubkey(), bank_asset_mint_fixture);

        let mut accounts = marginfi::accounts::LendingPoolAddBank {
            marginfi_group: self.key,
            admin: self.ctx.borrow().payer.pubkey(),
            fee_payer: self.ctx.borrow().payer.pubkey(),
            bank_mint,
            bank: bank_key.pubkey(),
            liquidity_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Liquidity).0,
            liquidity_vault: bank_fixture.get_vault(BankVaultType::Liquidity).0,
            insurance_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Insurance).0,
            insurance_vault: bank_fixture.get_vault(BankVaultType::Insurance).0,
            fee_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Fee).0,
            fee_vault: bank_fixture.get_vault(BankVaultType::Fee).0,
            rent: sysvar::rent::id(),
            token_program: token::ID,
            system_program: system_program::id(),
        }
        .to_account_metas(Some(true));

        accounts.push(AccountMeta::new_readonly(bank_config.oracle_keys[0], false));

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingPoolAddBank {
                bank_config: bank_config.into(),
            }
            .data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.ctx.borrow().payer.pubkey().clone()),
            &[&self.ctx.borrow().payer, &bank_key],
            self.ctx.borrow().last_blockhash,
        );

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?;

        Ok(bank_fixture)
    }

    pub async fn try_lending_pool_add_bank_with_seed(
        &self,
        bank_asset_mint_fixture: &MintFixture,
        bank_config: BankConfig,
        bank_seed: u64,
    ) -> Result<BankFixture, BanksClientError> {
        let bank_mint = bank_asset_mint_fixture.key;

        let bank_keypair_seed = [
            self.key.as_ref(),
            bank_mint.as_ref(),
            &bank_seed.to_le_bytes(),
        ]
        .as_slice();

        // Create PDA account from seeds
        let (pda, bump) = Pubkey::find_program_address(&bank_keypair_seed, &marginfi::id());

        let bank_mint = bank_asset_mint_fixture.key;
        let bank_fixture = BankFixture::new(self.ctx.clone(), pda, bank_asset_mint_fixture);

        let mut accounts = marginfi::accounts::LendingPoolAddBank {
            marginfi_group: self.key,
            admin: self.ctx.borrow().payer.pubkey(),
            fee_payer: self.ctx.borrow().payer.pubkey(),
            bank_mint,
            bank: pda,
            liquidity_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Liquidity).0,
            liquidity_vault: bank_fixture.get_vault(BankVaultType::Liquidity).0,
            insurance_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Insurance).0,
            insurance_vault: bank_fixture.get_vault(BankVaultType::Insurance).0,
            fee_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Fee).0,
            fee_vault: bank_fixture.get_vault(BankVaultType::Fee).0,
            rent: sysvar::rent::id(),
            token_program: token::ID,
            system_program: system_program::id(),
        }
        .to_account_metas(Some(true));

        accounts.push(AccountMeta::new_readonly(bank_config.oracle_keys[0], false));

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingPoolAddBank {
                bank_config: bank_config.into(),
            }
            .data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.ctx.borrow().payer.pubkey().clone()),
            &[&self.ctx.borrow().payer],
            self.ctx.borrow().last_blockhash,
        );

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?;

        Ok(bank_fixture)
    }

    pub fn make_lending_pool_configure_bank_ix(
        &self,
        bank: &BankFixture,
        bank_config_opt: BankConfigOpt,
    ) -> Instruction {
        let mut accounts = marginfi::accounts::LendingPoolConfigureBank {
            bank: bank.key,
            marginfi_group: self.key,
            admin: self.ctx.borrow().payer.pubkey(),
        }
        .to_account_metas(Some(true));

        if let Some(oracle_config) = bank_config_opt.oracle {
            accounts.extend(
                oracle_config
                    .keys
                    .iter()
                    .map(|k| AccountMeta::new_readonly(*k, false)),
            );
        }

        Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingPoolConfigureBank { bank_config_opt }.data(),
        }
    }

    pub async fn try_lending_pool_configure_bank(
        &self,
        bank: &BankFixture,
        bank_config_opt: BankConfigOpt,
    ) -> Result<(), BanksClientError> {
        let ix = self.make_lending_pool_configure_bank_ix(bank, bank_config_opt);
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.ctx.borrow().payer.pubkey().clone()),
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

    pub async fn try_accrue_interest(&self, bank: &BankFixture) -> Result<()> {
        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingPoolAccrueBankInterest {
                marginfi_group: self.key,
                bank: bank.key,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingPoolAccrueBankInterest {}.data(),
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

    pub async fn try_collect_fees(&self, bank: &BankFixture) -> Result<()> {
        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingPoolCollectBankFees {
                marginfi_group: self.key,
                bank: bank.key,
                liquidity_vault_authority: bank.get_vault_authority(BankVaultType::Liquidity).0,
                liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
                insurance_vault: bank.get_vault(BankVaultType::Insurance).0,
                fee_vault: bank.get_vault(BankVaultType::Fee).0,
                token_program: token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingPoolCollectBankFees {}.data(),
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
                .load_observation_account_metas(vec![], vec![])
                .await,
        );

        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts,
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
