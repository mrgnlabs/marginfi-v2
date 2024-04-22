use anchor_lang::{
    accounts::{account::Account, account_loader::AccountLoader, signer::Signer},
    context::Context,
    emit, Accounts, Key,
};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{
    check,
    state::{marginfi_group::Bank, native_oracle::NativeOracle, price::OracleSetup},
    MarginfiError, MarginfiGroup, MarginfiResult,
};

pub fn lending_pool_configure_bank(ctx: Context<LendingPoolConfigureBank>) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    check!(
        matches!(bank.config.oracle_setup, OracleSetup::NativePythnet),
        MarginfiError::InvalidOracleSetup,
        "Oracle setup is not NativePythnet"
    );

    match bank.native_oracle {
        NativeOracle::PythCrosschain(ref mut native_oracle) => native_oracle.try_update(
            ctx.accounts.price_update_v2.as_ref(),
            bank.get_pyth_crosschain_verificaiton_ctl()?,
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
pub struct LendingPoolConfigureBank<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    pub price_update_v2: Account<'info, PriceUpdateV2>,
}
