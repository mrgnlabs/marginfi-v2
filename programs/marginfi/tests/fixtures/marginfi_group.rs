#![cfg(feature = "test-bpf")]
#![allow(unused)]

use crate::fixtures::{spl::*, utils::*};
use anchor_lang::{prelude::*, solana_program::system_program, InstructionData};
use marginfi::{prelude::MarginfiGroup, state::marginfi_group::GroupConfig};
use solana_program::sysvar;
use solana_program_test::*;
use solana_sdk::{
    account::AccountSharedData, instruction::Instruction, signature::Keypair, signer::Signer,
    system_instruction, transaction::Transaction, transport::TransportError,
};
use std::{
    cell::{RefCell, RefMut},
    convert::TryInto,
    rc::Rc,
    result::Result,
};

pub struct MarginfiGroupFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
}

impl MarginfiGroupFixture {
    pub async fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        collateral_mint: &Pubkey,
        config_arg: GroupConfig,
    ) -> MarginfiGroupFixture {
        let ctx_ref = ctx.clone();

        let group_key = Keypair::new();

        {
            let mut ctx = ctx.borrow_mut();

            let accounts = marginfi::accounts::InitializeMarginfiGroup {
                marginfi_group: group_key.pubkey(),
                admin: ctx.payer.pubkey(),
                system_program: system_program::id(),
            };
            let init_marginfi_group_ix = Instruction {
                program_id: marginfi::id(),
                accounts: accounts.to_account_metas(Some(true)),
                data: marginfi::instruction::InitializeMarginfiGroup {}.data(),
            };
            let rent = ctx.banks_client.get_rent().await.unwrap();
            let size = MarginfiGroupFixture::get_size();
            let create_marginfi_group_ix = system_instruction::create_account(
                &ctx.payer.pubkey(),
                &group_key.pubkey(),
                rent.minimum_balance(size),
                size as u64,
                &marginfi::id(),
            );

            let tx = Transaction::new_signed_with_payer(
                &[create_marginfi_group_ix, init_marginfi_group_ix],
                Some(&ctx.payer.pubkey().clone()),
                &[&ctx.payer, &group_key],
                ctx.last_blockhash,
            );
            ctx.banks_client.process_transaction(tx).await.unwrap();
        }

        let tester_group = MarginfiGroupFixture {
            ctx: ctx_ref.clone(),
            key: group_key.pubkey(),
        };

        tester_group
    }

    pub fn get_size() -> usize {
        8 + bytemuck::bytes_of(&marginfi::state::marginfi_group::MarginfiGroup::default()).len()
    }

    pub async fn load(&self) -> marginfi::state::marginfi_group::MarginfiGroup {
        load_and_deserialize::<marginfi::state::marginfi_group::MarginfiGroup>(
            self.ctx.clone(),
            &self.key,
        )
        .await
    }
}
