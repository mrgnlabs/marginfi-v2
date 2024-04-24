use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{
    check,
    events::{GroupEventHeader, NativeOraclePythnetUpdateEvent},
    state::{marginfi_group::Bank, native_oracle::NativeOracle, price::OracleSetup},
    MarginfiError, MarginfiGroup, MarginfiResult,
};

pub fn bank_update_native_oracle_pythnet(
    ctx: Context<BankUpdateNativeOraclePythnet>,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    check!(
        matches!(bank.config.oracle_setup, OracleSetup::NativePythnet),
        MarginfiError::InvalidOracleSetup,
        "Oracle setup is not NativePythnet"
    );

    let pythnet_verificaiton_ctl = bank.get_pythnet_verificaiton_ctl()?;

    match bank.native_oracle {
        NativeOracle::Pythnet(ref mut native_oracle) => native_oracle.try_update(
            ctx.accounts.price_update_v2.as_ref(),
            pythnet_verificaiton_ctl,
        )?,
        _ => unreachable!(),
    };

    emit!(NativeOraclePythnetUpdateEvent {
        header: GroupEventHeader {
            signer: None,
            marginfi_group: ctx.accounts.marginfi_group.key(),
        },
        bank: ctx.accounts.bank.key(),
        feed_id: ctx.accounts.price_update_v2.price_message.feed_id,
        publish_timestamp: ctx.accounts.price_update_v2.price_message.publish_time,
        timestamp: Clock::get()?.unix_timestamp,
        price: ctx.accounts.price_update_v2.price_message.price,
        conf: ctx.accounts.price_update_v2.price_message.conf,
        ema_price: ctx.accounts.price_update_v2.price_message.ema_price,
        ema_conf: ctx.accounts.price_update_v2.price_message.ema_conf,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct BankUpdateNativeOraclePythnet<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    pub price_update_v2: Account<'info, PriceUpdateV2>,
}
