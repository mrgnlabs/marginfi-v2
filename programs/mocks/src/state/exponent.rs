use crate::number::Number;
use anchor_lang::prelude::*;

#[account]
pub struct State {
    pub admin: Pubkey,
    pub exchange_rate: Number,
    pub mint_sy: Pubkey,
    pub pool_sy: Pubkey,
    pub bump: u8,
    pub mint_bump: u8,
    pub pool_bump: u8,
    pub pad0: [u8; 5],
}

#[account]
pub struct Personal {
    pub owner: Pubkey,
    pub sy_balance: u64,
}


#[derive(AnchorDeserialize, AnchorSerialize, Clone, Default)]
pub struct SyState {
    pub exchange_rate: Number,
    pub emission_indexes: Vec<Number>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct PositionState {
    pub owner: Pubkey,
    pub sy_balance: u64,
    pub emissions: Vec<Emission>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct Emission {
    pub mint: Pubkey,
    pub amount_claimable: u64,
    pub last_seen_emission_index: Number,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct MintSyReturnData {
    pub sy_out_amount: u64,
    pub exchange_rate: Number,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RedeemSyReturnData {
    pub base_out_amount: u64,
    pub exchange_rate: Number,
}
