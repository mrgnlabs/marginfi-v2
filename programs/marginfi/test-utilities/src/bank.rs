use crate::utils::{get_shares_token_mint, get_shares_token_mint_authority};

use super::utils::load_and_deserialize;
use anchor_lang::{
    prelude::{AccountMeta, Pubkey},
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::token::spl_token;
use marginfi::{
    state::marginfi_group::{Bank, BankConfigOpt, BankVaultType},
    utils::{find_bank_vault_authority_pda, find_bank_vault_pda},
};
use solana_program::instruction::Instruction;
use solana_program_test::{BanksClientError, ProgramTestContext, ProgramTestError};
use solana_sdk::{signer::Signer, transaction::Transaction};
use std::{cell::RefCell, fmt::Debug, rc::Rc};

#[derive(Clone)]
pub struct BankFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
}

impl BankFixture {
    pub fn new(ctx: Rc<RefCell<ProgramTestContext>>, key: Pubkey) -> Self {
        Self { ctx, key }
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

    pub async fn try_mint_shares(
        &self,
        amount: u64,
        asset_token_account: Pubkey,
        shares_token_account: Pubkey,
    ) -> anyhow::Result<(), BanksClientError> {
        let bank = self.load().await;

        let (shares_token_mint, _) = get_shares_token_mint(&self.key);
        let (shares_token_mint_authority, _) = get_shares_token_mint_authority(&self.key);

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::BankMintShares {
                marginfi_group: bank.group,
                bank: self.key,
                shares_token_mint,
                shares_token_mint_authority,
                liquidity_vault: bank.liquidity_vault,
                signer: self.ctx.borrow().payer.pubkey(),
                user_deposit_token_account: asset_token_account,
                user_shares_token_account: shares_token_account,
                token_program: spl_token::id(),
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::BankMintShares { amount }.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.ctx.borrow().payer.pubkey()),
            &[&self.ctx.borrow().payer],
            self.ctx.borrow().last_blockhash,
        );

        Ok(self
            .ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?)
    }

    pub async fn try_redeem_shares(
        &self,
        amount: u64,
        asset_token_account: Pubkey,
        shares_token_account: Pubkey,
    ) -> anyhow::Result<(), BanksClientError> {
        let bank = self.load().await;

        let (shares_token_mint, _) = get_shares_token_mint(&self.key);

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::BankRedeemShares {
                marginfi_group: bank.group,
                bank: self.key,
                shares_token_mint,
                liquidity_vault: bank.liquidity_vault,
                signer: self.ctx.borrow().payer.pubkey(),
                user_deposit_token_account: asset_token_account,
                user_shares_token_account: shares_token_account,
                bank_liquidity_vault_authority: self
                    .get_vault_authority(BankVaultType::Liquidity)
                    .0,
                token_program: spl_token::id(),
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::BankRedeemShares {
                shares_amount: amount,
            }
            .data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.ctx.borrow().payer.pubkey()),
            &[&self.ctx.borrow().payer],
            self.ctx.borrow().last_blockhash,
        );

        Ok(self
            .ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?)
    }
}

impl Debug for BankFixture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BankFixture")
            .field("key", &self.key)
            .finish()
    }
}
