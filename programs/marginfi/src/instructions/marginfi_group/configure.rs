use crate::check;
use crate::events::{GroupEventHeader, MarginfiGroupConfigureEvent};
use crate::prelude::MarginfiError;
use crate::state::marginfi_account::{MarginfiAccount, ACCOUNT_TRANSFER_AUTHORITY_ALLOWED};
use crate::{state::marginfi_group::MarginfiGroup, MarginfiResult};
use anchor_lang::prelude::*;

/// Configure margin group.
///
/// Note: not even the group admin can configure `PROGRAM_FEES_ENABLED`, only the program admin can
/// with `configure_group_fee`
///
/// Admin only
pub fn configure(
    ctx: Context<MarginfiGroupConfigure>,
    new_admin: Pubkey,
    new_emode_admin: Pubkey,
    is_arena_group: bool,
) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_mut()?;

    marginfi_group.update_admin(new_admin);
    marginfi_group.update_emode_admin(new_emode_admin);
    marginfi_group.set_arena_group(is_arena_group)?;

    msg!("flags set to: {:?}", marginfi_group.group_flags);

    emit!(MarginfiGroupConfigureEvent {
        header: GroupEventHeader {
            marginfi_group: ctx.accounts.marginfi_group.key(),
            signer: Some(*ctx.accounts.admin.key)
        },
        admin: new_admin,
        flags: marginfi_group.group_flags
    });

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiGroupConfigure<'info> {
    #[account(
        mut,
        has_one = admin
    )]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    pub admin: Signer<'info>,
}

/// Only these flags can be configured
///
/// Example:
/// CONFIGURABLE_FLAGS = 0b01100
///
/// 0b0010 is a valid flag
/// 0b0110 is a valid flag
/// 0b1000 is a valid flag
/// 0b01100 is a valid flag
/// 0b0101 is not a valid flag
const CONFIGURABLE_FLAGS: u64 = ACCOUNT_TRANSFER_AUTHORITY_ALLOWED;

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
    #[account(
        has_one = admin
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

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
    #[account(
        has_one = admin
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub admin: Signer<'info>,
}

#[cfg(test)]
mod tests {
    use crate::state::marginfi_account::{
        ACCOUNT_DISABLED, ACCOUNT_FLAG_DEPRECATED, ACCOUNT_IN_FLASHLOAN,
        ACCOUNT_TRANSFER_AUTHORITY_ALLOWED,
    };

    #[test]
    fn test_check_flag() {
        let flag1 = ACCOUNT_FLAG_DEPRECATED;
        let flag2 = ACCOUNT_FLAG_DEPRECATED + ACCOUNT_IN_FLASHLOAN;
        let flag3 = ACCOUNT_IN_FLASHLOAN + ACCOUNT_DISABLED + ACCOUNT_FLAG_DEPRECATED;
        let flag4 = ACCOUNT_DISABLED + ACCOUNT_IN_FLASHLOAN;
        let flag5 = ACCOUNT_FLAG_DEPRECATED + ACCOUNT_TRANSFER_AUTHORITY_ALLOWED;
        let flag6 = ACCOUNT_DISABLED + ACCOUNT_FLAG_DEPRECATED + ACCOUNT_TRANSFER_AUTHORITY_ALLOWED;
        let flag7 = ACCOUNT_DISABLED
            + ACCOUNT_FLAG_DEPRECATED
            + ACCOUNT_IN_FLASHLOAN
            + ACCOUNT_TRANSFER_AUTHORITY_ALLOWED;
        let flag8 = ACCOUNT_TRANSFER_AUTHORITY_ALLOWED;

        // Malformed flags should fail
        assert!(!super::flag_can_be_set(flag1));
        assert!(!super::flag_can_be_set(flag2));
        assert!(!super::flag_can_be_set(flag3));
        assert!(!super::flag_can_be_set(flag4));
        assert!(!super::flag_can_be_set(flag5));
        assert!(!super::flag_can_be_set(flag6));
        assert!(!super::flag_can_be_set(flag7));

        // Good flags should succeed
        assert!(super::flag_can_be_set(flag8));
    }
}
