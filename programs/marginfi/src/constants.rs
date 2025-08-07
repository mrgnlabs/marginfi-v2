use anchor_lang::prelude::*;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use pyth_solana_receiver_sdk::price_update::VerificationLevel;

// This file should only contain the constants which couldn't be moved to type-crate:
// 1. the constants used for testing/internal purposes
// 2. or the ones dependant on some 3rd party crates which are not part of type-crate dependency tree

/// Used for the health cache to track which version of the program generated it.
/// * 0 = invalid
/// * 1 = 0.1.3
/// * 2 = 0.1.4
/// * others = invalid
pub const PROGRAM_VERSION: u8 = 2;

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

pub const COMPUTE_PROGRAM_KEY: Pubkey = pubkey!("ComputeBudget111111111111111111111111111111");

pub const NATIVE_STAKE_ID: Pubkey = pubkey!("Stake11111111111111111111111111111111111111");

/// The default fee, in native SOL in native decimals (i.e. lamports) used in testing
pub const INIT_BANK_ORIGINATION_FEE_DEFAULT: u32 = 10000;
/// The default fee, in native SOL in native decimals (i.e. lamports) used in testing
pub const LIQUIDATION_FLAT_FEE_DEFAULT: u32 = 5000;
/// Liquidators can claim at least this premium, as a percent, when liquidating an asset in
/// receivership liquidation, e.g. (1 + this) * amount repaid <= asset seized
/// * This is the minimum value the program allows for the above, if fee state is set below this,
///   the program will use this instead.
pub const LIQUIDATION_MAX_FEE_MINIMUM: I80F48 = I80F48!(0.05);
/// Liquidators can consume/close out the entire account with essentially no limits (e.g. regardless
/// of liquidation bonus, etc) if it has net assets worth less than this amount in dollars. This
/// roughly covers the fee to open a liquidation record plus a little extra.
pub const LIQUIDATION_DOLLAR_THRESHOLD: I80F48 = I80F48!(5);

pub const MIN_PYTH_PUSH_VERIFICATION_LEVEL: VerificationLevel = VerificationLevel::Full;

// TODO move this to the global fee wallet eventually
/// A nominal fee paid to the global wallet when intiating an account transfer. Primarily intended
/// to avoid spamming account migration, which is mildly annoying to backend systems that track the
/// state of accounts.
/// * Should be ~ $0.50 or around that magnitude
/// * In lamports
pub const ACCOUNT_TRANSFER_FEE: u64 = 5_000_000;
