use crate::{
    bank_authority_seed, bank_seed,
    state::marginfi_group::{Bank, BankVaultType},
    MarginfiError, MarginfiResult,
};
use anchor_lang::{
    prelude::{InterfaceAccount, Pubkey},
    Id,
};
use anchor_spl::{
    token::Token,
    token_2022::spl_token_2022::{
        self,
        extension::{
            transfer_fee::TransferFeeConfig, BaseStateWithExtensions, StateWithExtensions,
        },
    },
    token_interface::Mint,
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

pub fn calculate_pre_fee_spl_deposit_amount(
    mint_ai: AccountInfo,
    post_fee_amount: u64,
    epoch: u64,
) -> MarginfiResult<u64> {
    if mint_ai.owner.eq(&Token::id()) {
        return Ok(post_fee_amount);
    }

    let mint_data = mint_ai.try_borrow_data()?;
    let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;

    let fee = if let Ok(transfer_fee_config) = mint.get_extension::<TransferFeeConfig>() {
        transfer_fee_config
            .calculate_inverse_epoch_fee(epoch, post_fee_amount)
            .unwrap()
    } else {
        0
    };

    let pre_fee_amount = post_fee_amount
        .checked_add(fee)
        .ok_or(MarginfiError::MathError)?;

    Ok(pre_fee_amount)
}

pub fn calculate_post_fee_spl_deposit_amount(
    mint_ai: AccountInfo,
    input_amount: u64,
    epoch: u64,
) -> MarginfiResult<u64> {
    if mint_ai.owner.eq(&Token::id()) {
        return Ok(input_amount);
    }

    let mint_data = mint_ai.try_borrow_data()?;
    let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;

    let fee = if let Ok(transfer_fee_config) = mint.get_extension::<TransferFeeConfig>() {
        transfer_fee_config
            .calculate_epoch_fee(epoch, input_amount)
            .unwrap()
    } else {
        0
    };

    let output_amount = input_amount
        .checked_sub(fee)
        .ok_or(MarginfiError::MathError)?;

    Ok(output_amount)
}

pub fn nonzero_fee(mint_ai: AccountInfo, epoch: u64) -> MarginfiResult<bool> {
    if mint_ai.owner.eq(&Token::id()) {
        return Ok(false);
    }

    let mint_data = mint_ai.try_borrow_data()?;
    let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;

    if let Ok(transfer_fee_config) = mint.get_extension::<TransferFeeConfig>() {
        return Ok(u16::from_le_bytes(
            transfer_fee_config
                .get_epoch_fee(epoch)
                .transfer_fee_basis_points
                .0,
        ) != 0);
    }

    Ok(false)
}

/// Checks if first account is a mint account. If so, updates remaining_account -> &remaining_account[1..]
pub fn maybe_get_bank_mint<'info>(
    remaining_accounts: &mut &'info [AccountInfo<'info>],
    bank: &Bank,
) -> Option<InterfaceAccount<'info, Mint>> {
    let (maybe_mint, remaining) = remaining_accounts.split_first()?;

    if bank.mint != *maybe_mint.key {
        return None;
    }

    if let Ok(mint) = InterfaceAccount::try_from(maybe_mint) {
        *remaining_accounts = remaining;
        return Some(mint);
    }

    None
}
