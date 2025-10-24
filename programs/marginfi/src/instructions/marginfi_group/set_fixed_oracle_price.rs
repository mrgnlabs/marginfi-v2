use crate::events::{GroupEventHeader, LendingPoolBankSetFixedOraclePriceEvent};
use crate::state::bank::BankImpl;
use crate::{check, errors::MarginfiError, MarginfiResult};
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use marginfi_type_crate::{
    constants::FREEZE_SETTINGS,
    types::{Bank, MarginfiGroup, OracleSetup, WrappedI80F48},
};

pub fn lending_pool_set_fixed_oracle_price(
    ctx: Context<LendingPoolSetFixedOraclePrice>,
    price: WrappedI80F48,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    if bank.get_flag(FREEZE_SETTINGS) {
        panic!("cannot change oracle settings on frozen banks");
    }

    check!(
        bank.config.oracle_setup == OracleSetup::Fixed,
        MarginfiError::InvalidOracleSetup
    );

    let price_i80: I80F48 = price.into();
    check!(
        price_i80 > I80F48::ZERO,
        MarginfiError::FixedOraclePriceNotSet
    );

    bank.fixed_price = price;

    emit!(LendingPoolBankSetFixedOraclePriceEvent {
        header: GroupEventHeader {
            marginfi_group: ctx.accounts.group.key(),
            signer: Some(*ctx.accounts.admin.key),
        },
        bank: ctx.accounts.bank.key(),
        price,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolSetFixedOraclePrice<'info> {
    #[account(has_one = admin)]
    pub group: AccountLoader<'info, MarginfiGroup>,

    pub admin: Signer<'info>,

    #[account(mut, has_one = group)]
    pub bank: AccountLoader<'info, Bank>,
}
