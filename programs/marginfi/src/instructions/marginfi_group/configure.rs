use crate::check;
use crate::events::{GroupEventHeader, MarginfiGroupConfigureEvent};
use crate::prelude::MarginfiError;
use crate::state::marginfi_account::{MarginfiAccount, FLASHLOAN_ENABLED_FLAG};
use crate::{
    state::marginfi_group::{GroupConfig, MarginfiGroup},
    MarginfiResult,
};
use anchor_lang::prelude::*;

/// Configure margin group
///
/// Admin only
pub fn configure(ctx: Context<MarginfiGroupConfigure>, config: GroupConfig) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_mut()?;

    marginfi_group.configure(&config)?;

    emit!(MarginfiGroupConfigureEvent {
        header: GroupEventHeader {
            marginfi_group: ctx.accounts.marginfi_group.key(),
            signer: Some(*ctx.accounts.admin.key)
        },
        config,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiGroupConfigure<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,
}

/// Only these flags can be configured
///
/// Example:
/// CONFIGURABLE_FLAGS = 0b0110
///
/// 0b0010 is a valid flag
/// 0b0110 is a valid flag
/// 0b1000 is not a valid flag
/// 0b0101 is not a valid flag
const CONFIGURABLE_FLAGS: u64 = FLASHLOAN_ENABLED_FLAG;

fn flag_can_be_set(flag: u64) -> bool {
    // If bitwise AND operation between flag and its bitwise NOT of CONFIGURABLE_FLAGS is 0,
    // it means no bit in flag is set outside the configurable bits.
    (flag & !CONFIGURABLE_FLAGS) == 0
}

pub fn set_account_flag(ctx: Context<SetAccountFlag>, flag: u64) -> MarginfiResult {
    check!(flag_can_be_set(flag), MarginfiError::IllegalFlag);

    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;

    marginfi_account.set_flag(flag);

    Ok(())
}

#[derive(Accounts)]
pub struct SetAccountFlag<'info> {
    #[account(address = marginfi_account.load()?.group)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    /// Admin only
    #[account(address = marginfi_group.load()?.admin)]
    pub admin: Signer<'info>,
}

pub fn unset_account_flag(ctx: Context<UnsetAccountFlag>, flag: u64) -> MarginfiResult {
    check!(flag_can_be_set(flag), MarginfiError::IllegalFlag);

    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;

    marginfi_account.unset_flag(flag);

    Ok(())
}

#[derive(Accounts)]
pub struct UnsetAccountFlag<'info> {
    #[account(address = marginfi_account.load()?.group)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    /// Admin only
    #[account(address = marginfi_group.load()?.admin)]
    pub admin: Signer<'info>,
}

#[cfg(test)]
mod tests {
    use crate::state::marginfi_account::{
        DISABLED_FLAG, FLASHLOAN_ENABLED_FLAG, IN_FLASHLOAN_FLAG,
    };

    #[test]
    ///
    /// 0b0001 is a valid flag
    /// 0b0011 is a invalid flag
    /// 0b0101 is a invalid flag
    /// 0b1000 is a invalid flag
    fn test_check_flag() {
        let flag1 = FLASHLOAN_ENABLED_FLAG;
        let flag2 = FLASHLOAN_ENABLED_FLAG + IN_FLASHLOAN_FLAG;
        let flag3 = IN_FLASHLOAN_FLAG + DISABLED_FLAG + FLASHLOAN_ENABLED_FLAG;
        let flag4 = DISABLED_FLAG + IN_FLASHLOAN_FLAG;

        assert!(super::flag_can_be_set(flag1));
        assert!(!super::flag_can_be_set(flag2));
        assert!(!super::flag_can_be_set(flag3));
        assert!(!super::flag_can_be_set(flag4));
    }
}
