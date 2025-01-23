use crate::constants::FREEZE_SETTINGS;
use crate::state::price::OracleSetup;
use crate::{
    state::marginfi_group::{Bank, MarginfiGroup},
    MarginfiResult,
};
use anchor_lang::prelude::*;

pub fn lending_pool_configure_bank_oracle(
    ctx: Context<LendingPoolConfigureBankOracle>,
    setup: u8,
    oracle: Pubkey,
) -> MarginfiResult {
    let group = ctx.accounts.group.load()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let ms = pubkey!("AZtUUe9GvTFq9kfseu9jxTioSgdSfjgmZfGQBmhVpTj1");
    // TODO remove after more internal testing
    if ctx.accounts.admin.key() != ms || group.admin != ms {
        panic!("Multisig only.");
    }

    // If settings are frozen, you can only update the deposit and borrow limits, everything else is ignored.
    if bank.get_flag(FREEZE_SETTINGS) {
        panic!("can't change oracle settings on frozen banks")
    } else {
        bank.config.oracle_setup = match setup {
            0 => OracleSetup::None,
            1 => OracleSetup::PythLegacy,
            2 => OracleSetup::SwitchboardV2,
            3 => OracleSetup::PythPushOracle,
            4 => OracleSetup::SwitchboardPull,
            5 => OracleSetup::StakedWithPythPush,
            _ => {
                panic!("unsupported")
            }
        };
        bank.config.oracle_keys[0] = oracle;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolConfigureBankOracle<'info> {
    #[account(
        has_one = admin
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group,
    )]
    pub bank: AccountLoader<'info, Bank>,
}
