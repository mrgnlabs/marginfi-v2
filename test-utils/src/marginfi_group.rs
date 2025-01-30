use super::{bank::BankFixture, marginfi_account::MarginfiAccountFixture};
use crate::prelude::{get_oracle_id_from_feed_id, MintFixture};
use crate::utils::*;
use anchor_lang::{prelude::*, solana_program::system_program, InstructionData};

use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anyhow::Result;
use bytemuck::bytes_of;
use marginfi::constants::{
    FEE_STATE_SEED, INIT_BANK_ORIGINATION_FEE_DEFAULT, PROTOCOL_FEE_FIXED_DEFAULT,
    PROTOCOL_FEE_RATE_DEFAULT,
};
use marginfi::state::fee_state::FeeState;
use marginfi::state::marginfi_group::BankConfigCompact;
use marginfi::state::price::OracleSetup;
use marginfi::{
    prelude::MarginfiGroup,
    state::marginfi_group::{BankConfig, BankConfigOpt, BankVaultType, GroupConfig},
};
use solana_program::sysvar;
use solana_program_test::*;
use solana_sdk::system_transaction;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, instruction::Instruction, signature::Keypair,
    signer::Signer, transaction::Transaction,
};
use std::{cell::RefCell, mem, rc::Rc};

async fn airdrop_sol(context: &mut ProgramTestContext, key: &Pubkey, amount: u64) {
    let recent_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = system_transaction::transfer(&context.payer, key, amount, recent_blockhash);
    context.banks_client.process_transaction(tx).await.unwrap();
}

pub struct MarginfiGroupFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
    pub fee_state: Pubkey,
    pub fee_wallet: Pubkey,
}

