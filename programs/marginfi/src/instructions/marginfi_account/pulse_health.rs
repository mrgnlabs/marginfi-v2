use anchor_lang::prelude::*;
use bytemuck::Zeroable;
use anchor_lang::solana_program::{clock::Clock, sysvar::Sysvar};

use crate::{
    state::{
        health_cache::HealthCache,
        marginfi_account::{MarginfiAccount, RiskEngine},
    },
    MarginfiResult,
};

pub fn lending_account_pulse_health<'info>(
    ctx: Context<'_, '_, 'info, 'info, PulseHealth<'info>>,
) -> MarginfiResult {
    let clock = Clock::get()?;
    let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;

    let mut health_cache = HealthCache::zeroed();
    health_cache.timestamp = clock.unix_timestamp;

    match RiskEngine::check_account_init_health(
        &marginfi_account,
        ctx.remaining_accounts,
        &mut Some(&mut health_cache),
    ) {
        Ok(()) => {
            health_cache.set_engine_ok(true);
        }
        Err(_) => {
            health_cache.set_engine_ok(false);
        }
    }

    marginfi_account.health_cache = health_cache;

    Ok(())
}

#[derive(Accounts)]
pub struct PulseHealth<'info> {
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
}
