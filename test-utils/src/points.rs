use anchor_lang::AnchorDeserialize;
use anchor_lang::{
    prelude::{Pubkey, ToAccountMetas},
    InstructionData,
};
use solana_program::instruction::Instruction;
use solana_program_test::ProgramTestContext;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use std::{cell::RefCell, rc::Rc};

pub struct PointsFixture {
    pub key: Pubkey,
    ctx: Rc<RefCell<ProgramTestContext>>,
}

impl PointsFixture {
    pub fn new(ctx: Rc<RefCell<ProgramTestContext>>, key: Pubkey) -> Self {
        Self { key, ctx }
    }

    pub async fn try_accrue_points(&self) -> Result<()> {
        

        Ok(())
    }

}