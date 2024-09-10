use anchor_lang::prelude::*;

/// A PDA that holds the assets delegated to this program and tracks various information about its holdings.
#[account(zero_copy)]
pub struct StakeHolder {
    /// The account's own key, a pda of `vote_account`, `admin, and b"stakeholder"
    pub key: Pubkey,

    /// Currently unused
    pub admin: Pubkey,

    /// The validator's vote account where stake is delegated
    pub vote_account: Pubkey,

    /// The stake account where held stake is stored
    pub stake_account: Pubkey,

    // TODO also track program-wide (or admin-wide) delegation on another struct
    /// Net SOL controlled by this account
    /// * In SOL, in native decimals (lamports)
    pub net_delegation: u64,

    /// Reserved for future use
    pub reserved0: [u8; 512],
}

impl StakeHolder {
    pub const LEN: usize = std::mem::size_of::<StakeHolder>();
}
