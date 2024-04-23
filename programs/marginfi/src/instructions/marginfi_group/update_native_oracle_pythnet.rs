use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{
    check,
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
        ),
        _ => unreachable!(),
    };

    // emit!(LendingPoolBankConfigureEvent {
    //     header: GroupEventHeader {
    //         marginfi_group: ctx.accounts.marginfi_group.key(),
    //         signer: Some(*ctx.accounts.admin.key)
    //     },
    //     bank: ctx.accounts.bank.key(),
    //     mint: bank.mint,
    //     config: bank_config,
    // });

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
