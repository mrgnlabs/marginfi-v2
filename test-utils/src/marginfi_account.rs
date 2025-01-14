use super::{bank::BankFixture, prelude::*};
use crate::ui_to_native;
use anchor_lang::{prelude::*, system_program, InstructionData, ToAccountMetas};

use marginfi::state::{
    bank::Bank, marginfi_account::MarginfiAccount, marginfi_group::BankVaultType,
    price::OracleSetup,
};
use solana_program::{instruction::Instruction, sysvar};
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
                authority: ctx.payer.pubkey(),
                fee_payer: ctx.payer.pubkey(),
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

    pub async fn make_bank_deposit_ix<T: Into<f64>>(
        &self,
        funding_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
    ) -> Instruction {
        let marginfi_account = self.load().await;
        let ctx = self.ctx.borrow_mut();

        let mut accounts = marginfi::accounts::LendingAccountDeposit {
            marginfi_group: marginfi_account.group,
            marginfi_account: self.key,
            signer: ctx.payer.pubkey(),
            bank: bank.key,
            signer_token_account: funding_account,
            bank_liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            token_program: bank.get_token_program(),
        }
        .to_account_metas(Some(true));
        if bank.mint.token_program == spl_token_2022::ID {
            accounts.push(AccountMeta::new_readonly(bank.mint.key, false));
        }

        Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingAccountDeposit {
                amount: ui_to_native!(ui_amount.into(), bank.mint.mint.decimals),
            }
            .data(),
        }
    }

    pub async fn try_bank_deposit<T: Into<f64> + Copy>(
        &self,
        funding_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
    ) -> anyhow::Result<(), BanksClientError> {
        let mut ix = self
            .make_bank_deposit_ix(funding_account, bank, ui_amount)
            .await;

        // If t22 with transfer hook, add remaining accounts
        let fetch_account_data_fn = |key| async move {
            Ok(self
                .ctx
                .borrow_mut()
                .banks_client
                .get_account(key)
                .await
                .map(|acc| acc.map(|a| a.data))?)
        };
        let payer = self.ctx.borrow_mut().payer.pubkey();
        if bank.mint.token_program == spl_token_2022::ID {
            // TODO: do that only if hook exists
            println!(
                "[TODO] Adding extra account metas for execute for mint {:?}",
                bank.mint.key
            );
            let _ = spl_transfer_hook_interface::offchain::add_extra_account_metas_for_execute(
                &mut ix,
                &super::transfer_hook::TEST_HOOK_ID,
                &funding_account,
                &bank.mint.key,
                &bank.get_vault(BankVaultType::Liquidity).0,
                &payer,
                ui_to_native!(ui_amount.into(), bank.mint.mint.decimals),
                fetch_account_data_fn,
            )
            .await;
        }

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

    pub async fn make_bank_withdraw_ix<T: Into<f64>>(
        &self,
        destination_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
        withdraw_all: Option<bool>,
    ) -> Instruction {
        let marginfi_account = self.load().await;

        let mut accounts = marginfi::accounts::LendingAccountWithdraw {
            marginfi_group: marginfi_account.group,
            marginfi_account: self.key,
            signer: self.ctx.borrow().payer.pubkey(),
            bank: bank.key,
            destination_token_account: destination_account,
            bank_liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            bank_liquidity_vault_authority: bank.get_vault_authority(BankVaultType::Liquidity).0,
            token_program: bank.get_token_program(),
        }
        .to_account_metas(Some(true));
        if bank.mint.token_program == spl_token_2022::ID {
            accounts.push(AccountMeta::new_readonly(bank.mint.key, false));
        }

        let mut ix = Instruction {
            program_id: marginfi::id(),
            accounts,
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

        ix
    }

    pub async fn try_bank_withdraw<T: Into<f64>>(
        &self,
        destination_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
        withdraw_all: Option<bool>,
    ) -> anyhow::Result<(), BanksClientError> {
        let ix = self
            .make_bank_withdraw_ix(destination_account, bank, ui_amount, withdraw_all)
            .await;

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

    pub async fn make_bank_borrow_ix<T: Into<f64>>(
        &self,
        destination_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
    ) -> Instruction {
        let marginfi_account = self.load().await;

        let mut accounts = marginfi::accounts::LendingAccountBorrow {
            marginfi_group: marginfi_account.group,
            marginfi_account: self.key,
            signer: self.ctx.borrow().payer.pubkey(),
            bank: bank.key,
            destination_token_account: destination_account,
            bank_liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            bank_liquidity_vault_authority: bank.get_vault_authority(BankVaultType::Liquidity).0,
            token_program: bank.get_token_program(),
        }
        .to_account_metas(Some(true));
        if bank.mint.token_program == spl_token_2022::ID {
            accounts.push(AccountMeta::new_readonly(bank.mint.key, false));
        }

        let mut ix = Instruction {
            program_id: marginfi::id(),
            accounts,
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

        ix
    }

    pub async fn try_bank_borrow<T: Into<f64> + Copy>(
        &self,
        destination_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
    ) -> anyhow::Result<(), BanksClientError> {
        self.try_bank_borrow_with_nonce(destination_account, bank, ui_amount, 100)
            .await
    }

    pub async fn try_bank_borrow_with_nonce<T: Into<f64> + Copy>(
        &self,
        destination_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
        nonce: u64,
    ) -> anyhow::Result<(), BanksClientError> {
        let mut ix = self
            .make_bank_borrow_ix(destination_account, bank, ui_amount)
            .await;

        if bank.mint.token_program == spl_token_2022::ID {
            let fetch_account_data_fn = |key| async move {
                Ok(self
                    .ctx
                    .borrow_mut()
                    .banks_client
                    .get_account(key)
                    .await
                    .map(|acc| acc.map(|a| a.data))?)
            };

            let payer = self.ctx.borrow().payer.pubkey();
            let _ = spl_transfer_hook_interface::offchain::add_extra_account_metas_for_execute(
                &mut ix,
                &super::transfer_hook::TEST_HOOK_ID,
                &bank.get_vault(BankVaultType::Liquidity).0,
                &bank.mint.key,
                &destination_account,
                &payer,
                ui_to_native!(ui_amount.into(), bank.mint.mint.decimals),
                fetch_account_data_fn,
            )
            .await;
        }

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);
        let nonce_ix = ComputeBudgetInstruction::set_compute_unit_price(nonce);

        let mut ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[compute_budget_ix, nonce_ix, ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(())
    }

    pub async fn make_bank_repay_ix<T: Into<f64>>(
        &self,
        funding_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
        repay_all: Option<bool>,
    ) -> Instruction {
        let marginfi_account = self.load().await;
        let ctx = self.ctx.borrow_mut();

        let mut accounts = marginfi::accounts::LendingAccountRepay {
            marginfi_group: marginfi_account.group,
            marginfi_account: self.key,
            signer: ctx.payer.pubkey(),
            bank: bank.key,
            signer_token_account: funding_account,
            bank_liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            token_program: bank.get_token_program(),
        }
        .to_account_metas(Some(true));
        if bank.mint.token_program == spl_token_2022::ID {
            accounts.push(AccountMeta::new_readonly(bank.mint.key, false));
        }

        Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingAccountRepay {
                amount: ui_to_native!(ui_amount.into(), bank.mint.mint.decimals),
                repay_all,
            }
            .data(),
        }
    }

    pub async fn try_bank_repay<T: Into<f64>>(
        &self,
        funding_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
        repay_all: Option<bool>,
    ) -> anyhow::Result<(), BanksClientError> {
        let ix = self
            .make_bank_repay_ix(funding_account, bank, ui_amount, repay_all)
            .await;
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

    pub async fn try_balance_close(
        &self,
        bank: &BankFixture,
    ) -> anyhow::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;
        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountCloseBalance {
                marginfi_group: marginfi_account.group,
                marginfi_account: self.key,
                signer: ctx.payer.pubkey(),
                bank: bank.key,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingAccountCloseBalance.data(),
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

    pub async fn try_liquidate<T: Into<f64> + Copy>(
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
            token_program: liab_bank_fixture.get_token_program(),
        }
        .to_account_metas(Some(true));

        if liab_bank_fixture.mint.token_program == spl_token_2022::ID {
            accounts.push(AccountMeta::new_readonly(liab_bank_fixture.mint.key, false));
        }

        let oracle_accounts = vec![asset_bank.config, liab_bank.config]
            .iter()
            .map(|config| {
                AccountMeta::new_readonly(
                    {
                        match config.oracle_setup {
                            OracleSetup::PythPushOracle => {
                                get_oracle_id_from_feed_id(config.oracle_keys[0]).unwrap()
                            }
                            _ => config.oracle_keys[0],
                        }
                    },
                    false,
                )
            })
            .collect::<Vec<AccountMeta>>();

        accounts.extend(oracle_accounts);

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

        if liab_bank_fixture.mint.token_program == spl_token_2022::ID {
            let payer = self.ctx.borrow().payer.pubkey();
            let fetch_account_data_fn = |key| async move {
                Ok(self
                    .ctx
                    .borrow_mut()
                    .banks_client
                    .get_account(key)
                    .await
                    .map(|acc| acc.map(|a| a.data))?)
            };

            let _ = spl_transfer_hook_interface::offchain::add_extra_account_metas_for_execute(
                &mut ix,
                &super::transfer_hook::TEST_HOOK_ID,
                &liab_bank_fixture.mint.key,
                &liab_bank_fixture.mint.key,
                &liab_bank_fixture.mint.key,
                &payer,
                0,
                fetch_account_data_fn,
            )
            .await;
        }

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

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);

        let mut ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[compute_budget_ix, ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await
    }

    pub async fn try_withdraw_emissions(
        &self,
        bank: &BankFixture,
        recv_account: &TokenAccountFixture,
    ) -> std::result::Result<(), BanksClientError> {
        let emissions_mint = bank.load().await.emissions_mint;
        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountWithdrawEmissions {
                marginfi_group: self.load().await.group,
                marginfi_account: self.key,
                signer: self.ctx.borrow().payer.pubkey(),
                emissions_mint,
                emissions_auth: get_emissions_authority_address(bank.key, emissions_mint).0,
                emissions_vault: get_emissions_token_account_address(bank.key, emissions_mint).0,
                destination_account: recv_account.key,
                bank: bank.key,
                token_program: recv_account.token_program,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingAccountWithdrawEmissions {}.data(),
        };

        let mut ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await
    }

    /// Set a flag on the account
    ///
    /// Function assumes signer is group admin
    pub async fn try_set_flag(&self, flag: u64) -> std::result::Result<(), BanksClientError> {
        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::SetAccountFlag {
                marginfi_group: self.load().await.group,
                marginfi_account: self.key,
                admin: self.ctx.borrow().payer.pubkey(),
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::SetAccountFlag { flag }.data(),
        };

        let mut ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await
    }

    /// Unset a flag on the account
    ///
    /// Function assumes signer is group admin
    pub async fn try_unset_flag(&self, flag: u64) -> std::result::Result<(), BanksClientError> {
        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::UnsetAccountFlag {
                marginfi_group: self.load().await.group,
                marginfi_account: self.key,
                admin: self.ctx.borrow().payer.pubkey(),
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::UnsetAccountFlag { flag }.data(),
        };

        let mut ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await
    }

    pub async fn make_lending_account_start_flashloan_ix(&self, end_index: u64) -> Instruction {
        Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountStartFlashloan {
                marginfi_account: self.key,
                signer: self.ctx.borrow().payer.pubkey(),
                ixs_sysvar: sysvar::instructions::id(),
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingAccountStartFlashloan { end_index }.data(),
        }
    }

    pub async fn make_lending_account_end_flashloan_ix(
        &self,
        include_banks: Vec<Pubkey>,
        exclude_banks: Vec<Pubkey>,
    ) -> Instruction {
        let mut account_metas = marginfi::accounts::LendingAccountEndFlashloan {
            marginfi_account: self.key,
            signer: self.ctx.borrow().payer.pubkey(),
        }
        .to_account_metas(Some(true));

        account_metas.extend(
            self.load_observation_account_metas(include_banks, exclude_banks)
                .await,
        );

        Instruction {
            program_id: marginfi::id(),
            accounts: account_metas,
            data: marginfi::instruction::LendingAccountEndFlashloan {}.data(),
        }
    }

    /// Wrap `ixs` between a start and end flashloan instruction,
    /// automatically sets the end index and send the transaction
    pub async fn try_flashloan(
        &self,
        ixs: Vec<Instruction>,
        exclude_banks: Vec<Pubkey>,
        include_banks: Vec<Pubkey>,
    ) -> std::result::Result<(), BanksClientError> {
        let mut ixs = ixs;
        let start_ix = self
            .make_lending_account_start_flashloan_ix(ixs.len() as u64 + 1)
            .await;
        let end_ix = self
            .make_lending_account_end_flashloan_ix(include_banks, exclude_banks)
            .await;

        ixs.insert(0, start_ix);
        ixs.push(end_ix);

        let mut ctx = self.ctx.borrow_mut();

        let tx = Transaction::new_signed_with_payer(
            &ixs,
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
        // Check all active banks in marginfi account balances
        let mut bank_pks = marginfi_account
            .lending_account
            .balances
            .iter()
            .filter_map(|balance| {
                if balance.active {
                    Some(balance.bank_pk)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Add bank pks in include_banks if they are not already in bank_pks
        // (and exclude the ones contained in exclude_banks)
        for bank_pk in include_banks {
            if !bank_pks.contains(&bank_pk) {
                bank_pks.push(bank_pk);
            }
        }
        bank_pks.retain(|bank_pk| !exclude_banks.contains(bank_pk));

        // Load all banks
        let mut banks = vec![];
        for bank_pk in bank_pks.clone() {
            let bank = load_and_deserialize::<Bank>(self.ctx.clone(), &bank_pk).await;
            banks.push(bank);
        }

        // Bank -> AccountMetas
        let account_metas = banks
            .iter()
            .zip(bank_pks.iter())
            .flat_map(|(bank, bank_pk)| {
                let oracle_key = {
                    let oracle_key = bank.config.oracle_keys[0];
                    match bank.config.oracle_setup {
                        OracleSetup::PythPushOracle => {
                            get_oracle_id_from_feed_id(oracle_key).unwrap()
                        }
                        _ => oracle_key,
                    }
                };

                vec![
                    AccountMeta {
                        pubkey: *bank_pk,
                        is_signer: false,
                        is_writable: false,
                    },
                    AccountMeta {
                        pubkey: oracle_key,
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

    /// Use the client to send the transfer ix authority transaction
    /// Pass the new authority as an argument
    /// Optional: use a different signer (for negative test case)
    pub async fn try_transfer_account_authority(
        &self,
        new_authority: Pubkey,
        signer_keypair: Option<Keypair>,
    ) -> std::result::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;
        let mut ctx = self.ctx.borrow_mut();
        let signer = if let Some(s) = signer_keypair {
            s
        } else {
            ctx.payer.insecure_clone()
        };

        // create instruction
        let transfer_account_authority_ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::MarginfiAccountSetAccountAuthority {
                marginfi_account: self.key,
                signer: signer.pubkey(),
                new_authority,
                fee_payer: signer.pubkey(),
                marginfi_group: marginfi_account.group,
            }
            .to_account_metas(None),
            data: marginfi::instruction::SetNewAccountAuthority {}.data(),
        };

        // create transaction
        let tx = Transaction::new_signed_with_payer(
            &[transfer_account_authority_ix],
            Some(&signer.pubkey().clone()),
            &[&signer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await
    }

    pub async fn try_close_account(&self, nonce: u64) -> std::result::Result<(), BanksClientError> {
        let mut ctx: std::cell::RefMut<ProgramTestContext> = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::MarginfiAccountClose {
                marginfi_account: self.key,
                authority: ctx.payer.pubkey(),
                fee_payer: ctx.payer.pubkey(),
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::MarginfiAccountClose {}.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ComputeBudgetInstruction::set_compute_unit_price(nonce), ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await
    }
}
