use anchor_lang::{prelude::*, Discriminator};
use marginfi::constants::PYTH_ID;
use pyth_sdk_solana::state::{
    AccountType, PriceAccount, PriceInfo, PriceStatus, Rational, MAGIC, VERSION_2,
};
use solana_program::{instruction::Instruction, pubkey};
use solana_program_test::*;
use solana_sdk::{account::Account, signature::Keypair};
use std::mem::size_of;
use std::{cell::RefCell, rc::Rc};
use switchboard_v2::SWITCHBOARD_PROGRAM_ID;
use switchboard_v2::{
    AggregatorAccountData, AggregatorResolutionMode, AggregatorRound, SwitchboardDecimal,
};

pub const MS_PER_SLOT: u64 = 400;
pub const RUST_LOG_DEFAULT: &str = "solana_rbpf::vm=info,\
             solana_program_runtime::stable_log=debug,\
             solana_runtime::message_processor=debug,\
             solana_runtime::system_instruction_processor=info,\
             solana_program_test=info,\
             solana_bpf_loader_program=debug";

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

pub fn create_pyth_price_account(
    mint: Pubkey,
    ui_price: i64,
    mint_decimals: i32,
    timestamp: Option<i64>,
) -> Account {
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
            expo: -mint_decimals,
            prev_price: native_price,
            magic: MAGIC,
            ver: VERSION_2,
            atype: AccountType::Price as u32,
            timestamp: 0,
            ema_price: Rational {
                val: native_price,
                numer: native_price,
                denom: 1,
            },
            prev_timestamp: timestamp.unwrap_or(0),
            ema_conf: Rational {
                val: 0,
                numer: 0,
                denom: 1,
            },
            ..Default::default()
        })
        .to_vec(),
        owner: PYTH_ID,
        executable: false,
        rent_epoch: 361,
    }
}

