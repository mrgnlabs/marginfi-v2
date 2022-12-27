#![cfg(feature = "test")]
#![allow(dead_code)]

use anchor_lang::prelude::*;
use marginfi::{
    bank_authority_seed, bank_seed,
    constants::{LENDING_POOL_BANK_SEED, PYTH_ID},
    state::marginfi_group::BankVaultType,
};
use pyth_sdk_solana::state::{AccountType, PriceAccount, PriceInfo, PriceStatus, MAGIC, VERSION_2};
use solana_program::instruction::Instruction;
use solana_program_test::*;
use solana_sdk::{account::Account, signature::Keypair};
use std::{cell::RefCell, rc::Rc};

pub const MS_PER_SLOT: u64 = 400;
pub const RUST_LOG_DEFAULT: &str = "solana_rbpf::vm=info,\
             solana_program_runtime::stable_log=debug,\
             solana_runtime::message_processor=debug,\
             solana_runtime::system_instruction_processor=info,\
             solana_program_test=info,\
             solana_bpf_loader_program=debug";

pub fn find_bank_pda(marginfi_group_pk: &Pubkey, asset_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            LENDING_POOL_BANK_SEED,
            &marginfi_group_pk.to_bytes(),
            &asset_mint.to_bytes(),
        ],
        &marginfi::id(),
    )
}

pub fn find_bank_vault_pda(
    marginfi_group_pk: &Pubkey,
    asset_mint: &Pubkey,
    vault_type: BankVaultType,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        bank_seed!(vault_type, asset_mint, marginfi_group_pk),
        &marginfi::id(),
    )
}

pub fn find_bank_vault_authority_pda(
    marginfi_group_pk: &Pubkey,
    asset_mint: &Pubkey,
    vault_type: BankVaultType,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        bank_authority_seed!(vault_type, asset_mint, marginfi_group_pk),
        &marginfi::id(),
    )
}

pub async fn load_and_deserialize<T: AccountDeserialize>(
    ctx: Rc<RefCell<ProgramTestContext>>,
    address: &Pubkey,
) -> T {
    let ai = ctx
        .borrow_mut()
        .banks_client
        .get_account(*address)
        .await
        .unwrap()
        .unwrap();

    T::try_deserialize(&mut ai.data.as_slice()).unwrap()
}

pub fn make_ix<T>(accounts: T, ix_data: Vec<u8>) -> Instruction
where
    T: ToAccountMetas,
{
    Instruction {
        program_id: marginfi::id(),
        accounts: accounts.to_account_metas(Some(true)),
        data: ix_data,
    }
}

pub fn craft_pyth_price_account(mint: Pubkey, ui_price: i64, mint_decimals: i32) -> Account {
    let native_price = ui_price * 10_i64.pow(mint_decimals as u32);
    Account {
        lamports: 1_000_000,
        data: bytemuck::bytes_of(&PriceAccount {
            prod: mint,
            agg: PriceInfo {
                conf: 0,
                price: native_price,
                status: PriceStatus::Trading,
                ..Default::default()
            },
            expo: mint_decimals,
            prev_price: native_price,
            magic: MAGIC,
            ver: VERSION_2,
            atype: AccountType::Price as u32,
            ..Default::default()
        })
        .to_vec(),
        owner: PYTH_ID,
        executable: false,
        rent_epoch: 361,
    }
}

#[macro_export]
macro_rules! assert_custom_error {
    ($error:expr, $matcher:expr) => {
        match $error {
            solana_program_test::BanksClientError::TransactionError(
                solana_sdk::transaction::TransactionError::InstructionError(
                    _,
                    solana_program::instruction::InstructionError::Custom(n),
                ),
            ) => {
                assert_eq!(n, anchor_lang::error::ERROR_CODE_OFFSET + $matcher as u32)
            }
            _ => assert!(false),
        }
    };
}

#[macro_export]
macro_rules! assert_anchor_error {
    ($error:expr, $matcher:expr) => {
        match $error {
            solana_program_test::BanksClientError::TransactionError(
                solana_sdk::transaction::TransactionError::InstructionError(
                    _,
                    solana_program::instruction::InstructionError::Custom(n),
                ),
            ) => {
                assert_eq!(n, $matcher as u32)
            }
            _ => assert!(false),
        }
    };
}

#[macro_export]
macro_rules! assert_program_error {
    ($error:expr, $matcher:expr) => {
        match $error {
            solana_sdk::transport::TransportError::TransactionError(
                solana_sdk::transaction::InstructionError(_, x),
            ) => {
                assert_eq!(x, $matcher)
            }
            _ => assert!(false),
        };
    };
}

#[macro_export]
macro_rules! assert_eq_noise {
    ($a:expr, $b:expr, $tolerance:expr) => {
        let diff = ($a - $b).abs();
        assert!(
            diff <= $tolerance,
            "Difference between {} and {} larger than {} tolerated",
            $a,
            $b,
            $tolerance
        )
    };

    ($a:expr, $b:expr) => {
        let tolerance = fixed_macro::types::I80F48!(0.00001);
        let diff = ($a - $b).abs();
        assert!(
            diff < tolerance,
            "Difference between {} and {} larger than {} tolerated",
            $a,
            $b,
            tolerance
        )
    };
}

#[macro_export]
macro_rules! native {
    ($val: expr, "USDC") => {
        $val * 10_u64.pow(6)
    };

    ($val: expr, "USDC", f64) => {
        (($val) * 10_u64.pow(6) as f64) as u64
    };

    ($val: expr, "SOL") => {
        $val * 10_u64.pow(9)
    };

    ($val: expr, "SOL", f64) => {
        (($val) * 10_u64.pow(9) as f64) as u64
    };

    ($val: expr, "SOL_EQ") => {
        $val * 10_u64.pow(9)
    };

    ($val: expr, "SOL_EQ", f64) => {
        (($val) * 10_u64.pow(9) as f64) as u64
    };

    ($val: expr, "MNDE") => {
        $val * 10_u64.pow(9)
    };

    ($val: expr, "MNDE", f64) => {
        (($val) * 10_u64.pow(9) as f64) as u64
    };
}

#[macro_export]
macro_rules! f_native {
    ($val: expr) => {
        I80F48::from_num($val * 10_u64.pow(6))
    };
}

pub fn clone_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_bytes(&keypair.to_bytes()).unwrap()
}
