#![cfg(feature = "lip")]

use crate::utils::lip::*;
use anchor_lang::AnchorDeserialize;
use anchor_lang::{
    prelude::{Pubkey, ToAccountMetas},
    InstructionData,
};
use anyhow::Result;
use liquidity_incentive_program as lip;
use solana_program::instruction::Instruction;
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use std::{cell::RefCell, rc::Rc};

use crate::bank::BankFixture;

pub struct LipCampaignFixture {
    pub key: Pubkey,
    bank_f: BankFixture,
    ctx: Rc<RefCell<ProgramTestContext>>,
}

impl LipCampaignFixture {
    pub fn new(ctx: Rc<RefCell<ProgramTestContext>>, bank_f: BankFixture, key: Pubkey) -> Self {
        Self { key, bank_f, ctx }
    }

    pub async fn try_create_deposit(
        &self,
        funding_account: Pubkey,
        amount: u64,
    ) -> Result<Pubkey, BanksClientError> {
        let bank = self.bank_f.load().await;
        let deposit_key = Keypair::new();
        let temp_token_account_key = Keypair::new();

        let ix = Instruction {
            program_id: lip::id(),
            accounts: lip::accounts::CreateDeposit {
                campaign: self.key,
                signer: self.ctx.borrow().payer.pubkey(),
                deposit: deposit_key.pubkey(),
                mfi_pda_signer: get_deposit_mfi_authority(deposit_key.pubkey()).0,
                funding_account,
                temp_token_account: temp_token_account_key.pubkey(),
                asset_mint: bank.mint,
                marginfi_group: bank.group,
                marginfi_bank: self.bank_f.key,
                marginfi_account: get_marginfi_account_address(deposit_key.pubkey()).0,
                marginfi_bank_vault: bank.liquidity_vault,
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
            &[
                &self.ctx.borrow().payer,
                &deposit_key,
                &temp_token_account_key,
            ],
            self.ctx.borrow().last_blockhash,
        );

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?;

        Ok(deposit_key.pubkey())
    }

    pub async fn try_end_deposit(
        &self,
        deposit_pk: Pubkey,
        destination_account_address: Pubkey,
    ) -> Result<()> {
        let bank = self.bank_f.load().await;
        let temp_token_account_key = Keypair::new();

        let ix = Instruction {
            program_id: lip::id(),
            accounts: lip::accounts::EndDeposit {
                campaign: self.key,
                campaign_reward_vault: get_reward_vault_address(self.key).0,
                campaign_reward_vault_authority: get_reward_vault_authority(self.key).0,
                signer: self.ctx.borrow().payer.pubkey(),
                deposit: deposit_pk,
                mfi_pda_signer: get_deposit_mfi_authority(deposit_pk).0,
                temp_token_account: temp_token_account_key.pubkey(),
                temp_token_account_authority: get_temp_token_account_authority(deposit_pk).0,
                destination_account: destination_account_address,
                asset_mint: bank.mint,
                marginfi_account: get_marginfi_account_address(deposit_pk).0,
                marginfi_group: bank.group,
                marginfi_bank: self.bank_f.key,
                marginfi_bank_vault: bank.liquidity_vault,
                marginfi_bank_vault_authority: self
                    .bank_f
                    .get_vault_authority(marginfi::state::marginfi_group::BankVaultType::Liquidity)
                    .0,
                marginfi_program: marginfi::id(),
                token_program: anchor_spl::token::ID,
                system_program: solana_program::system_program::id(),
            }
            .to_account_metas(Some(true)),
            data: lip::instruction::EndDeposit {}.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.ctx.borrow().payer.pubkey()),
            &[&self.ctx.borrow().payer, &temp_token_account_key],
            self.ctx.borrow().last_blockhash,
        );

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?;

        Ok(())
    }

    pub async fn load(&self) -> lip::state::Campaign {
        let account = self
            .ctx
            .borrow_mut()
            .banks_client
            .get_account(self.key)
            .await
            .unwrap()
            .unwrap();

        lip::state::Campaign::deserialize(&mut &account.data[8..]).unwrap()
    }

    pub async fn load_deposit(&self, deposit_key: Pubkey) -> lip::state::Deposit {
        let account = self
            .ctx
            .borrow_mut()
            .banks_client
            .get_account(deposit_key)
            .await
            .unwrap()
            .unwrap();

        lip::state::Deposit::deserialize(&mut &account.data[8..]).unwrap()
    }
}
