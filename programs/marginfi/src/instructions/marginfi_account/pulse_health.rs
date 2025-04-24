use anchor_lang::prelude::*;
use bytemuck::Zeroable;
use fixed::types::I80F48;
use solana_program::{clock::Clock, sysvar::Sysvar};

use crate::{
    constants::PROGRAM_VERSION,
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
    health_cache.program_version = PROGRAM_VERSION;

    let (engine_result, engine) = RiskEngine::check_account_init_health(
        &marginfi_account,
        ctx.remaining_accounts,
        &mut Some(&mut health_cache),
    );
    match engine_result {
        Ok(()) => {
            if health_cache.internal_err != 0 {
                health_cache.set_oracle_ok(false);
            } else {
                health_cache.set_oracle_ok(true);
            }
            health_cache.set_engine_ok(true);
        }
        Err(e) => match e {
            Error::AnchorError(a_e) => {
                let e_n = a_e.error_code_number;
                health_cache.mrgn_err = e_n;
                let mfi_err: MarginfiError = e_n.into();
                if mfi_err.is_risk_engine_rejection() {
                    // risk engine failure is ignored for engine_ok purposes
                    health_cache.set_engine_ok(true);
                } else {
                    health_cache.set_engine_ok(false);
                }
                if mfi_err.is_oracle_error() || health_cache.internal_err != 0 {
                    health_cache.set_oracle_ok(false);
                } else {
                    health_cache.set_oracle_ok(true);
                }
            }
            Error::ProgramError(_) => {
                health_cache.set_engine_ok(false);
            }
        },
    };

    // If the engine wasn't returned that means it failed to load for some fundamental reason and
    // the values will likely be garbage regardless.
    if engine.is_some() {
        let engine = engine.unwrap();
        // Note: if the risk engine didn't error for init, it's unlikely it will error here
        let liq_result: MarginfiResult<I80F48> = engine
            .check_pre_liquidation_condition_and_get_account_health(
                None,
                &mut Some(&mut health_cache),
            );
        if liq_result.is_err() {
            msg!("liquidation health check failed: this should never happen");
        }
        let bankruptcy_result: MarginfiResult =
            engine.check_account_bankrupt(&mut Some(&mut health_cache));
        if bankruptcy_result.is_err() {
            msg!("bankruptcy health check failed: this should never happen");
        }
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