impl MarginfiGroupFixture {
    pub async fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        config: GroupConfig,
    ) -> MarginfiGroupFixture {
        let ctx_ref = ctx.clone();

        let group_key = Keypair::new();
        let fee_wallet_key: Pubkey;
        let (fee_state_key, _bump) =
            Pubkey::find_program_address(&[FEE_STATE_SEED.as_bytes()], &marginfi::id());

        {
            let mut ctx = ctx.borrow_mut();

            let initialize_marginfi_group_ix = Instruction {
                program_id: marginfi::id(),
                accounts: marginfi::accounts::MarginfiGroupInitialize {
                    marginfi_group: group_key.pubkey(),
                    admin: ctx.payer.pubkey(),
                    fee_state: fee_state_key,
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

            // Check if the fee state account already exists
            let fee_state_account = ctx.banks_client.get_account(fee_state_key).await.unwrap();

            // Account exists, read it and proceed with group initialization
            if let Some(account) = fee_state_account {
                if !account.data.is_empty() {
                    // Deserialize the account data to extract the fee_wallet public key
                    let fee_state_data: FeeState =
                        FeeState::try_deserialize(&mut &account.data[..]).unwrap();
                    fee_wallet_key = fee_state_data.global_fee_wallet;

                    let tx = Transaction::new_signed_with_payer(
                        &[initialize_marginfi_group_ix, configure_marginfi_group_ix],
                        Some(&ctx.payer.pubkey().clone()),
                        &[&ctx.payer, &group_key],
                        ctx.last_blockhash,
                    );
                    ctx.banks_client.process_transaction(tx).await.unwrap();
                } else {
                    panic!("Fee state exists but is empty")
                }
            } else {
                // Account does not exist, proceed with group and fee state initialization
                let fee_wallet = Keypair::new();
                // The wallet needs some sol to be rent exempt
                airdrop_sol(&mut ctx, &fee_wallet.pubkey(), 1_000_000).await;
                fee_wallet_key = fee_wallet.pubkey();

                let init_fee_state_ix = Instruction {
                    program_id: marginfi::id(),
                    accounts: marginfi::accounts::InitFeeState {
                        payer: ctx.payer.pubkey(),
                        fee_state: fee_state_key,
                        rent: sysvar::rent::id(),
                        system_program: system_program::id(),
                    }
                    .to_account_metas(Some(true)),
                    data: marginfi::instruction::InitGlobalFeeState {
                        admin: ctx.payer.pubkey(),
                        fee_wallet: fee_wallet.pubkey(),
                        bank_init_flat_sol_fee: INIT_BANK_ORIGINATION_FEE_DEFAULT,
                        program_fee_fixed: PROTOCOL_FEE_FIXED_DEFAULT.into(),
                        program_fee_rate: PROTOCOL_FEE_RATE_DEFAULT.into(),
                    }
                    .data(),
                };

                let tx = Transaction::new_signed_with_payer(
                    &[
                        init_fee_state_ix,
                        initialize_marginfi_group_ix,
                        configure_marginfi_group_ix,
                    ],
                    Some(&ctx.payer.pubkey().clone()),
                    &[&ctx.payer, &group_key],
                    ctx.last_blockhash,
                );
                ctx.banks_client.process_transaction(tx).await.unwrap();
            }
        }

        MarginfiGroupFixture {
            ctx: ctx_ref.clone(),
            key: group_key.pubkey(),
            fee_state: fee_state_key,
            fee_wallet: fee_wallet_key,
        }
    }

    /// Adds bank and configures the oracle.
    ///
    /// Note: AddBank and LendingPoolConfigureBankOracle were seperated to handle a tx size issue in
    /// squads. This test fixture packs both ixes into one tx as is typical outside of squads.
    pub async fn try_lending_pool_add_bank(
        &self,
        bank_asset_mint_fixture: &MintFixture,
        bank_config: BankConfig,
    ) -> Result<BankFixture, BanksClientError> {
        let bank_key = Keypair::new();
        let bank_mint = bank_asset_mint_fixture.key;
        let bank_fixture =
            BankFixture::new(self.ctx.clone(), bank_key.pubkey(), bank_asset_mint_fixture);
        let config_compact: BankConfigCompact = bank_config.into();

        let accounts = marginfi::accounts::LendingPoolAddBank {
            marginfi_group: self.key,
            admin: self.ctx.borrow().payer.pubkey(),
            fee_payer: self.ctx.borrow().payer.pubkey(),
            fee_state: self.fee_state,
            global_fee_wallet: self.fee_wallet,
            bank_mint,
            bank: bank_key.pubkey(),
            liquidity_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Liquidity).0,
            liquidity_vault: bank_fixture.get_vault(BankVaultType::Liquidity).0,
            insurance_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Insurance).0,
            insurance_vault: bank_fixture.get_vault(BankVaultType::Insurance).0,
            fee_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Fee).0,
            fee_vault: bank_fixture.get_vault(BankVaultType::Fee).0,
            rent: sysvar::rent::id(),
            token_program: bank_asset_mint_fixture.token_program,
            system_program: system_program::id(),
        }
        .to_account_metas(Some(true));

        let init_ix = Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingPoolAddBank {
                bank_config: config_compact,
            }
            .data(),
        };

        let feed_oracle = {
            if bank_config.oracle_setup == OracleSetup::PythPushOracle
                || bank_config.oracle_setup == OracleSetup::StakedWithPythPush
            {
                Some(get_oracle_id_from_feed_id(bank_config.oracle_keys[0]).unwrap())
            } else {
                None
            }
        };

        let config_oracle_ix = self.make_lending_pool_configure_bank_oracle_ix(
            &bank_fixture,
            bank_config.oracle_setup as u8,
            bank_config.oracle_keys[0],
            feed_oracle,
        );

        let tx = Transaction::new_signed_with_payer(
            &[init_ix, config_oracle_ix],
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

    /// Adds bank and configures the oracle.
    ///
    /// Note: AddBank and LendingPoolConfigureBankOracle were seperated to handle a tx size issue in
    /// squads. This test fixture packs both ixes into one tx as is typical outside of squads.
    pub async fn try_lending_pool_add_bank_with_seed(
        &self,
        bank_asset_mint_fixture: &MintFixture,
        bank_config: BankConfig,
        bank_seed: u64,
    ) -> Result<BankFixture, BanksClientError> {
        let bank_mint = bank_asset_mint_fixture.key;

        // Create PDA account from seeds
        let (pda, _bump) = Pubkey::find_program_address(
            [
                self.key.as_ref(),
                bank_mint.as_ref(),
                &bank_seed.to_le_bytes(),
            ]
            .as_slice(),
            &marginfi::id(),
        );

        let bank_mint = bank_asset_mint_fixture.key;
        let bank_fixture = BankFixture::new(self.ctx.clone(), pda, bank_asset_mint_fixture);
        let config_compact: BankConfigCompact = bank_config.into();

        let accounts = marginfi::accounts::LendingPoolAddBankWithSeed {
            marginfi_group: self.key,
            admin: self.ctx.borrow().payer.pubkey(),
            fee_payer: self.ctx.borrow().payer.pubkey(),
            fee_state: self.fee_state,
            global_fee_wallet: self.fee_wallet,
            bank_mint,
            bank: pda,
            liquidity_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Liquidity).0,
            liquidity_vault: bank_fixture.get_vault(BankVaultType::Liquidity).0,
            insurance_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Insurance).0,
            insurance_vault: bank_fixture.get_vault(BankVaultType::Insurance).0,
            fee_vault_authority: bank_fixture.get_vault_authority(BankVaultType::Fee).0,
            fee_vault: bank_fixture.get_vault(BankVaultType::Fee).0,
            rent: sysvar::rent::id(),
            token_program: bank_fixture.get_token_program(),
            system_program: system_program::id(),
        }
        .to_account_metas(Some(true));

        let init_ix = Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingPoolAddBankWithSeed {
                bank_config: config_compact,
                bank_seed,
            }
            .data(),
        };

        let feed_oracle = {
            if bank_config.oracle_setup == OracleSetup::PythPushOracle
                || bank_config.oracle_setup == OracleSetup::StakedWithPythPush
            {
                let id = get_oracle_id_from_feed_id(bank_config.oracle_keys[0]);
                if id.is_none() {
                    panic!("Unsupported Pyth feed ID, this should never happen");
                }
                id
            } else {
                None
            }
        };

        let config_oracle_ix = self.make_lending_pool_configure_bank_oracle_ix(
            &bank_fixture,
            bank_config.oracle_setup as u8,
            bank_config.oracle_keys[0],
            feed_oracle,
        );

        let tx = Transaction::new_signed_with_payer(
            &[init_ix, config_oracle_ix],
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
        let accounts = marginfi::accounts::LendingPoolConfigureBank {
            bank: bank.key,
            marginfi_group: self.key,
            admin: self.ctx.borrow().payer.pubkey(),
        }
        .to_account_metas(Some(true));

        Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingPoolConfigureBank { bank_config_opt }.data(),
        }
    }

    pub fn make_lending_pool_configure_bank_oracle_ix(
        &self,
        bank: &BankFixture,
        setup: u8,
        oracle: Pubkey,
        feed_oracle: Option<Pubkey>,
    ) -> Instruction {
        let mut accounts = marginfi::accounts::LendingPoolConfigureBankOracle {
            bank: bank.key,
            group: self.key,
            admin: self.ctx.borrow().payer.pubkey(),
        }
        .to_account_metas(Some(true));

        accounts.push(AccountMeta::new_readonly(
            feed_oracle.unwrap_or(oracle),
            false,
        ));

        Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::LendingPoolConfigureBankOracle { setup, oracle }.data(),
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

    pub async fn try_update(&self, config: GroupConfig) -> Result<(), BanksClientError> {
        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::MarginfiGroupConfigure {
                marginfi_group: self.key,
                admin: self.ctx.borrow().payer.pubkey(),
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::MarginfiGroupConfigure { config }.data(),
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

        Ok(())
    }

    pub async fn try_collect_fees(&self, bank: &BankFixture) -> Result<()> {
        let mut ctx = self.ctx.borrow_mut();

        let fee_ata = get_associated_token_address_with_program_id(
            &self.fee_wallet,
            &bank.mint.key,
            &bank.get_token_program(),
        );

        let mut accounts = marginfi::accounts::LendingPoolCollectBankFees {
            marginfi_group: self.key,
            bank: bank.key,
            liquidity_vault_authority: bank.get_vault_authority(BankVaultType::Liquidity).0,
            liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            insurance_vault: bank.get_vault(BankVaultType::Insurance).0,
            fee_vault: bank.get_vault(BankVaultType::Fee).0,
            token_program: bank.get_token_program(),
            fee_state: self.fee_state,
            fee_ata,
        }
        .to_account_metas(Some(true));
        if bank.mint.token_program == spl_token_2022::ID {
            accounts.push(AccountMeta::new_readonly(bank.mint.key, false));
        }

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts,
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
        self.try_handle_bankruptcy_with_nonce(bank, marginfi_account, 100)
            .await
    }

    pub async fn try_handle_bankruptcy_with_nonce(
        &self,
        bank: &BankFixture,
        marginfi_account: &MarginfiAccountFixture,
        nonce: u64,
    ) -> Result<(), BanksClientError> {
        let mut accounts = marginfi::accounts::LendingPoolHandleBankruptcy {
            marginfi_group: self.key,
            signer: self.ctx.borrow().payer.pubkey(),
            bank: bank.key,
            marginfi_account: marginfi_account.key,
            liquidity_vault: bank.get_vault(BankVaultType::Liquidity).0,
            insurance_vault: bank.get_vault(BankVaultType::Insurance).0,
            insurance_vault_authority: bank.get_vault_authority(BankVaultType::Insurance).0,
            token_program: bank.get_token_program(),
        }
        .to_account_metas(Some(true));
        if bank.mint.token_program == spl_token_2022::ID {
            accounts.push(AccountMeta::new_readonly(bank.mint.key, false));
        }

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

        let nonce_ix = ComputeBudgetInstruction::set_compute_unit_price(nonce);

        let tx = Transaction::new_signed_with_payer(
            &[ix, nonce_ix],
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

    pub async fn set_protocol_fees_flag(&self, enabled: bool) {
        let mut group = self.load().await;
        let mut ctx = self.ctx.borrow_mut();
        let mut account = ctx
            .banks_client
            .get_account(self.key)
            .await
            .unwrap()
            .unwrap();

        group.group_flags = if enabled { 1 } else { 0 };

        let data = bytes_of(&group);

        account.data[8..].copy_from_slice(data);

        ctx.set_account(&self.key, &account.into())
    }
}
