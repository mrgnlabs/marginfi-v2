use super::prelude::*;
use anchor_lang::{prelude::*, system_program, InstructionData, ToAccountMetas};
use anchor_spl::token;
use marginfi::state::{marginfi_account::MarginfiAccount, marginfi_group::BankVaultType};
use solana_program::{instruction::Instruction, system_instruction};
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
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

            let accounts = marginfi::accounts::InitializeMarginfiAccount {
                marginfi_account: account_key.pubkey(),
                marginfi_group: *marginfi_group,
                signer: ctx.payer.pubkey(),
                system_program: system_program::ID,
            };
            let init_marginfi_account_ix = Instruction {
                program_id: marginfi::id(),
                accounts: accounts.to_account_metas(Some(true)),
                data: marginfi::instruction::InitializeMarginfiAccount {}.data(),
            };
            let rent = ctx.banks_client.get_rent().await.unwrap();
            let size = MarginfiAccountFixture::get_size();
            let create_marginfi_account_ix = system_instruction::create_account(
                &ctx.payer.pubkey(),
                &account_key.pubkey(),
                rent.minimum_balance(size),
                size as u64,
                &marginfi::id(),
            );

            let tx = Transaction::new_signed_with_payer(
                &[create_marginfi_account_ix, init_marginfi_account_ix],
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

    pub async fn try_bank_deposit(
        &self,
        bank_asset_mint: Pubkey,
        funding_account: Pubkey,
        amount: u64,
    ) -> anyhow::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;

        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::BankDeposit {
                marginfi_group: marginfi_account.group,
                marginfi_account: self.key,
                signer: ctx.payer.pubkey(),
                asset_mint: bank_asset_mint,
                signer_token_account: funding_account,
                bank_liquidity_vault: find_bank_vault_pda(
                    &marginfi_account.group,
                    &bank_asset_mint,
                    BankVaultType::Liquidity,
                )
                .0,
                token_program: token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::BankDeposit { amount }.data(),
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

    pub async fn try_bank_withdraw(
        &self,
        bank_asset_mint: Pubkey,
        destination_account: Pubkey,
        amount: u64,
    ) -> anyhow::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;

        let mut ctx = self.ctx.borrow_mut();

        let mut ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::BankWithdraw {
                marginfi_group: marginfi_account.group,
                marginfi_account: self.key,
                signer: ctx.payer.pubkey(),
                asset_mint: bank_asset_mint,
                destination_token_account: destination_account,
                bank_liquidity_vault: find_bank_vault_pda(
                    &marginfi_account.group,
                    &bank_asset_mint,
                    BankVaultType::Liquidity,
                )
                .0,
                bank_liquidity_vault_authority: find_bank_vault_authority_pda(
                    &marginfi_account.group,
                    &bank_asset_mint,
                    BankVaultType::Liquidity,
                )
                .0,
                token_program: token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::BankWithdraw { amount }.data(),
        };
        ix.accounts.extend_from_slice(&[
            AccountMeta {
                pubkey: PYTH_USDC_FEED,
                is_signer: false,
                is_writable: false,
            },
            AccountMeta {
                pubkey: PYTH_SOL_FEED,
                is_signer: false,
                is_writable: false,
            },
        ]); // Need to generalise. SDK!

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(())
    }

    pub async fn load(&self) -> MarginfiAccount {
        load_and_deserialize::<MarginfiAccount>(self.ctx.clone(), &self.key).await
    }

    pub fn get_size() -> usize {
        mem::size_of::<MarginfiAccount>() + 8
    }
}
