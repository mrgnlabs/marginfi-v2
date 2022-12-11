use crate::{set_if_some, MarginfiResult};
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;

#[account(zero_copy)]
#[repr(packed)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
#[derive(Default)]
pub struct MarginfiGroup {
    pub admin: Pubkey,
    pub lending_pool: LendingPool,
}

impl MarginfiGroup {
    /// Configure the group parameters.
    /// This function validates config values so the group remains in a valid state.
    /// Any modification of group config should happen through this function.
    pub fn configure(&mut self, config: GroupConfig) -> MarginfiResult {
        set_if_some!(self.admin, config.admin);

        Ok(())
    }

    /// Set the group parameters when initializing a group.
    /// This should be called only when the group is first initialized.
    /// Both margin requirements are initially set to 100% and should be configured before use.
    #[allow(clippy::too_many_arguments)]
    pub fn set_initial_configuration(&mut self, admin_pk: Pubkey) {
        *self = MarginfiGroup {
            admin: admin_pk,
            ..Default::default()
        };
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct GroupConfig {
    pub admin: Option<Pubkey>,
}

const MAX_LENDING_POOL_RESERVES: usize = 128;
const I80F48_ONE: I80F48 = I80F48!(1);

#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
#[zero_copy]
pub struct LendingPool {
    pub banks: [Bank; MAX_LENDING_POOL_RESERVES],
}

impl Default for LendingPool {
    fn default() -> Self {
        Self {
            banks: [Bank::default(); MAX_LENDING_POOL_RESERVES],
        }
    }
}

impl LendingPool {
    pub fn get_bank(&self, mint_pk: &Pubkey) -> Option<&Bank> {
        self.banks.iter().find(|reserve| reserve.mint.eq(mint_pk))
    }

    pub fn get_bank_mut(&mut self, mint_pk: &Pubkey) -> Option<&mut Bank> {
        self.banks
            .iter_mut()
            .find(|reserve| reserve.mint.eq(mint_pk))
    }
}

#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
#[zero_copy]
#[derive(Default)]
pub struct Bank {
    pub mint: Pubkey,

    pub deposit_share_value: I80F48,
    pub liability_share_value: I80F48,

    pub liquidity_vault: Pubkey,
    pub insurance_vault: Pubkey,
    pub fee_vault: Pubkey,

    pub config: BankConfig,

    pub total_borrows: u64,
    pub total_deposits: u64,
}

impl Bank {
    pub fn initialize(
        &mut self,
        config: BankConfig,
        mint_pk: Pubkey,
        liquidity_vault: Pubkey,
        insurance_vault: Pubkey,
        fee_vault: Pubkey,
    ) {
        *self = Bank {
            mint: mint_pk,
            deposit_share_value: I80F48_ONE,
            liability_share_value: I80F48_ONE,
            liquidity_vault,
            insurance_vault,
            fee_vault,
            config,
            total_borrows: 0,
            total_deposits: 0,
        }
    }
}

#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
#[zero_copy]
#[derive(Default)]
pub struct BankConfig {
    pub collateral_weight: I80F48,
    pub liability_weight: I80F48,
    pub reserve_max_capacity: u64,

    pub pyth_oracle: Pubkey,
    pub switchboard_oracle: Pubkey,
}
