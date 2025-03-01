use anchor_lang::prelude::*;
use bytemuck::Zeroable;
use solana_program::{clock::Clock, sysvar::Sysvar};

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

    // Check account health, and update the cache accordingly.
    match RiskEngine::check_account_init_health(&marginfi_account, ctx.remaining_accounts) {
        Ok((assets, liabilities, prices)) => {
            health_cache.asset_value = assets.into();
            health_cache.liability_value = liabilities.into();

            let assets_raw: i128 = assets.to_num();
            let liabs_raw: u128 = liabilities.to_num();
            msg!("assets: {:?} liabilities: {:?}", assets_raw, liabs_raw);

            let healthy = assets >= liabilities;
            health_cache.set_healthy(healthy);
            health_cache.set_engine_ok(true);

            for (i, slot) in health_cache.prices.iter_mut().enumerate() {
                if let Some(price) = prices.get(i) {
                    *slot = price.wrapping_to_num();
                } else {
                    *slot = 0;
                }
            }
        }
        Err(_) => {
            // TODO still log on err? We could pack the data into the Err type but it may be incomplete.

            // If check_account_init_health errors, mark the engine as not OK.
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
