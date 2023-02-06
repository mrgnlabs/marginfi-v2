use super::{bank::BankFixture, prelude::*};
use crate::ui_to_native;
use anchor_lang::{prelude::*, system_program, InstructionData, ToAccountMetas};
use anchor_spl::token;
use marginfi::state::{
    marginfi_account::MarginfiAccount,
    marginfi_group::{Bank, BankVaultType},
};
use solana_program::instruction::Instruction;
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use std::{cell::RefCell, mem, rc::Rc};

#[derive(Default, Clone)]
pub struct MarginfiAccountConfig {}

pub struct MarginfiAccountFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
}

impl MarginfiAccountFixture {
    pub async fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        marginfi_group: &Pubkey,
    ) -> MarginfiAccountFixture {
        let ctx_ref = ctx.clone();
        let account_key = Keypair::new();

        {
            let mut ctx = ctx.borrow_mut();

            let accounts = marginfi::accounts::MarginfiAccountInitialize {
                marginfi_account: account_key.pubkey(),
                marginfi_group: *marginfi_group,
                signer: ctx.payer.pubkey(),
                system_program: system_program::ID,
            };
            let init_marginfi_account_ix = Instruction {
                program_id: marginfi::id(),
                accounts: accounts.to_account_metas(Some(true)),
                data: marginfi::instruction::MarginfiAccountInitialize {}.data(),
            };

            let tx = Transaction::new_signed_with_payer(
                &[init_marginfi_account_ix],
                Some(&ctx.payer.pubkey()),
                &[&ctx.payer, &account_key],
                ctx.last_blockhash,
            );
            ctx.banks_client.process_transaction(tx).await.unwrap();
        }

        MarginfiAccountFixture {
            ctx: ctx_ref,
            key: account_key.pubkey(),
        }
    }

    pub async fn try_bank_deposit<T: Into<f64>>(
        &self,
        funding_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
    ) -> anyhow::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;
        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountDeposit {
                marginfi_group: marginfi_account.group,
                marginfi_account: self.key,
                signer: ctx.payer.pubkey(),
                bank: bank.key,
                signer_token_account: funding_account,
                bank_liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
                token_program: token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingAccountDeposit {
                amount: ui_to_native!(ui_amount.into(), bank.mint.mint.decimals),
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

    pub async fn try_bank_withdraw<T: Into<f64>>(
        &self,
        destination_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
        withdraw_all: Option<bool>,
    ) -> anyhow::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;

        let mut ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountWithdraw {
                marginfi_group: marginfi_account.group,
                marginfi_account: self.key,
                signer: self.ctx.borrow().payer.pubkey(),
                bank: bank.key,
                destination_token_account: destination_account,
                bank_liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
                bank_liquidity_vault_authority: bank
                    .get_vault_authority(BankVaultType::Liquidity)
                    .0,
                token_program: token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingAccountWithdraw {
                amount: ui_to_native!(ui_amount.into(), bank.mint.mint.decimals),
                withdraw_all,
            }
            .data(),
        };

        let exclude_vec = match withdraw_all.unwrap_or(false) {
            true => vec![bank.key],
            false => vec![],
        };
        ix.accounts.extend_from_slice(
            &self
                .load_observation_account_metas(vec![], exclude_vec)
                .await,
        );

        let mut ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(())
    }

    pub async fn try_bank_borrow<T: Into<f64>>(
        &self,
        destination_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
    ) -> anyhow::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;

        let mut ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountBorrow {
                marginfi_group: marginfi_account.group,
                marginfi_account: self.key,
                signer: self.ctx.borrow().payer.pubkey(),
                bank: bank.key,
                destination_token_account: destination_account,
                bank_liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
                bank_liquidity_vault_authority: bank
                    .get_vault_authority(BankVaultType::Liquidity)
                    .0,
                token_program: token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingAccountBorrow {
                amount: ui_to_native!(ui_amount.into(), bank.mint.mint.decimals),
            }
            .data(),
        };

        ix.accounts.extend_from_slice(
            &self
                .load_observation_account_metas(vec![bank.key], vec![])
                .await,
        );

        let mut ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(())
    }

    pub async fn try_bank_repay<T: Into<f64>>(
        &self,
        funding_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
        repay_all: Option<bool>,
    ) -> anyhow::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;
        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountRepay {
                marginfi_group: marginfi_account.group,
                marginfi_account: self.key,
                signer: ctx.payer.pubkey(),
                bank: bank.key,
                signer_token_account: funding_account,
                bank_liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
                token_program: token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingAccountRepay {
                amount: ui_to_native!(ui_amount.into(), bank.mint.mint.decimals),
                repay_all,
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

    pub async fn try_liquidate<T: Into<f64>>(
        &self,
        liquidatee: &MarginfiAccountFixture,
        asset_bank_fixture: &BankFixture,
        asset_ui_amount: T,
        liab_bank_fixture: &BankFixture,
    ) -> std::result::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;

        let asset_bank = asset_bank_fixture.load().await;
        let liab_bank = liab_bank_fixture.load().await;

        let mut accounts = marginfi::accounts::LendingAccountLiquidate {
            marginfi_group: marginfi_account.group,
            asset_bank: asset_bank_fixture.key,
            liab_bank: liab_bank_fixture.key,
            liquidator_marginfi_account: self.key,
            signer: self.ctx.borrow().payer.pubkey(),
            liquidatee_marginfi_account: liquidatee.key,
            bank_liquidity_vault_authority: liab_bank_fixture
                .get_vault_authority(BankVaultType::Liquidity)
                .0,
            bank_liquidity_vault: liab_bank_fixture.get_vault(BankVaultType::Liquidity).0,
            bank_insurance_vault: liab_bank_fixture.get_vault(BankVaultType::Insurance).0,
            token_program: token::ID,
        }
        .to_account_metas(Some(true));

        accounts.extend(vec![
            AccountMeta::new_readonly(asset_bank.config.get_pyth_oracle_key(), false),
            AccountMeta::new_readonly(liab_bank.config.get_pyth_oracle_key(), false),
        ]);

        let mut ix = Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingAccountLiquidate {
                asset_amount: ui_to_native!(
                    asset_ui_amount.into(),
                    asset_bank_fixture.mint.mint.decimals
                ),
            }
            .data(),
        };

        ix.accounts.extend_from_slice(
            &self
                .load_observation_account_metas(
                    vec![asset_bank_fixture.key, liab_bank_fixture.key],
                    vec![],
                )
                .await,
        );

        ix.accounts.extend_from_slice(
            &liquidatee
                .load_observation_account_metas(vec![], vec![])
                .await,
        );

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);

        let mut ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[ix, compute_budget_ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await
    }

    pub async fn load_observation_account_metas(
        &self,
        include_banks: Vec<Pubkey>,
        exclude_banks: Vec<Pubkey>,
    ) -> Vec<AccountMeta> {
        let marginfi_account = self.load().await;
        let mut bank_pks = marginfi_account
            .lending_account
            .balances
            .iter()
            .filter_map(|balance| balance.active.then_some(balance.bank_pk))
            .collect::<Vec<_>>();

        for bank_pk in include_banks {
            if !bank_pks.contains(&bank_pk) {
                bank_pks.push(bank_pk);
            }
        }
        bank_pks.retain(|bank_pk| !exclude_banks.contains(bank_pk));

        let mut banks = vec![];
        for bank_pk in bank_pks.clone() {
            let bank = load_and_deserialize::<Bank>(self.ctx.clone(), &bank_pk).await;
            banks.push(bank);
        }

        let account_metas = banks
            .iter()
            .zip(bank_pks.iter())
            .flat_map(|(bank, bank_pk)| {
                vec![
                    AccountMeta {
                        pubkey: *bank_pk,
                        is_signer: false,
                        is_writable: false,
                    },
                    AccountMeta {
                        pubkey: bank.config.get_pyth_oracle_key(),
                        is_signer: false,
                        is_writable: false,
                    },
                ]
            })
            .collect::<Vec<_>>();
        account_metas
    }
    pub async fn set_account(&self, mfi_account: &MarginfiAccount) -> anyhow::Result<()> {
        let mut ctx = self.ctx.borrow_mut();
        let mut account = ctx.banks_client.get_account(self.key).await?.unwrap();
        let mut discriminator = account.data[..8].to_vec();
        let mut new_data = vec![];
        new_data.append(&mut discriminator);
        new_data.append(&mut bytemuck::bytes_of(mfi_account).to_vec());
        account.data = new_data;
        ctx.set_account(&self.key, &account.into());

        Ok(())
    }

    pub async fn load(&self) -> MarginfiAccount {
        load_and_deserialize::<MarginfiAccount>(self.ctx.clone(), &self.key).await
    }

    pub fn get_size() -> usize {
        mem::size_of::<MarginfiAccount>() + 8
    }
}
