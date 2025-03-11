use crate::constants::FEE_STATE_SEED;
use crate::events::{GroupEventHeader, MarginfiGroupCreateEvent};
use crate::state::fee_state::FeeState;
use crate::{state::marginfi_group::MarginfiGroup, MarginfiResult};
use anchor_lang::prelude::*;

pub fn initialize_group(
    ctx: Context<MarginfiGroupInitialize>,
    is_arena_group: bool,
) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_init()?;

    marginfi_group.set_initial_configuration(ctx.accounts.admin.key());
    marginfi_group.set_arena_group(is_arena_group)?;

    msg!(
        "Group admin: {:?} flags: {:?}",
        marginfi_group.admin,
        marginfi_group.group_flags
    );

    let fee_state = ctx.accounts.fee_state.load()?;

    marginfi_group.fee_state_cache.global_fee_wallet = fee_state.global_fee_wallet;
    marginfi_group.fee_state_cache.program_fee_fixed = fee_state.program_fee_fixed;
    marginfi_group.fee_state_cache.program_fee_rate = fee_state.program_fee_rate;
    marginfi_group.banks = 0;

    let cache = marginfi_group.fee_state_cache;
    msg!(
        "global fee wallet: {:?}, fixed fee: {:?}, program free {:?}",
        cache.global_fee_wallet,
        cache.program_fee_fixed,
        cache.program_fee_rate
    );

    emit!(MarginfiGroupCreateEvent {
        header: GroupEventHeader {
            marginfi_group: ctx.accounts.marginfi_group.key(),
            signer: Some(*ctx.accounts.admin.key)
        },
    });

    Ok(())
}

#[derive(Accounts)]
pub struct MarginfiGroupInitialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + std::mem::size_of::<MarginfiGroup>(),
    )]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [FEE_STATE_SEED.as_bytes()],
        bump,
    )]
    pub fee_state: AccountLoader<'info, FeeState>,

    pub system_program: Program<'info, System>,
}
