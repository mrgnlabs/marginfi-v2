use std::{cell::RefCell, rc::Rc};

use anchor_lang::{InstructionData, ToAccountMetas};
use fixed::types::I80F48;
use marginfi::state::{
    liquid_insurance_fund::{LiquidInsuranceFund, LiquidInsuranceFundAccount},
    marginfi_group::BankVaultType,
};
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{
    account::AccountSharedData, compute_budget::ComputeBudgetInstruction, instruction::Instruction,
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_program, transaction::Transaction,
};
use switchboard_solana::AccountMeta;

use crate::{bank::BankFixture, prelude::TokenAccountFixture};

pub struct LiquidInsuranceFundFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
    data: Vec<u8>,
}

impl LiquidInsuranceFundFixture {
    pub fn new(ctx: Rc<RefCell<ProgramTestContext>>, key: Pubkey) -> LiquidInsuranceFundFixture {
        LiquidInsuranceFundFixture {
            ctx,
            key,
            data: vec![],
        }
    }

    pub async fn load<'a>(&'a mut self) -> &'a LiquidInsuranceFund {
        let mut ctx = self.ctx.borrow_mut();

        // Fetch account
        let account = ctx
            .banks_client
            .get_account(self.key)
            .await
            .unwrap()
            .unwrap();

        self.data = account.data;

        // Deserialize
        bytemuck::from_bytes(&self.data[8..])
    }

    pub async fn create_lif_user_account(
        &self,
    ) -> Result<LiquidInsuranceFundAccountFixture, BanksClientError> {
        let mut ctx = self.ctx.borrow_mut();
        let user = Keypair::new();
        // Give user some SOL
        ctx.set_account(
            &user.pubkey(),
            &AccountSharedData::new(1_000_000_000, 0, &system_program::ID),
        );

        let lif_account = LiquidInsuranceFundAccount::address(&user.pubkey());
        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::CreateLiquidInsuranceFundAccount {
                user_insurance_fund_account: lif_account,
                signer: user.pubkey(),
                system_program: system_program::ID,
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::CreateLiquidInsuranceFundAccount {}.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &user],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(LiquidInsuranceFundAccountFixture {
            ctx: self.ctx.clone(),
            user,
            key: lif_account,
            data: vec![],
        })
    }
}

pub struct LiquidInsuranceFundAccountFixture {
    pub ctx: Rc<RefCell<ProgramTestContext>>,
    user: Keypair,
    key: Pubkey,
    data: Vec<u8>,
}

impl LiquidInsuranceFundAccountFixture {
    #[inline]
    fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        user: Keypair,
    ) -> LiquidInsuranceFundAccountFixture {
        LiquidInsuranceFundAccountFixture {
            ctx,
            key: LiquidInsuranceFundAccount::address(&user.pubkey()),
            user,
            data: vec![],
        }
    }

    pub fn user(&self) -> &Keypair {
        &self.user
    }

    pub async fn load<'a>(&'a mut self) -> &'a mut LiquidInsuranceFundAccount {
        let mut ctx = self.ctx.borrow_mut();

        // Fetch account
        let account = ctx
            .banks_client
            .get_account(self.key)
            .await
            .unwrap()
            .unwrap();

        self.data = account.data;

        // Deserialize
        bytemuck::from_bytes_mut(&mut self.data[8..])
    }

    pub async fn try_user_deposit_insurance(
        &self,
        bank: &BankFixture,
        source_account: &TokenAccountFixture,
        amount: u64,
    ) -> Result<(), BanksClientError> {
        let mut ctx = self.ctx.borrow_mut();

        let mut accounts = marginfi::accounts::DepositIntoLiquidInsuranceFund {
            token_program: bank.get_token_program(),
            liquid_insurance_fund: LiquidInsuranceFund::address(&bank.key),
            signer: self.user.pubkey(),
            signer_token_account: source_account.key,
            system_program: system_program::ID,
            bank_insurance_vault: bank.get_vault(BankVaultType::Insurance).0,
            user_insurance_fund_account: LiquidInsuranceFundAccount::address(&self.user.pubkey()),
        }
        .to_account_metas(Some(true));

        if source_account.token_program == spl_token_2022::ID {
            accounts.push(AccountMeta::new(source_account.token.mint, false));
        }

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::DepositIntoLiquidInsuranceFund { amount }.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.user.pubkey()),
            &[&self.user],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(())
    }

    pub async fn try_user_withdraw_insurance_request(
        &self,
        bank: &BankFixture,
        shares: Option<I80F48>,
        nonce: u64,
    ) -> Result<(), BanksClientError> {
        let mut ctx = self.ctx.borrow_mut();

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts: marginfi::accounts::WithdrawRequestLiquidInsuranceFund {
                user_insurance_fund_account: self.key,
                liquid_insurance_fund: LiquidInsuranceFund::address(&bank.key),
                signer: self.user.pubkey(),
            }
            .to_account_metas(Some(true)),
            data: marginfi::instruction::CreateWithdrawRequestFromLiquidTokenFund {
                amount: shares.map(|s| s.into()),
            }
            .data(),
        };

        let nonce_ix = ComputeBudgetInstruction::set_compute_unit_price(nonce);
        let tx = Transaction::new_signed_with_payer(
            &[ix, nonce_ix],
            Some(&self.user.pubkey()),
            &[&self.user],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(())
    }

    pub async fn try_user_withdraw_insurance_claim(
        &self,
        bank: &BankFixture,
        dest_token_account: &TokenAccountFixture,
        nonce: u64,
    ) -> Result<(), BanksClientError> {
        let mut ctx = self.ctx.borrow_mut();

        let mut accounts = marginfi::accounts::SettleWithdrawClaimInLiquidInsuranceFund {
            user_insurance_fund_account: self.key,
            signer_token_account: dest_token_account.key,
            bank_insurance_vault: bank.get_vault(BankVaultType::Insurance).0,
            bank_insurance_vault_authority: bank.get_vault_authority(BankVaultType::Insurance).0,
            token_program: dest_token_account.token_program,
            liquid_insurance_fund: LiquidInsuranceFund::address(&bank.key),
            signer: self.user.pubkey(),
        }
        .to_account_metas(Some(true));

        if dest_token_account.token_program == spl_token_2022::ID {
            accounts.push(AccountMeta::new(dest_token_account.token.mint, false));
        }

        let ix = Instruction {
            program_id: marginfi::id(),
            accounts,
            data: marginfi::instruction::SettleWithdrawClaimInLiquidInsuranceFund {}.data(),
        };
        let nonce_ix = ComputeBudgetInstruction::set_compute_unit_price(nonce);

        let tx = Transaction::new_signed_with_payer(
            &[nonce_ix, ix],
            Some(&self.user.pubkey()),
            &[&self.user],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;

        Ok(())
    }
}
