use crate::utils::*;
use anchor_lang::{
    prelude::{Pubkey, ToAccountMetas},
    InstructionData,
};
use anyhow::Result;
use lip::*;
use solana_program::instruction::Instruction;
use solana_program_test::ProgramTestContext;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use std::{cell::RefCell, rc::Rc};

use crate::{
    bank::BankFixture,
    prelude::{get_shares_token_mint, get_shares_token_mint_authority},
};

pub struct LipCampaignFixture {
    pub key: Pubkey,
    bank_f: BankFixture,
    ctx: Rc<RefCell<ProgramTestContext>>,
}

impl LipCampaignFixture {
    pub async fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        lockup_period: u64,
        max_deposits: u64,
        max_rewards: u64,
        funding_account: Pubkey,
        bank_fixture: &BankFixture,
    ) -> Self {
        let campaign_key = Keypair::new();

        let bank = bank_fixture.load().await;

        let ix = Instruction {
            program_id: lip::id(),
            accounts: lip::accounts::CreateCampaign {
                campaign: campaign_key.pubkey(),
                campaign_reward_vault: get_reward_vault_address(campaign_key.pubkey()).0,
                campaign_reward_vault_authority: get_reward_vault_authority_address(
                    campaign_key.pubkey(),
                )
                .0,
                asset_mint: bank.mint,
                marginfi_bank: bank_fixture.key,
                admin: ctx.borrow().payer.pubkey(),
                funding_account,
                rent: anchor_lang::solana_program::sysvar::rent::id(),
                token_program: anchor_spl::token::ID,
                system_program: solana_program::system_program::id(),
            }
            .to_account_metas(Some(true)),
            data: lip::instruction::CreateCampaing {
                lockup_period,
                max_deposits,
                max_rewards,
            }
            .data(),
        };

        let tx = {
            let ctx = ctx.borrow_mut();

            Transaction::new_signed_with_payer(
                &[ix],
                Some(&ctx.payer.pubkey()),
                &[&ctx.payer, &campaign_key],
                ctx.last_blockhash,
            )
        };

        ctx.borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        Self {
            key: campaign_key.pubkey(),
            bank_f: bank_fixture.clone(),
            ctx,
        }
    }

    pub async fn try_deposit(&self, funding_account: Pubkey, amount: u64) -> Result<()> {
        let bank = self.bank_f.load().await;
        let deposit_key = Keypair::new();

        let ix = Instruction {
            program_id: lip::id(),
            accounts: lip::accounts::CreateDeposit {
                campaign: self.key,
                signer: self.ctx.borrow().payer.pubkey(),
                deposit: deposit_key.pubkey(),
                deposit_shares_vault: get_deposit_shares_vault_address(self.key).0,
                deposit_shares_vault_authority: get_deposit_shares_vault_authority_address(
                    self.key,
                )
                .0,
                funding_account,
                marginfi_group: bank.group,
                marginfi_bank: self.bank_f.key,
                marginfi_bank_vault: bank.liquidity_vault,
                marginfi_shares_mint: get_shares_token_mint(&self.bank_f.key).0,
                marginfi_shares_mint_authority: get_shares_token_mint_authority(&self.bank_f.key).0,
                marginfi_program: marginfi::id(),
                token_program: anchor_spl::token::ID,
                rent: anchor_lang::solana_program::sysvar::rent::id(),
                system_program: solana_program::system_program::id(),
            }
            .to_account_metas(Some(true)),
            data: lip::instruction::CreateDeposit { amount }.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.ctx.borrow().payer.pubkey()),
            &[&self.ctx.borrow().payer, &deposit_key],
            self.ctx.borrow().last_blockhash,
        );

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        Ok(())
    }

    pub async fn try_end_deposit(
        &self,
        deposit_key: Pubkey,
        destination_account: Pubkey,
    ) -> Result<()> {
        let bank = self.bank_f.load().await;

        let ix = Instruction {
            program_id: lip::id(),
            accounts: lip::accounts::CloseDeposit {
                campaign: self.key,
                signer: self.ctx.borrow().payer.pubkey(),
                campaign_reward_vault: get_reward_vault_address(self.key).0,
                campaign_reward_vault_authority: get_reward_vault_authority_address(self.key).0,
                deposit: deposit_key,
                deposit_shares_vault: get_deposit_shares_vault_address(deposit_key).0,
                deposit_shares_vault_authority: get_deposit_shares_vault_authority_address(
                    deposit_key,
                )
                .0,
                ephemeral_token_account: get_ephemeral_token_account_address(deposit_key).0,
                ephemeral_token_account_authority: get_ephemeral_token_account_authority_address(
                    deposit_key,
                )
                .0,
                destination_account,
                asset_mint: bank.mint,
                marginfi_group: bank.group,
                marginfi_bank: self.bank_f.key,
                marginfi_bank_vault: bank.liquidity_vault,
                marginfi_bank_vault_authority: self
                    .bank_f
                    .get_vault_authority(marginfi::state::marginfi_group::BankVaultType::Liquidity)
                    .0,
                marginfi_shares_mint: get_shares_token_mint(&self.bank_f.key).0,
                marginfi_program: marginfi::id(),
                token_program: anchor_spl::token::ID,
                system_program: solana_program::system_program::id(),
            }
            .to_account_metas(Some(true)),
            data: lip::instruction::CloseDeposit {}.data(),
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
            .await
            .unwrap();

        Ok(())
    }
}
