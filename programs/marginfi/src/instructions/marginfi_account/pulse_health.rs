use anchor_lang::prelude::*;
use bytemuck::Zeroable;
use solana_program::{clock::Clock, sysvar::Sysvar};

use crate::{
    events::HealthPulseEvent,
    state::{
        health_cache::HealthCache,
        marginfi_account::{MarginfiAccount, RiskEngine},
    },
    MarginfiError, MarginfiResult,
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
            health_cache.set_oracle_ok(true);
            health_cache.set_engine_ok(true);
        }
        Err(e) => match e {
            Error::AnchorError(a_e) => {
                let e_n = a_e.error_code_number;
                let mfi_err: MarginfiError = e_n.into();
                if mfi_err.is_risk_engine_rejection() {
                    // risk engine failure is ignored for engine_ok purposes
                    health_cache.set_engine_ok(true);
                } else {
                    health_cache.set_engine_ok(false);
                }
                // TODO bubble up the sub-error in risk engine rejection...
                if mfi_err.is_oracle_error() {
                    health_cache.set_oracle_ok(false);
                } else {
                    health_cache.set_oracle_ok(true);
                }
            }
            Error::ProgramError(_) => {
                health_cache.set_engine_ok(false);
            }
        },
    }

    marginfi_account.health_cache = health_cache;

    emit!(HealthPulseEvent {
        account: ctx.accounts.marginfi_account.key(),
        health_cache: health_cache
    });

    Ok(())
}

#[derive(Accounts)]
pub struct PulseHealth<'info> {
    #[account(mut)]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,
}
