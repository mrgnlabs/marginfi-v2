use crate::{
    bank_authority_seed, bank_seed, check,
    constants::IXS_SYSVAR_MARGINFI_ACCOUNT_INDEX,
    prelude::{MarginfiError, MarginfiResult},
    state::marginfi_group::BankVaultType,
};
use anchor_lang::{
    prelude::{AccountInfo, Pubkey},
    Discriminator,
};
use solana_program::sysvar::instructions::{
    load_current_index_checked, load_instruction_at_checked,
};

pub fn find_bank_vault_pda(bank_pk: &Pubkey, vault_type: BankVaultType) -> (Pubkey, u8) {
    Pubkey::find_program_address(bank_seed!(vault_type, bank_pk), &crate::id())
}

pub fn find_bank_vault_authority_pda(bank_pk: &Pubkey, vault_type: BankVaultType) -> (Pubkey, u8) {
    Pubkey::find_program_address(bank_authority_seed!(vault_type, bank_pk), &crate::id())
}

/// Verifies that the an instruction to end the flashloan exists in the transactions
///
/// This check is critical for a flashloan start instruction
///
/// ## Flashloan security model
/// In a flashloan health checks of a marginfi account are done once at the end of the transaction.
/// This gives the user much more flexiblity in how to use the marginfi.
///
/// This works throug 2 mechanisms.
/// One mechanism is diabling health checks on the marignfi account during the flashloan,
/// and the other mechanism is ensuring that the health check is performed by the end of the flashloan.
///
/// This achieved with two instructions:
/// - Flashloan Start: This instruction sets a flag on the marginfi account to disable health checks, and the verifies that a valid corresponding Flashloan End instruction exists in the same transaction.
/// - Flashloan End: Sets a flag to enable health checks on a marginfi account, and perfoms an intialization health check.
///
/// The Flashloan Start additionally checks that the marginfi account isn't already in a flashloan and the Flashloan End checks that the account is in a flashloan,
/// I'm not aware of any attack vectors this is covering, its mostly there as a sanity check, and will prevent multiple flashloans overlapping.
///
/// Assumptions enforced by this check
/// - There exists a flashloan end instruction (FE) in the transaction.
/// - The FE is for the same program.
/// - The FE is positioned after the current instruction (flashloan start - FS) in the transaction.
/// - The FE is matched by the FE instruction discriminator.
/// - The FE and FS are for the same marginfi account, the FE ix has the marginfi account positioned at the IXS_SYSVAR_MARGINFI_ACCOUNT_INDEX (1) in the account metas.
///
pub fn verify_flashloan_ixs_sysvar(
    sysvar_ai: &AccountInfo,
    end_index: usize,
    marginfi_account_pk: &Pubkey,
    program_id: &Pubkey,
) -> MarginfiResult {
    let end_ix = load_instruction_at_checked(end_index, sysvar_ai)?;

    check!(
        &end_ix.program_id == program_id,
        MarginfiError::FlashloanIxsSysvarInvalid
    );

    check!(
        end_ix.data[..8] == crate::instruction::MarginfiAccountFlashloanEnd::DISCRIMINATOR,
        MarginfiError::FlashloanIxsSysvarInvalid
    );

    let current_ix_idx = load_current_index_checked(sysvar_ai)? as usize;

    check!(
        end_index > current_ix_idx,
        MarginfiError::FlashloanIxsSysvarInvalid
    );

    let marginfi_account_account_meta = end_ix
        .accounts
        .get(IXS_SYSVAR_MARGINFI_ACCOUNT_INDEX)
        .unwrap();

    check!(
        marginfi_account_account_meta.pubkey.eq(marginfi_account_pk),
        MarginfiError::FlashloanIxsSysvarInvalid
    );

    Ok(())
}
