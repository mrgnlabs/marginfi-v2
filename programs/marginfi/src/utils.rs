use crate::{bank_authority_seed, bank_seed, state::marginfi_group::BankVaultType};
use anchor_lang::prelude::Pubkey;

pub fn find_bank_vault_pda(bank_pk: &Pubkey, vault_type: BankVaultType) -> (Pubkey, u8) {
    Pubkey::find_program_address(bank_seed!(vault_type, bank_pk), &crate::id())
}

pub fn find_bank_vault_authority_pda(bank_pk: &Pubkey, vault_type: BankVaultType) -> (Pubkey, u8) {
    Pubkey::find_program_address(bank_authority_seed!(vault_type, bank_pk), &crate::id())
}
