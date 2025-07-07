use crate::events::{GroupEventHeader, MarginfiGroupConfigureEvent};
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
    new_curve_admin: Pubkey,
    new_limit_admin: Pubkey,
    new_emissions_admin: Pubkey,
    is_arena_group: bool,
) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_mut()?;

    marginfi_group.update_admin(new_admin);
    marginfi_group.update_emode_admin(new_emode_admin);
    marginfi_group.update_curve_admin(new_curve_admin);
    marginfi_group.update_limit_admin(new_limit_admin);
    marginfi_group.update_emissions_admin(new_emissions_admin);
    marginfi_group.set_arena_group(is_arena_group)?;

    let clock = Clock::get()?;
    marginfi_group.fee_state_cache.last_update = clock.unix_timestamp;

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
