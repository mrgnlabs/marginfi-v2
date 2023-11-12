use super::utils::load_and_deserialize;
use crate::prelude::{
    get_emissions_authority_address, get_emissions_token_account_address, MintFixture,
    TokenAccountFixture,
};
use anchor_lang::{
    prelude::{AccountMeta, Pubkey},
    InstructionData, ToAccountMetas,
};
use fixed::types::I80F48;
use marginfi::{
    state::marginfi_group::{Bank, BankConfigOpt, BankVaultType},
    utils::{find_bank_vault_authority_pda, find_bank_vault_pda},
};
use solana_program::instruction::Instruction;
use solana_program_test::BanksClientError;
use solana_program_test::ProgramTestContext;
#[cfg(feature = "lip")]
use solana_sdk::signature::Keypair;
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

    #[cfg(feature = "lip")]
    pub async fn try_create_campaign(
        &self,
        lockup_period: u64,
        max_deposits: u64,
        max_rewards: u64,
        reward_funding_account: Pubkey,
    ) -> Result<crate::lip::LipCampaignFixture, BanksClientError> {
        use crate::prelude::lip::*;

        let campaign_key = Keypair::new();

        let bank = self.load().await;

        let ix = Instruction {
            program_id: liquidity_incentive_program::id(),
            accounts: liquidity_incentive_program::accounts::CreateCampaign {
                campaign: campaign_key.pubkey(),
                campaign_reward_vault: get_reward_vault_address(campaign_key.pubkey()).0,
                campaign_reward_vault_authority: get_reward_vault_authority(campaign_key.pubkey())
                    .0,
                asset_mint: bank.mint,
                marginfi_bank: self.key,
                admin: self.ctx.borrow().payer.pubkey(),
                funding_account: reward_funding_account,
                rent: solana_program::sysvar::rent::id(),
                token_program: anchor_spl::token::ID,
                system_program: solana_program::system_program::id(),
            }
            .to_account_metas(Some(true)),
            data: liquidity_incentive_program::instruction::CreateCampaign {
                lockup_period,
                max_deposits,
                max_rewards,
            }
            .data(),
        };

        let tx = {
            let ctx = self.ctx.borrow_mut();

            Transaction::new_signed_with_payer(
                &[ix],
                Some(&ctx.payer.pubkey()),
                &[&ctx.payer, &campaign_key],
                ctx.last_blockhash,
            )
        };

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?;

        Ok(crate::lip::LipCampaignFixture::new(
            self.ctx.clone(),
            self.clone(),
            campaign_key.pubkey(),
        ))
    }

    pub async fn try_setup_emissions(
        &self,
        flags: u64,
        rate: u64,
        total_emissions: u64,
        emissions_mint: Pubkey,
        funding_account: Pubkey,
    ) -> Result<(), BanksClientError> {
        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingPoolSetupEmissions {
                marginfi_group: self.load().await.group,
                admin: self.ctx.borrow().payer.pubkey(),
                bank: self.key,
                emissions_mint,
                emissions_funding_account: funding_account,
                emissions_auth: get_emissions_authority_address(self.key, emissions_mint).0,
                emissions_token_account: get_emissions_token_account_address(
                    self.key,
                    emissions_mint,
                )
                .0,
                token_program: anchor_spl::token::ID,
                system_program: solana_program::system_program::id(),
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingPoolSetupEmissions {
                rate,
                flags,
                total_emissions,
            }
            .data(),
        };

        let tx = {
            let ctx = self.ctx.borrow_mut();

            Transaction::new_signed_with_payer(
                &[ix],
                Some(&ctx.payer.pubkey()),
                &[&ctx.payer],
                ctx.last_blockhash,
            )
        };

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?;

        Ok(())
    }

    pub async fn try_update_emissions(
        &self,
        emissions_flags: Option<u64>,
        emissions_rate: Option<u64>,
        additional_emissions: Option<(u64, Pubkey)>,
    ) -> Result<(), BanksClientError> {
        let bank = self.load().await;

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingPoolUpdateEmissionsParameters {
                marginfi_group: self.load().await.group,
                admin: self.ctx.borrow().payer.pubkey(),
                bank: self.key,
                emissions_mint: bank.emissions_mint,
                emissions_funding_account: additional_emissions.map(|(_, f)| f).unwrap_or_default(),
                emissions_token_account: get_emissions_token_account_address(
                    self.key,
                    bank.emissions_mint,
                )
                .0,
                token_program: anchor_spl::token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::LendingPoolUpdateEmissionsParameters {
                emissions_flags,
                emissions_rate,
                additional_emissions: additional_emissions.map(|(a, _)| a),
            }
            .data(),
        };

        let tx = {
            let ctx = self.ctx.borrow_mut();

            Transaction::new_signed_with_payer(
                &[ix],
                Some(&ctx.payer.pubkey()),
                &[&ctx.payer],
                ctx.last_blockhash,
            )
        };

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

    pub async fn set_asset_share_value(&self, value: I80F48) {
        let mut bank_ai = self
            .ctx
            .borrow_mut()
            .banks_client
            .get_account(self.key)
            .await
            .unwrap()
            .unwrap();
        let bank = bytemuck::from_bytes_mut::<Bank>(&mut bank_ai.data.as_mut_slice()[8..]);

        bank.asset_share_value = value.into();

        self.ctx
            .borrow_mut()
            .set_account(&self.key, &bank_ai.into());
    }
}

impl Debug for BankFixture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BankFixture")
            .field("key", &self.key)
            .finish()
    }
}
