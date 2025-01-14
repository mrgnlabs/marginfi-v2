use crate::{
    bank_authority_seed, bank_seed,
    constants::{ASSET_TAG_DEFAULT, ASSET_TAG_SOL, ASSET_TAG_STAKED},
    state::{bank::Bank, marginfi_account::MarginfiAccount, marginfi_group::BankVaultType},
    MarginfiError, MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    token::Token,
    token_2022::spl_token_2022::{
        self,
        extension::{
            transfer_fee::{TransferFee, TransferFeeConfig},
            BaseStateWithExtensions, StateWithExtensions,
        },
    },
    token_interface::Mint,
};
use fixed::types::I80F48;

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

    match mint.get_extension::<TransferFeeConfig>() {
        Ok(transfer_fee_config) => {
            let epoch_fee = transfer_fee_config.get_epoch_fee(epoch);
            let pre_fee_amount = calculate_pre_fee_amount(epoch_fee, post_fee_amount).unwrap();
            Ok(pre_fee_amount)
        }
        Err(_) => Ok(post_fee_amount),
    }
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
        return Ok(u16::from(
            transfer_fee_config
                .get_epoch_fee(epoch)
                .transfer_fee_basis_points,
        ) != 0);
    }

    Ok(false)
}

/// Checks if first account is a mint account. If so, updates remaining_account -> &remaining_account[1..]
///
/// Ok(None) if Tokenkeg
pub fn maybe_take_bank_mint<'info>(
    remaining_accounts: &mut &'info [AccountInfo<'info>],
    bank: &Bank,
    token_program: &Pubkey,
) -> MarginfiResult<Option<InterfaceAccount<'info, Mint>>> {
    match *token_program {
        anchor_spl::token::ID => Ok(None),
        anchor_spl::token_2022::ID => {
            let (maybe_mint, remaining) = remaining_accounts
                .split_first()
                .ok_or(MarginfiError::T22MintRequired)?;
            *remaining_accounts = remaining;

            if bank.mint != *maybe_mint.key {
                return err!(MarginfiError::T22MintRequired);
            }

            InterfaceAccount::try_from(maybe_mint)
                .map(Option::Some)
                .map_err(|e| {
                    msg!("failed to parse mint account: {:?}", e);
                    MarginfiError::T22MintRequired.into()
                })
        }

        _ => panic!("unsupported token program"),
    }
}

const ONE_IN_BASIS_POINTS: u128 = 10_000;
/// backported fix from
/// https://github.com/solana-labs/solana-program-library/commit/20e6792179fc7f1251579c1c33a4a0feec48e15e
pub fn calculate_pre_fee_amount(transfer_fee: &TransferFee, post_fee_amount: u64) -> Option<u64> {
    let maximum_fee = u64::from(transfer_fee.maximum_fee);
    let transfer_fee_basis_points = u16::from(transfer_fee.transfer_fee_basis_points) as u128;
    match (transfer_fee_basis_points, post_fee_amount) {
        // no fee, same amount
        (0, _) => Some(post_fee_amount),
        // 0 zero out, 0 in
        (_, 0) => Some(0),
        // 100%, cap at max fee
        (ONE_IN_BASIS_POINTS, _) => maximum_fee.checked_add(post_fee_amount),
        _ => {
            let numerator = (post_fee_amount as u128).checked_mul(ONE_IN_BASIS_POINTS)?;
            let denominator = ONE_IN_BASIS_POINTS.checked_sub(transfer_fee_basis_points)?;
            let raw_pre_fee_amount = ceil_div(numerator, denominator)?;

            if raw_pre_fee_amount.checked_sub(post_fee_amount as u128)? >= maximum_fee as u128 {
                post_fee_amount.checked_add(maximum_fee)
            } else {
                // should return `None` if `pre_fee_amount` overflows
                u64::try_from(raw_pre_fee_amount).ok()
            }
        }
    }
}

// Private function from spl-program-library
fn ceil_div(numerator: u128, denominator: u128) -> Option<u128> {
    numerator
        .checked_add(denominator)?
        .checked_sub(1)?
        .checked_div(denominator)
}

/// A minimal tool to convert a hex string like "22f123639" into the byte equivalent.
pub fn hex_to_bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks(2)
        .map(|chunk| {
            let high = chunk[0] as char;
            let low = chunk[1] as char;
            let high = high.to_digit(16).expect("Invalid hex character") as u8;
            let low = low.to_digit(16).expect("Invalid hex character") as u8;
            (high << 4) | low
        })
        .collect()
}

/// Validate that after a deposit to Bank, the users's account contains either all Default/SOL
/// balances, or all Staked/Sol balances. Default and Staked assets cannot mix.
pub fn validate_asset_tags(bank: &Bank, marginfi_account: &MarginfiAccount) -> MarginfiResult {
    let mut has_default_asset = false;
    let mut has_staked_asset = false;

    for balance in marginfi_account.lending_account.balances.iter() {
        if balance.active {
            match balance.bank_asset_tag {
                ASSET_TAG_DEFAULT => has_default_asset = true,
                ASSET_TAG_SOL => { /* Do nothing, SOL can mix with any asset type */ }
                ASSET_TAG_STAKED => has_staked_asset = true,
                _ => panic!("unsupported asset tag"),
            }
        }
    }

    // 1. Regular assets (DEFAULT) cannot mix with Staked assets
    if bank.config.asset_tag == ASSET_TAG_DEFAULT && has_staked_asset {
        return err!(MarginfiError::AssetTagMismatch);
    }

    // 2. Staked SOL cannot mix with Regular asset (DEFAULT)
    if bank.config.asset_tag == ASSET_TAG_STAKED && has_default_asset {
        return err!(MarginfiError::AssetTagMismatch);
    }

    Ok(())
}

/// Validate that two banks are compatible based on their asset tags. See the following combinations
/// (* is wildcard, e.g. any tag):
///
/// Allowed:
/// 1) Default/Default
/// 2) Sol/*
/// 3) Staked/Staked
///
/// Forbidden:
/// 1) Default/Staked
///
/// Returns an error if the two banks have mismatching asset tags according to the above.
pub fn validate_bank_asset_tags(bank_a: &Bank, bank_b: &Bank) -> MarginfiResult {
    let is_bank_a_default = bank_a.config.asset_tag == ASSET_TAG_DEFAULT;
    let is_bank_a_staked = bank_a.config.asset_tag == ASSET_TAG_STAKED;
    let is_bank_b_default = bank_b.config.asset_tag == ASSET_TAG_DEFAULT;
    let is_bank_b_staked = bank_b.config.asset_tag == ASSET_TAG_STAKED;
    // Note: Sol is compatible with all other tags and doesn't matter...

    // 1. Default assets cannot mix with Staked assets
    if is_bank_a_default && is_bank_b_staked {
        return err!(MarginfiError::AssetTagMismatch);
    }
    if is_bank_a_staked && is_bank_b_default {
        return err!(MarginfiError::AssetTagMismatch);
    }

    Ok(())
}
