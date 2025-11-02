use anchor_lang::AccountDeserialize;
use kamino_mocks::kamino_lending::accounts::{Obligation, Reserve};
use solana_program_test::ProgramTestContext;
use solana_sdk::account::ReadableAccount;
use std::{cell::RefCell, rc::Rc};

use crate::utils::load_account_from_file;

#[derive(Clone)]
pub struct KaminoFixture {
    pub reserve: Reserve,
    pub obligation: Obligation,
}

impl KaminoFixture {
    pub fn new_from_files(
        ctx: Rc<RefCell<ProgramTestContext>>,
        reserve_json_path: &str,
        obligation_json_path: &str,
    ) -> Self {
        let (reserve_key, reserve_acc) = load_account_from_file(reserve_json_path);
        let (obligation_key, obligation_acc) = load_account_from_file(obligation_json_path);

        let mut c = ctx.borrow_mut();
        c.set_account(&reserve_key, &reserve_acc);
        c.set_account(&obligation_key, &obligation_acc);

        let mut reserve_bytes: &[u8] = reserve_acc.data();
        let reserve: Reserve = AccountDeserialize::try_deserialize(&mut reserve_bytes).unwrap();

        let mut obligation_bytes: &[u8] = obligation_acc.data();
        let obligation: Obligation =
            AccountDeserialize::try_deserialize(&mut obligation_bytes).unwrap();

        Self {
            reserve,
            obligation,
        }
    }
}
