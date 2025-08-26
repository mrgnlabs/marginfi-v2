use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as ix_sysvar;
use anchor_lang::solana_program::sysvar::instructions::load_instruction_at_checked;
use pyth_solana_receiver_sdk::price_update::VerificationLevel;

use crate::MarginfiResult;

// This file should only contain the constants which couldn't be moved to type-crate:
// 1. the constants used for testing/internal purposes
// 2. or the ones dependant on some 3rd party crates which are not part of type-crate dependency tree

/// Mocks program ID for third-party ID restrictions
pub const MOCKS_PROGRAM_ID: Pubkey = pubkey!("5XaaR94jBubdbrRrNW7DtRvZeWvLhSHkEGU3jHTEXV3C");

/// Used for the health cache to track which version of the program generated it.
/// * 0 = invalid
/// * 1 = 0.1.3
/// * 2 = 0.1.4
/// * 3 = 0.1.5
/// * others = invalid
pub const PROGRAM_VERSION: u8 = 3;

cfg_if::cfg_if! {
    if #[cfg(feature = "devnet")] {
        pub const PYTH_ID: Pubkey = pubkey!("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s");
    } else if #[cfg(any(feature = "mainnet-beta", feature = "staging"))] {
        pub const PYTH_ID: Pubkey = pubkey!("FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH");
    } else {
        // The key of the mock program on localnet (see its declared id)
        pub const PYTH_ID: Pubkey = pubkey!("5XaaR94jBubdbrRrNW7DtRvZeWvLhSHkEGU3jHTEXV3C");
    }
}

// TODO update to the actual deployment key on mainnet/devnet/staging
cfg_if::cfg_if! {
    if #[cfg(feature = "devnet")] {
        pub const SPL_SINGLE_POOL_ID: Pubkey = pubkey!("SVSPxpvHdN29nkVg9rPapPNDddN5DipNLRUFhyjFThE");
    } else if #[cfg(any(feature = "mainnet-beta", feature = "staging"))] {
        pub const SPL_SINGLE_POOL_ID: Pubkey = pubkey!("SVSPxpvHdN29nkVg9rPapPNDddN5DipNLRUFhyjFThE");
    } else {
        pub const SPL_SINGLE_POOL_ID: Pubkey = pubkey!("SVSPxpvHdN29nkVg9rPapPNDddN5DipNLRUFhyjFThE");
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "devnet")] {
        pub const SWITCHBOARD_PULL_ID: Pubkey = pubkey!("Aio4gaXjXzJNVLtzwtNVmSqGKpANtXhybbkhtAC94ji2");
    } else {
        pub const SWITCHBOARD_PULL_ID: Pubkey = pubkey!("SBondMDrcV3K4kxZR1HNVT7osZxAHVHgYXL5Ze1oMUv");
    }
}

pub const NATIVE_STAKE_ID: Pubkey = pubkey!("Stake11111111111111111111111111111111111111");

/// The default fee, in native SOL in native decimals (i.e. lamports) used in testing
pub const INIT_BANK_ORIGINATION_FEE_DEFAULT: u32 = 10000;

pub const MIN_PYTH_PUSH_VERIFICATION_LEVEL: VerificationLevel = VerificationLevel::Full;

// TODO move this to the global fee wallet eventually
/// A nominal fee paid to the global wallet when intiating an account transfer. Primarily intended
/// to avoid spamming account migration, which is mildly annoying to backend systems that track the
/// state of accounts.
/// * Should be ~ $0.50 or around that magnitude
/// * In lamports
pub const ACCOUNT_TRANSFER_FEE: u64 = 5_000_000;

/// When creating a mrgn account using a PDA, programs that wish to specify a third_party_id must be
/// registered here. This confers no other benefits. Creating accounts with third_party_id = 0 (the
/// default) is freely available to any caller.
///
/// This enables third-parties (who have registered) to quickly sort all mrgn accounts that are
/// relevant to their use-case by memcmp without loading the entire mrgn ecosystem.
///
/// Registration is free, we will include your registration in the next program update. Feel free to
/// request multiple.
///
/// Contact us to register at // TODO need a good email....
pub const THIRD_PARTY_CPI_RULES: &[(u32, Pubkey)] = &[
    (10_001, MOCKS_PROGRAM_ID),
    // (7, SOME_OTHER_PROGRAM_ID),
    // (99, YET_ANOTHER_PROGRAM_ID),
];

/// * IDs < FREE_THRESHOLD are "free" (no special CPI restriction), just go ahead and use them
/// 
/// * IDs >= FREE_THRESHOLD are "restricted": must contact us to register first.
pub const PDA_FREE_THRESHOLD: u32 = 10_000;

// TODO move to ix_utils after liquidation_remix merged into 0.1.5
/// Numbers > PDA_FREE_THRESHOLD are restricted, contact us to secure one.
///
///
/// Returns:
/// - Ok(true)  => it *is* a CPI from the allowed program for `third_party_id`
/// - Ok(false) => not a CPI (direct call) OR CPI from a different program
pub fn is_allowed_cpi_for_third_party_id(
    sysvar_info: &AccountInfo,
    third_party_id: u32,
) -> MarginfiResult<bool> {
    // Free tier: no gating at all.
    if third_party_id < PDA_FREE_THRESHOLD {
        return Ok(true);
    }

    // Restricted tier: must have a rule.
    let allowed_program = match THIRD_PARTY_CPI_RULES
        .iter()
        .find(|(id, _)| *id == third_party_id)
        .map(|(_, program_id)| *program_id)
    {
        Some(p) => p,
        None => {
            return Ok(false);
        }
    };

    let current_ix_index = ix_sysvar::load_current_index_checked(sysvar_info)?;
    let current_ixn = load_instruction_at_checked(current_ix_index as usize, sysvar_info)?;

    // If the current (top-level) instruction is *this* program, it's a direct call (not CPI) -> no "third party" id allowed
    if current_ixn.program_id == crate::ID {
        return Ok(false);
    }

    Ok(current_ixn.program_id == allowed_program)
}
