use anchor_lang::{
    prelude::ToAccountMetas,
    InstructionData,
};
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use points_program as points;
use solana_program_test::{ProgramTestContext, BanksClientError};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use std::{cell::RefCell, rc::Rc};

use crate::prelude::load_and_deserialize;

pub struct PointsFixture {
    pub keypair: Keypair,
    ctx: Rc<RefCell<ProgramTestContext>>,
}

impl PointsFixture {
    pub fn new(ctx: Rc<RefCell<ProgramTestContext>>, keypair: Keypair) -> Self {
        Self { keypair, ctx }
    }

    pub async fn try_initialize_global_points(&self) -> Result<(), BanksClientError> {
        let create_ix = solana_sdk::system_instruction::create_account(
            &self.ctx.borrow().payer.pubkey(),
            &self.keypair.pubkey(),
            18_250_000_000,
            2_600_016, 
            &points::id(),
        );

        let init_ix = Instruction {
            program_id: points::id(),
            accounts: points::accounts::InitializeGlobalPoints {
                points_mapping: self.keypair.pubkey(),
                payer: self.ctx.borrow().payer.pubkey(),
                system_program: solana_program::system_program::id(),
            }.to_account_metas(Some(true)),
            data: points::instruction::InitializeGlobalPoints.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[create_ix, init_ix],
            Some(&self.ctx.borrow().payer.pubkey()),
            &[
                &self.ctx.borrow().payer,
                &self.keypair,
            ],
            self.ctx.borrow().last_blockhash,
        );

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?;

        let points_mapping = self.ctx
                            .borrow_mut()
                            .banks_client
                            .get_account(self.keypair.pubkey()).await?;

        assert!(points_mapping.is_some());

        Ok(())
    }

    pub async fn try_initialize_points_account(&self, key: Pubkey) -> Result<(), BanksClientError> {
        let ix = Instruction {
            program_id: points::id(),
            accounts: points::accounts::InitializePointsAccount {
                points_mapping: self.keypair.pubkey(),
                marginfi_account: key,
                payer: self.ctx.borrow().payer.pubkey(),
                system_program: solana_program::system_program::id(),
            }.to_account_metas(Some(true)),
            data: points::instruction::InitializePointsAccount { initial_points: 0 }.data(),
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

    pub async fn try_accrue_points(
        &self, 
        account_balance_datas: Vec<(Pubkey, points::AccountBalances)>,
        price_data: Vec<(Pubkey, f64)>,
        starting_index: usize
    ) -> Result<(), BanksClientError> {
        
        let ix = Instruction {
            program_id: points::id(),
            accounts: points::accounts::AccruePoints {
                points_mapping: self.keypair.pubkey(),
                payer: self.ctx.borrow().payer.pubkey(),
                system_program: solana_program::system_program::id(),
            }.to_account_metas(Some(true)),
            data: points::instruction::AccruePoints {
                account_balance_datas,
                price_data,
                starting_index,
            }.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.ctx.borrow().payer.pubkey()),
            &[&self.ctx.borrow().payer],
            self.ctx.borrow().last_blockhash
        );

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await?;

        Ok(())
    }

    pub async fn load(&self) -> points::PointsMapping {
        load_and_deserialize(self.ctx.clone(), &self.keypair.pubkey()).await
    }
}