use crate::{
    bank_authority_seed, bank_seed, state::marginfi_group::BankVaultType, MarginfiError,
    MarginfiResult,
};
use anchor_lang::{prelude::Pubkey, Id};
use anchor_spl::{
    token::Token,
    token_2022::spl_token_2022::{
        self,
        extension::{
            transfer_fee::TransferFeeConfig, BaseStateWithExtensions, StateWithExtensions,
        },
    },
};
use fixed::types::I80F48;
use switchboard_solana::AccountInfo;

pub fn find_bank_vault_pda(bank_pk: &Pubkey, vault_type: BankVaultType) -> (Pubkey, u8) {
    Pubkey::find_program_address(bank_seed!(vault_type, bank_pk), &crate::id())
}

pub fn find_bank_vault_authority_pda(bank_pk: &Pubkey, vault_type: BankVaultType) -> (Pubkey, u8) {
    Pubkey::find_program_address(bank_authority_seed!(vault_type, bank_pk), &crate::id())
}

pub trait NumTraitsWithTolerance<T> {
    fn is_zero_with_tolerance(&self, t: T) -> bool;
    fn is_positive_with_tolerance(&self, t: T) -> bool;
}

impl<T> NumTraitsWithTolerance<T> for I80F48
where
    I80F48: PartialOrd<T>,
{
    fn is_zero_with_tolerance(&self, t: T) -> bool {
        self.abs() < t
    }

    fn is_positive_with_tolerance(&self, t: T) -> bool {
        self.gt(&t)
    }
}

pub fn calculate_spl_deposit_amount(
    mint_ai: AccountInfo,
    target_amount: u64,
    epoch: u64,
) -> MarginfiResult<u64> {
    if mint_ai.owner.eq(&Token::id()) {
        return Ok(target_amount);
    }

    let mint_data = mint_ai.try_borrow_data()?;
    let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;

    let fee = if let Ok(transfer_fee_config) = mint.get_extension::<TransferFeeConfig>() {
        transfer_fee_config
            .calculate_inverse_epoch_fee(epoch, target_amount)
            .unwrap()
    } else {
        0
    };

    let deposit_amount = target_amount
        .checked_add(fee)
        .ok_or(MarginfiError::MathError)?;

    Ok(deposit_amount)
}
