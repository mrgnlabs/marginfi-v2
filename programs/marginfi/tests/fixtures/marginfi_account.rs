use super::prelude::*;
use anchor_lang::{prelude::*, system_program, InstructionData, ToAccountMetas};
use anchor_spl::token;
use marginfi::{
    constants::LENDING_POOL_BANK_SEED,
    state::{marginfi_account::MarginfiAccount, marginfi_group::BankVaultType},
};
use solana_program::{instruction::Instruction, system_instruction};
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
        bank_mint: Pubkey,
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
                bank_mint: bank_mint,
                bank: self
                    .find_lending_pool_bank_pda(&marginfi_account.group, &bank_mint)
                    .0,
                signer_token_account: funding_account,
                bank_liquidity_vault: find_bank_vault_pda(
                    &marginfi_account.group,
                    &bank_mint,
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
        bank_mint: Pubkey,
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
                bank_mint: bank_mint,
                bank: self
                    .find_lending_pool_bank_pda(&marginfi_account.group, &bank_mint)
                    .0,
                destination_token_account: destination_account,
                bank_liquidity_vault: find_bank_vault_pda(
                    &marginfi_account.group,
                    &bank_mint,
                    BankVaultType::Liquidity,
                )
                .0,
                bank_liquidity_vault_authority: find_bank_vault_authority_pda(
                    &marginfi_account.group,
                    &bank_mint,
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

    pub async fn try_liquidate(
        &self,
        liquidatee_pk: Pubkey,
        asset_mint: Pubkey,
        asset_amount: u64,
        liab_mint: Pubkey,
    ) -> std::result::Result<(), BanksClientError> {
        let marginfi_account = self.load().await;
        let mut ctx = self.ctx.borrow_mut();

        let mut ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::LendingAccountLiquidate {
                marginfi_group: marginfi_account.group,
                asset_mint,
                asset_bank: self
                    .find_lending_pool_bank_pda(&marginfi_account.group, &asset_mint)
                    .0,
                liab_mint,
                liab_bank: self
                    .find_lending_pool_bank_pda(&marginfi_account.group, &liab_mint)
                    .0,
                liquidator_marginfi_account: self.key,
                signer: ctx.payer.pubkey(),
                liquidatee_marginfi_account: liquidatee_pk,
                bank_liquidity_vault_authority: find_bank_vault_authority_pda(
                    &marginfi_account.group,
                    &liab_mint,
                    BankVaultType::Liquidity,
                )
                .0,
                bank_liquidity_vault: find_bank_vault_pda(
                    &marginfi_account.group,
                    &liab_mint,
                    BankVaultType::Liquidity,
                )
                .0,
                bank_insurance_vault: find_bank_vault_pda(
                    &marginfi_account.group,
                    &liab_mint,
                    BankVaultType::Insurance,
                )
                .0,
                token_program: token::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::Liquidate { asset_amount }.data(),
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
        ]);

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);

        let tx = Transaction::new_signed_with_payer(
            &[ix, compute_budget_ix],
            Some(&ctx.payer.pubkey().clone()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await
    }

    pub async fn load(&self) -> MarginfiAccount {
        load_and_deserialize::<MarginfiAccount>(self.ctx.clone(), &self.key).await
    }

    pub fn get_size() -> usize {
        mem::size_of::<MarginfiAccount>() + 8
    }

    pub fn find_lending_pool_bank_pda(
        &self,
        marginfi_group: &Pubkey,
        asset_mint: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                LENDING_POOL_BANK_SEED.as_bytes(),
                marginfi_group.as_ref(),
                &asset_mint.to_bytes(),
            ],
            &marginfi::id(),
        )
    }
}
