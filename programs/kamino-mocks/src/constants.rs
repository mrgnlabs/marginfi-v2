// Re-export constants from type-crate to avoid duplication and stack overflow
pub use marginfi_type_crate::constants::{EXP_10_I80F48, MAX_EXP_10_I80F48};

/// Even though the collateral mint uses this number of decimals internally, you should never need
/// this value because collateral is always denominated in mint_decimals.
pub const COLLATERAL_DECIMALS: usize = 6;
