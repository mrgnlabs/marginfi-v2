use anchor_lang::prelude::*;

pub type MarginfiResult<G = ()> = Result<G>;

pub use crate::{
    errors::MarginfiError,
    state::marginfi_group::{GroupConfig, MarginfiGroup},
};
