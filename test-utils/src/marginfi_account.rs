use super::{bank::BankFixture, prelude::*};
use crate::ui_to_native;
use anchor_lang::{prelude::*, system_program, InstructionData, ToAccountMetas};
use fixed::types::I80F48;
use marginfi::state::{
    marginfi_account::MarginfiAccount,
    marginfi_group::{Bank, BankVaultType},
};
use solana_program::{instruction::Instruction, sysvar};
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{
    commitment_config::CommitmentLevel, compute_budget::ComputeBudgetInstruction,
    signature::Keypair, signer::Signer, transaction::Transaction,
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
            let ctx = ctx.borrow_mut();

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
            ctx.banks_client
                .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
                .await
                .unwrap();
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
        deposit_up_to_limit: Option<bool>,
    ) -> Instruction {
        let marginfi_account = self.load().await;
        let ctx = self.ctx.borrow_mut();

        let mut accounts = marginfi::accounts::LendingAccountDeposit {
            group: marginfi_account.group,
            marginfi_account: self.key,
            authority: ctx.payer.pubkey(),
            bank: bank.key,
            signer_token_account: funding_account,
            liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            token_program: bank.get_token_program(),
        }
        .to_account_metas(Some(true));
        if bank.mint.token_program == anchor_spl::token_2022::ID {
            accounts.push(AccountMeta::new_readonly(bank.mint.key, false));
        }

        Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingAccountDeposit {
                amount: ui_to_native!(ui_amount.into(), bank.mint.mint.decimals),
                deposit_up_to_limit,
            }
            .data(),
        }
    }

    pub async fn try_bank_deposit<T: Into<f64> + Copy>(
        &self,
        funding_account: Pubkey,
        bank: &BankFixture,
        ui_amount: T,
        deposit_up_to_limit: Option<bool>,
    ) -> anyhow::Result<(), BanksClientError> {
        let mut ix = self
            .make_bank_deposit_ix(funding_account, bank, ui_amount, deposit_up_to_limit)
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
        if bank.mint.token_program == anchor_spl::token_2022::ID {
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

        let ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await?;

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
            group: marginfi_account.group,
            marginfi_account: self.key,
            authority: self.ctx.borrow().payer.pubkey(),
            bank: bank.key,
            destination_token_account: destination_account,
            liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            bank_liquidity_vault_authority: bank.get_vault_authority(BankVaultType::Liquidity).0,
            token_program: bank.get_token_program(),
        }
        .to_account_metas(Some(true));
        if bank.mint.token_program == anchor_spl::token_2022::ID {
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

        let ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await?;

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
            group: marginfi_account.group,
            marginfi_account: self.key,
            authority: self.ctx.borrow().payer.pubkey(),
            bank: bank.key,
            destination_token_account: destination_account,
            liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            bank_liquidity_vault_authority: bank.get_vault_authority(BankVaultType::Liquidity).0,
            token_program: bank.get_token_program(),
        }
        .to_account_metas(Some(true));
        if bank.mint.token_program == anchor_spl::token_2022::ID {
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

        if bank.mint.token_program == anchor_spl::token_2022::ID {
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

        let ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[compute_budget_ix, nonce_ix, ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await?;

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
            group: marginfi_account.group,
            marginfi_account: self.key,
            authority: ctx.payer.pubkey(),
            bank: bank.key,
            signer_token_account: funding_account,
            liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            token_program: bank.get_token_program(),
        }
        .to_account_metas(Some(true));
        if bank.mint.token_program == anchor_spl::token_2022::ID {
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
        let ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await?;

        Ok(())
    }

    pub async fn try_balance_close(
        &self,
        bank: &BankFixture,
    ) -> anyhow::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;
        let ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountCloseBalance {
                group: marginfi_account.group,
                marginfi_account: self.key,
                authority: ctx.payer.pubkey(),
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

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await?;

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
            group: marginfi_account.group,
            asset_bank: asset_bank_fixture.key,
            liab_bank: liab_bank_fixture.key,
            liquidator_marginfi_account: self.key,
            authority: self.ctx.borrow().payer.pubkey(),
            liquidatee_marginfi_account: liquidatee.key,
            bank_liquidity_vault_authority: liab_bank_fixture
                .get_vault_authority(BankVaultType::Liquidity)
                .0,
            bank_liquidity_vault: liab_bank_fixture.get_vault(BankVaultType::Liquidity).0,
            bank_insurance_vault: liab_bank_fixture.get_vault(BankVaultType::Insurance).0,
            token_program: liab_bank_fixture.get_token_program(),
        }
        .to_account_metas(Some(true));

        if liab_bank_fixture.mint.token_program == anchor_spl::token_2022::ID {
            accounts.push(AccountMeta::new_readonly(liab_bank_fixture.mint.key, false));
        }

        let oracle_accounts = vec![asset_bank.config, liab_bank.config]
            .iter()
            .map(|config| {
                AccountMeta::new_readonly(
                    {
                        get_oracle_id_from_feed_id(config.oracle_keys[0])
                            .unwrap_or(config.oracle_keys[0])
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

        if liab_bank_fixture.mint.token_program == anchor_spl::token_2022::ID {
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

        let ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[compute_budget_ix, ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await
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
                group: self.load().await.group,
                marginfi_account: self.key,
                authority: self.ctx.borrow().payer.pubkey(),
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

        let ctx = self.ctx.borrow_mut();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await
    }

    pub async fn make_lending_account_start_flashloan_ix(&self, end_index: u64) -> Instruction {
        Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountStartFlashloan {
                marginfi_account: self.key,
                authority: self.ctx.borrow().payer.pubkey(),
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
            authority: self.ctx.borrow().payer.pubkey(),
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
        signer: Option<&Keypair>,
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

        let ctx = self.ctx.borrow_mut();

        let signers = if let Some(signer) = signer {
            vec![&ctx.payer, signer]
        } else {
            vec![&ctx.payer]
        };

        let tx = Transaction::new_signed_with_payer(
            &ixs,
            Some(&ctx.payer.pubkey().clone()),
            &signers,
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await
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
                if balance.is_active() {
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

        // Sort all bank_pks in descending order
        bank_pks.sort_by(|a, b| b.cmp(a));

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
                    get_oracle_id_from_feed_id(oracle_key).unwrap_or(oracle_key)
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

    async fn build_transfer_account(
        &self,
        new_marginfi_account: Pubkey,
        new_authority: Pubkey,
        signer_keypair: Option<Keypair>,
        new_account_keypair: &Keypair,
        global_fee_wallet: Pubkey,
    ) -> Transaction {
        let marginfi_account = self.load().await;
        let ctx = self.ctx.borrow();
        let signer = signer_keypair.unwrap_or_else(|| ctx.payer.insecure_clone());

        let transfer_account_ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::TransferToNewAccount {
                old_marginfi_account: self.key,
                new_marginfi_account,
                group: marginfi_account.group,
                authority: signer.pubkey(),
                new_authority,
                global_fee_wallet,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: marginfi::instruction::TransferToNewAccount {}.data(),
        };

        Transaction::new_signed_with_payer(
            &[transfer_account_ix],
            Some(&signer.pubkey()),
            &[&signer, new_account_keypair],
            ctx.last_blockhash,
        )
    }

    /// Build and send the “transfer TransferToNewAccount transaction.
    /// Pass the new authority as an argument
    /// Optional: use a different signer (for negative test case)
    pub async fn try_transfer_account(
        &self,
        new_marginfi_account: Pubkey,
        new_authority: Pubkey,
        signer_keypair: Option<Keypair>,
        new_account_keypair: &Keypair,
        global_fee_wallet: Pubkey,
    ) -> std::result::Result<(), BanksClientError> {
        let tx = self
            .build_transfer_account(
                new_marginfi_account,
                new_authority,
                signer_keypair,
                new_account_keypair,
                global_fee_wallet,
            )
            .await;
        let ctx = self.ctx.borrow_mut();
        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await
    }

    /// Build (but don’t send) the “transfer TransferToNewAccount transaction.
    /// Pass the new authority as an argument
    /// Optional: use a different signer (for negative test case)
    pub async fn get_tx_transfer_account(
        &self,
        new_marginfi_account: Pubkey,
        new_authority: Pubkey,
        signer_keypair: Option<Keypair>,
        new_account_keypair: &Keypair,
        global_fee_wallet: Pubkey,
    ) -> Transaction {
        self.build_transfer_account(
            new_marginfi_account,
            new_authority,
            signer_keypair,
            new_account_keypair,
            global_fee_wallet,
        )
        .await
    }

    pub async fn try_close_account(&self, nonce: u64) -> std::result::Result<(), BanksClientError> {
        let ctx: std::cell::RefMut<ProgramTestContext> = self.ctx.borrow_mut();

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

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await
    }

    pub async fn nullify_assets_for_bank(&mut self, bank_pk: Pubkey) -> anyhow::Result<()> {
        let mut user_mfi_account: MarginfiAccount = self.load().await;

        let balance_index = user_mfi_account
            .lending_account
            .balances
            .iter()
            .position(|b| b.is_active() && b.bank_pk == bank_pk)
            .unwrap();

        user_mfi_account.lending_account.balances[balance_index].asset_shares = I80F48::ZERO.into();
        self.set_account(&user_mfi_account).await
    }
}
