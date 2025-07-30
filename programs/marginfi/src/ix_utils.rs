use anchor_lang::solana_program::sysvar::instructions::{
    get_instruction_relative, load_instruction_at_checked,
};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};

use crate::constants::COMPUTE_PROGRAM_KEY;
use crate::{MarginfiError, MarginfiResult};

// TODO implement this on start, end, withdraw, repay, etc.
/// Structs that implement this trait have a `get_hash` tool that returns the function discriminator
pub trait Hashable {
    fn get_hash() -> [u8; 8];
}

/// The function discrminator is constructed from these 8 bytes. Typically, the namespace is
/// "global" or "state"
pub fn get_function_hash(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{}:{}", namespace, name);
    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(
        &anchor_lang::solana_program::hash::hash(preimage.as_bytes()).to_bytes()[..8],
    );
    sighash
}

/// Validate the given ix hash is the first in the list of ixes, other than compute budget ixes,
/// and appears only once.
///
/// If the ix implements `Hashable`, use `get_hash()` to get the expected hash.
pub fn validate_ix_first(
    ixes: &Vec<Instruction>,
    program_id: &Pubkey,
    expected_hash: [u8; 8],
) -> MarginfiResult<()> {
    let compute_budget_key = COMPUTE_PROGRAM_KEY;
    let mut first_non_compute_ix_encountered = false;

    for instruction in ixes.iter() {
        if instruction.program_id == compute_budget_key {
            continue;
        }

        // The remainder of the data is the args, which don't matter for validation
        let ix_data_func = &instruction.data[0..8];

        // If this is the first non-compute ix, it is either the setup ix, or we fail
        if !first_non_compute_ix_encountered {
            if instruction.program_id == *program_id && ix_data_func == expected_hash {
                first_non_compute_ix_encountered = true;
            } else {
                msg!(
                    "Expected ix from program: {:?} w/ hash: {:?}",
                    program_id,
                    expected_hash
                );
                msg!(
                    "Got ix from program: {:?} w/ hash: {:?}",
                    instruction.program_id,
                    instruction.data,
                );
                msg!("Start IX was not the first ix (other than compute).");
                return err!(MarginfiError::StartNotFirst);
            }
        } else {
            // Already validated the first ix, check that the start ix does not appear again
            if instruction.program_id == *program_id && ix_data_func == expected_hash {
                msg!("Setup IX appears more than once.");
                return err!(MarginfiError::StartRepeats);
            }
        }
    }

    if first_non_compute_ix_encountered {
        Ok(())
    } else {
        msg!("Start IX was not found in the TX.");
        err!(MarginfiError::StartNotFirst)
    }
}

/// Validate the given ix hash is the last ix in the list of ixes.
pub fn validate_ix_last(
    ixes: &Vec<Instruction>,
    program_id: &Pubkey,
    expected_hash: [u8; 8],
) -> MarginfiResult<()> {
    let last_ix = ixes.last().unwrap(); // Safe unwrap, ixes.size() >= 1
    if !last_ix.program_id.eq(program_id) || last_ix.data != expected_hash {
        return err!(MarginfiError::EndNotLast);
    }
    Ok(())
}

/// Validate that no other ixes from the given program appear, other than those specified in the
/// allowlist of `expected_hashes`
pub fn validate_ixes_exclusive(
    ixes: &[Instruction],
    program_id: &Pubkey,
    // TODO start, end, the various withdraw/repay functions...
    expected_hashes: [[u8; 8]; 2],
) -> MarginfiResult<()> {
    for ix in ixes {
        if &ix.program_id == program_id {
            let ix_data = &ix.data[0..8];

            // Check if the instruction matches any of the allowed hashes
            let is_expected_hash = expected_hashes
                .iter()
                .any(|expected_hash| ix_data == expected_hash);

            if !is_expected_hash {
                return err!(MarginfiError::ForbiddenIx);
            }
        }
    }

    Ok(())
}

/*** We might use these later for something like to limit the swap venue to e.g. Jup */

/// Panics if the top-level relative instruction is not the Marginfi program
pub fn validate_not_cpi(sysvar: &AccountInfo) {
    let mrgn_key = id_crate::ID;
    let index_relative_to_current: i64 = 0;
    let instruction_sysvar_account_info = sysvar;
    let current_ix =
        get_instruction_relative(index_relative_to_current, instruction_sysvar_account_info)
            .unwrap();
    if current_ix.program_id != mrgn_key {
        panic!("This ix is not permitted within a CPI");
    }
}

/// The instruction uses one of the given hard-coded allowed programs.
pub fn validate_program_allowed(
    instruction: &Instruction,
    allowed_keys: &[Pubkey],
) -> MarginfiResult<()> {
    let id = &instruction.program_id;
    if !allowed_keys.iter().any(|key| key.eq(id)) {
        return err!(MarginfiError::ForbiddenIx);
    }
    Ok(())
}

/// Validate that all instructions in the tx belong to an allowed program key. Returns a Vec of ixes
/// in the same order that they appear in the tx.
pub fn load_and_validate_instructions(
    sysvar: &AccountInfo,
    allowed_keys: &[Pubkey],
) -> Result<Vec<Instruction>> {
    let mut ixes: Vec<Instruction> = Vec::new();
    let mut ix_count = 0;
    loop {
        match load_instruction_at_checked(ix_count, sysvar) {
            Ok(instruction) => {
                validate_program_allowed(&instruction, allowed_keys)?;
                ixes.push(instruction);
                ix_count += 1;
            }
            Err(ProgramError::InvalidArgument) => break, // Passed last ix, stop loop
            Err(e) => {
                msg!("unexpected error {:?}", e);
                panic!("Error reading some ix");
            }
        }
    }
    Ok(ixes)
}
