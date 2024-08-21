use anchor_lang::prelude::*;

#[account()]
pub struct PoolAuth {
    /// The account's own key
    pub key: Pubkey,
    pub pool_a: Pubkey,
    pub pool_b: Pubkey,
    pub bump_seed: u8,
    pub nonce: u16,
}

impl PoolAuth {
    pub const LEN: usize = std::mem::size_of::<PoolAuth>();
}
