use crate::{set_if_some, MarginfiResult};
use anchor_lang::prelude::*;

#[account(zero_copy)]
#[repr(packed)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, PartialEq, Eq)
)]
pub struct MarginfiGroup {
    pub admin: Pubkey,
}

impl Default for MarginfiGroup {
    fn default() -> Self {
        Self {
            admin: Pubkey::default(),
        }
    }
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
        *self = MarginfiGroup { admin: admin_pk };
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct GroupConfig {
    pub admin: Option<Pubkey>,
}
