use anchor_lang::prelude::{AccountInfo, Clock, Pubkey};
use lazy_static::lazy_static;
use solana_program::{entrypoint::ProgramResult, instruction::Instruction, program_stubs};

use crate::log;

lazy_static! {
    static ref VERBOSE: u32 = std::env::var("FUZZ_VERBOSE")
        .map(|s| s.parse())
        .ok()
        .transpose()
        .ok()
        .flatten()
        .unwrap_or(0);
}

pub struct TestSyscallStubs {
    pub unix_timestamp: Option<i64>,
}

impl program_stubs::SyscallStubs for TestSyscallStubs {
    fn sol_log(&self, message: &str) {
        if *VERBOSE == 0 {
            return;
        }
        log!("Program Log: {}", message);
    }

    fn sol_log_data(&self, fields: &[&[u8]]) {
        if *VERBOSE == 0 {
            return;
        }
        log!("data: {}", fields.iter().map(base64::encode).join(" "));
    }

    fn sol_invoke_signed(
        &self,
        instruction: &Instruction,
        account_infos: &[AccountInfo],
        signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        let mut new_account_infos = vec![];

        for meta in instruction.accounts.iter() {
            for account_info in account_infos.iter() {
                if meta.pubkey == *account_info.key {
                    let mut new_account_info = account_info.clone();
                    for seeds in signers_seeds.iter() {
                        let signer =
                            Pubkey::create_program_address(seeds, &marginfi::id()).unwrap();
                        if *account_info.key == signer {
                            new_account_info.is_signer = true;
                        }
                    }
                    new_account_infos.push(new_account_info);
                }
            }
        }

        spl_token::processor::Processor::process(
            &instruction.program_id,
            &new_account_infos,
            &instruction.data,
        )
    }

    fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
        let clock: Option<i64> = self.unix_timestamp;
        unsafe {
            *(var_addr as *mut _ as *mut Clock) = Clock {
                unix_timestamp: clock.unwrap(),
                ..Clock::default()
            };
        }
        solana_program::entrypoint::SUCCESS
    }
}

pub fn test_syscall_stubs(unix_timestamp: Option<i64>) {
    program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs { unix_timestamp }));
}