pub fn create_switchboard_price_feed(ui_price: i64, mint_decimals: i32) -> Account {
    let native_price = ui_price * 10_i64.pow(mint_decimals as u32);
    let aggregator_account = switchboard_v2::AggregatorAccountData {
        name: [0; 32],
        metadata: [0; 128],
        _reserved1: [0; 32],
        queue_pubkey: Pubkey::default(),
        oracle_request_batch_size: 4,
        min_oracle_results: 2,
        min_job_results: 1,
        min_update_delay_seconds: 6,
        start_after: 0,
        variance_threshold: SwitchboardDecimal {
            mantissa: 0,
            scale: 0,
        },
        force_report_period: 0,
        expiration: 0,
        consecutive_failure_count: 0,
        next_allowed_update_time: 1682220588,
        is_locked: false,
        crank_pubkey: Pubkey::default(),
        latest_confirmed_round: AggregatorRound {
            num_success: 4,
            num_error: 0,
            is_closed: true,
            round_open_slot: 189963416,
            round_open_timestamp: 1682220573,
            result: SwitchboardDecimal {
                mantissa: native_price as i128,
                scale: mint_decimals as u32,
            },
            std_deviation: SwitchboardDecimal {
                mantissa: 13942937500000000000000000,
                scale: 28,
            },
            min_response: SwitchboardDecimal {
                mantissa: 2175243675,
                scale: 8,
            },
            max_response: SwitchboardDecimal {
                mantissa: 21763,
                scale: 3,
            },
            oracle_pubkeys_data: [Pubkey::default(); 16],
            medians_data: [
                SwitchboardDecimal {
                    mantissa: 21757,
                    scale: 3,
                },
                SwitchboardDecimal {
                    mantissa: 21757,
                    scale: 3,
                },
                SwitchboardDecimal {
                    mantissa: 21757,
                    scale: 3,
                },
                SwitchboardDecimal {
                    mantissa: 217597885875,
                    scale: 10,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
            ],
            current_payout: [12500, 12500, 0, 12500, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            medians_fulfilled: [
                true, true, true, true, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            errors_fulfilled: [
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
        },
        current_round: AggregatorRound {
            num_success: 0,
            num_error: 0,
            is_closed: false,
            round_open_slot: 189963432,
            round_open_timestamp: 1682220581,
            result: SwitchboardDecimal {
                mantissa: 0,
                scale: 0,
            },
            std_deviation: SwitchboardDecimal {
                mantissa: 0,
                scale: 0,
            },
            min_response: SwitchboardDecimal {
                mantissa: 0,
                scale: 0,
            },
            max_response: SwitchboardDecimal {
                mantissa: 0,
                scale: 0,
            },
            oracle_pubkeys_data: [Pubkey::default(); 16],
            medians_data: [
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
                SwitchboardDecimal {
                    mantissa: 0,
                    scale: 0,
                },
            ],
            current_payout: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            medians_fulfilled: [
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            errors_fulfilled: [
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
        },
        job_pubkeys_data: [Pubkey::default(); 16],
        job_hashes: [switchboard_v2::Hash::default(); 16],
        job_pubkeys_size: 5,
        jobs_checksum: [
            119, 207, 222, 177, 160, 127, 254, 198, 132, 153, 111, 54, 202, 89, 87, 81, 75, 152,
            67, 132, 249, 111, 216, 90, 132, 22, 198, 45, 67, 233, 50, 225,
        ],
        authority: pubkey!("GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR"),
        history_buffer: pubkey!("E3cqnoFvTeKKNsGmC8YitpMjo2E39hwfoyt2Aiem7dCb"),
        previous_confirmed_round_result: SwitchboardDecimal {
            mantissa: 21757,
            scale: 3,
        },
        previous_confirmed_round_slot: 189963416,
        disable_crank: false,
        job_weights: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        creation_timestamp: 0,
        resolution_mode: AggregatorResolutionMode::ModeRoundResolution,
        _ebuf: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    let desc_bytes = <AggregatorAccountData as Discriminator>::DISCRIMINATOR;
    let mut data = vec![0u8; 8 + size_of::<AggregatorAccountData>()];
    data[..8].copy_from_slice(&desc_bytes);
    data[8..].copy_from_slice(bytemuck::bytes_of(&aggregator_account));

    Account {
        lamports: 10000,
        data,
        owner: SWITCHBOARD_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
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
macro_rules! ui_to_native {
    ($val: expr, $mint_decimals: expr) => {
        ($val * 10_u64.pow($mint_decimals as u32) as f64) as u64
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

    ($val: expr, $decimals: expr) => {
        $val * 10_u64.pow($decimals as u32)
    };

    ($val: expr, $decimals: expr, f64) => {
        (($val) * 10_u64.pow($decimals as u32) as f64) as u64
    };
}

#[macro_export]
macro_rules! time {
    ($val: expr) => {
        $val
    };

    ($val: expr, "s") => {
        $val
    };

    ($val: expr, "m") => {
        $val * 60
    };

    ($val: expr, "h") => {
        $val * 60 * 60
    };

    ($val: expr, "d") => {
        $val * 60 * 60 * 24
    };

    ($val: expr, "w") => {
        $val * 60 * 60 * 24 * 7
    };

    ($val: expr, "y") => {
        $val * 60 * 60 * 24 * 365
    };

    ($val: expr, "M") => {
        $val * 60 * 60 * 24 * 30
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

pub fn get_emissions_authority_address(bank_pk: Pubkey, emissions_mint: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            marginfi::constants::EMISSIONS_AUTH_SEED.as_bytes(),
            bank_pk.as_ref(),
            emissions_mint.as_ref(),
        ],
        &marginfi::id(),
    )
}

pub fn get_emissions_token_account_address(
    bank_pk: Pubkey,
    emissions_mint: Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            marginfi::constants::EMISSIONS_TOKEN_ACCOUNT_SEED.as_bytes(),
            bank_pk.as_ref(),
            emissions_mint.as_ref(),
        ],
        &marginfi::id(),
    )
}

#[cfg(feature = "lip")]
pub mod lip {
    use super::*;
    pub fn get_reward_vault_address(campaign_key: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                liquidity_incentive_program::constants::CAMPAIGN_SEED.as_bytes(),
                campaign_key.as_ref(),
            ],
            &liquidity_incentive_program::id(),
        )
    }

    pub fn get_reward_vault_authority(campaign_key: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                liquidity_incentive_program::constants::CAMPAIGN_AUTH_SEED.as_bytes(),
                campaign_key.as_ref(),
            ],
            &liquidity_incentive_program::id(),
        )
    }

    pub fn get_temp_token_account_authority(deposit_key: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                liquidity_incentive_program::constants::TEMP_TOKEN_ACCOUNT_AUTH_SEED.as_bytes(),
                deposit_key.as_ref(),
            ],
            &liquidity_incentive_program::id(),
        )
    }

    pub fn get_deposit_mfi_authority(deposit_key: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                liquidity_incentive_program::constants::DEPOSIT_MFI_AUTH_SIGNER_SEED.as_bytes(),
                deposit_key.as_ref(),
            ],
            &liquidity_incentive_program::id(),
        )
    }

    pub fn get_marginfi_account_address(deposit_key: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                liquidity_incentive_program::constants::MARGINFI_ACCOUNT_SEED.as_bytes(),
                deposit_key.as_ref(),
            ],
            &liquidity_incentive_program::id(),
        )
    }
}
