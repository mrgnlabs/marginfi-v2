// TODO deprecate in 1.7
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use marginfi_type_crate::types::{
    make_points, p1000_to_u32, p100_to_u32, Bank, RatePoint, INTEREST_CURVE_SEVEN_POINT,
};

#[derive(Accounts)]
pub struct MigrateCurve<'info> {
    #[account(mut)]
    pub bank: AccountLoader<'info, Bank>,
}

pub fn migrate_curve(ctx: Context<MigrateCurve>) -> Result<()> {
    let mut bank = ctx.accounts.bank.load_mut()?;
    let irc = &mut bank.config.interest_rate_config;

    if irc.curve_type == INTEREST_CURVE_SEVEN_POINT {
        msg!("Already migrated, doing nothing");
        return Ok(());
    }

    irc.zero_util_rate = 0;

    let hundred_rate: I80F48 = irc.max_interest_rate.into();
    irc.hundred_util_rate = p1000_to_u32(hundred_rate);

    let util: I80F48 = irc.optimal_utilization_rate.into();
    let rate: I80F48 = irc.plateau_interest_rate.into();
    let point = RatePoint {
        util: p100_to_u32(util),
        rate: p1000_to_u32(rate),
    };
    irc.points = make_points(&[point]);
    irc.curve_type = INTEREST_CURVE_SEVEN_POINT;

    // Zero out the old fields so they can be recycled
    irc.plateau_interest_rate = I80F48::ZERO.into();
    irc.max_interest_rate = I80F48::ZERO.into();
    irc.optimal_utilization_rate = I80F48::ZERO.into();

    let interest = bank.config.interest_rate_config;
    msg!(
        "Migration complete. Rate at 0: {:?} points: {:?} rate at 100: {:?}",
        interest.zero_util_rate,
        interest.points,
        interest.hundred_util_rate
    );

    Ok(())
}
