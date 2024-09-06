use anchor_lang::prelude::*;

#[account()]
pub struct StakeUser {
    /// The account's own key
    pub key: Pubkey,
}

impl StakeUser {
    pub const LEN: usize = std::mem::size_of::<StakeUser>();
}
