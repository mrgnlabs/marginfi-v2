use anchor_lang::solana_program::sysvar::instructions::{
    get_instruction_relative, load_instruction_at_checked,
};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};

use crate::constants::COMPUTE_PROGRAM_KEY;
use crate::{MarginfiError, MarginfiResult};

/// Structs that implement this trait have a `get_hash` tool that returns the function discriminator
pub trait Hashable {
    fn get_hash() -> [u8; 8];
}

/// The function of struct discriminator is constructed from these 8 bytes. Typically, the namespace  
/// is "account" or "state"
///
/// e.g. for LiquidateStart:
/// ```
///  let discrim = get_function_hash("global", "liquidate_start")
/// ```
pub fn get_discrim_hash(namespace: &str, name: &str) -> [u8; 8] {
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
    ixes: &[Instruction],
    program_id: &Pubkey,
    expected_hash: &[u8],
) -> MarginfiResult<()> {
    let compute_budget_key = COMPUTE_PROGRAM_KEY;
    let mut first_non_compute_ix_encountered = false;

    for instruction in ixes.iter() {
        if instruction.program_id == compute_budget_key {
            continue;
        }

        // Sanity check the instruction is valid
        if instruction.data.len() < 8 {
            panic!("malformed instruction");
        }
        let discrim = &instruction.data[0..8];

        // If this is the first non-compute ix, it is either the setup ix, or we fail
        if !first_non_compute_ix_encountered {
            if instruction.program_id == *program_id && discrim == expected_hash {
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
            if instruction.program_id == *program_id && discrim == expected_hash {
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
    ixes: &[Instruction],
    program_id: &Pubkey,
    expected_hash: &[u8],
) -> MarginfiResult<()> {
    let last_ix = ixes.last().unwrap(); // Safe unwrap, ixes.size() >= 1

    // Sanity check the instruction is valid
    if last_ix.data.len() < 8 {
        panic!("malformed instruction");
    }
    let discrim = &last_ix.data[0..8];

    if !last_ix.program_id.eq(program_id) || discrim != expected_hash {
        return err!(MarginfiError::EndNotLast);
    }
    Ok(())
}

// We could probably compress this into one chained iter().any(), but it would unreadable
/// Validate that no other ixes from the given program appear, other than those specified in the
/// allowlist of `expected_hashes`
pub fn validate_ixes_exclusive(
    ixes: &[Instruction],
    program_id: &Pubkey,
    expected_hashes: &[&[u8]],
) -> MarginfiResult<()> {
    // Loop over instructions just in the given program
    for ix in ixes.iter().filter(|ix| &ix.program_id == program_id) {
        // Sanity check the instruction is valid
        if ix.data.len() < 8 {
            panic!("malformed instruction");
        }
        let discrim = &ix.data[0..8];

        // If none of the allowed hashes match, reject
        let is_allowed = expected_hashes.iter().any(|&h| h == discrim);
        if !is_allowed {
            return err!(MarginfiError::ForbiddenIx);
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
///
/// * allowed_keys - Pass None to allow instructions from any program, or pass an array of keys to
///   validate only those programs occur in this tx.
pub fn load_and_validate_instructions(
    sysvar: &AccountInfo,
    allowed_keys: Option<&[Pubkey]>,
) -> Result<Vec<Instruction>> {
    let mut ixes: Vec<Instruction> = Vec::new();
    let mut ix_count = 0;
    loop {
        match load_instruction_at_checked(ix_count, sysvar) {
            Ok(instruction) => {
                if let Some(keys) = allowed_keys {
                    validate_program_allowed(&instruction, keys)?;
                }
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

#[cfg(test)]
mod tests {
    use marginfi_type_crate::constants::{discriminators, ix_discriminators};
    use pretty_assertions::assert_eq;

    use crate::{
        EndLiquidation, InitLiquidationRecord, LendingAccountRepay, LendingAccountSettleEmissions,
        LendingAccountWithdraw, LendingAccountWithdrawEmissions, StartLiquidation,
    };

    use super::*;

    #[test]
    fn check_struct_discrims_generated() {
        // ─── Bank ──────────────────────────────────────────────────────────────────
        let got_bank = get_discrim_hash("account", "Bank");
        let want_bank = discriminators::BANK;
        assert_eq!(got_bank, want_bank);

        // ─── MarginfiGroup ─────────────────────────────────────────────────────────
        let got_group = get_discrim_hash("account", "MarginfiGroup");
        let want_group = discriminators::GROUP;
        assert_eq!(got_group, want_group);

        // ─── MarginfiAccount ───────────────────────────────────────────────────────
        let got_account = get_discrim_hash("account", "MarginfiAccount");
        let want_account = discriminators::ACCOUNT;
        assert_eq!(got_account, want_account);

        // ─── FeeState ──────────────────────────────────────────────────────────────
        let got_fee_state = get_discrim_hash("account", "FeeState");
        let want_fee_state = discriminators::FEE_STATE;
        assert_eq!(got_fee_state, want_fee_state);

        // ─── StakedSettings ─────────────────────────────────────────────────────────
        let got_staked = get_discrim_hash("account", "StakedSettings");
        let want_staked = discriminators::STAKED_SETTINGS;
        assert_eq!(got_staked, want_staked);

        // ─── LiquidationRecord ─────────────────────────────────────────────────────
        let got_liquidation = get_discrim_hash("account", "LiquidationRecord");
        let want_liquidation = discriminators::LIQUIDATION_RECORD;
        assert_eq!(got_liquidation, want_liquidation);
    }

    #[test]
    fn check_instruction_hash_generated() {
        // ─── InitLiquidationRecord ───────────────────────────────────────────────
        let got_init = InitLiquidationRecord::get_hash();
        let want_init = ix_discriminators::INIT_LIQUIDATION_RECORD;
        assert_eq!(got_init, want_init);

        // ─── StartLiquidation ────────────────────────────────────────────────────
        let got_start = StartLiquidation::get_hash();
        let want_start = ix_discriminators::START_LIQUIDATION;
        assert_eq!(got_start, want_start);

        // ─── EndLiquidation ──────────────────────────────────────────────────────
        let got_end = EndLiquidation::get_hash();
        let want_end = ix_discriminators::END_LIQUIDATION;
        assert_eq!(got_end, want_end);

        // ─── LendingAccountWithdraw ──────────────────────────────────────────────
        let got_withdraw = LendingAccountWithdraw::get_hash();
        let want_withdraw = ix_discriminators::LENDING_ACCOUNT_WITHDRAW;
        assert_eq!(got_withdraw, want_withdraw);

        // ─── LendingAccountRepay ─────────────────────────────────────────────────
        let got_repay = LendingAccountRepay::get_hash();
        let want_repay = ix_discriminators::LENDING_ACCOUNT_REPAY;
        assert_eq!(got_repay, want_repay);

        // ─── LendingAccountWithdrawEmissions ─────────────────────────────────────────────────
        let got_repay = LendingAccountWithdrawEmissions::get_hash();
        let want_repay = ix_discriminators::LENDING_SETTLE_EMISSIONS;
        assert_eq!(got_repay, want_repay);

        // ─── LendingAccountSettleEmissions ─────────────────────────────────────────────────
        let got_repay = LendingAccountSettleEmissions::get_hash();
        let want_repay = ix_discriminators::LENDING_WITHDRAW_EMISSIONS;
        assert_eq!(got_repay, want_repay);
    }
}
