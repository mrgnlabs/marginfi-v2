use anchor_lang::prelude::Pubkey;
use solana_program::pubkey;

pub const LIQUIDITY_VAULT_AUTHORITY_SEED: &[u8; 20] = b"liquidity_vault_auth";
pub const INSURANCE_VAULT_AUTHORITY_SEED: &[u8; 20] = b"insurance_vault_auth";
pub const FEE_VAULT_AUTHORITY_SEED: &[u8; 14] = b"fee_vault_auth";

pub const LIQUIDITY_VAULT_SEED: &[u8; 15] = b"liquidity_vault";
pub const INSURANCE_VAULT_SEED: &[u8; 15] = b"insurance_vault";
pub const FEE_VAULT_SEED: &[u8; 9] = b"fee_vault";

/// Dummy PK
pub const PYTH_ID: Pubkey = pubkey!("5rYvdyWAunZgD2EC1aKo7hQbutUUnkt7bBFM6xNq2z7Z");
