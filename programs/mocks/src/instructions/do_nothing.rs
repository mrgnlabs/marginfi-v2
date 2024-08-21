use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct DoNothing<'info> {
    pub payer: Signer<'info>,
}

pub fn do_nothing(ctx: Context<DoNothing>) -> Result<()> {
    msg!(
        "Nothing was done. Signed by: {:?}",
        ctx.accounts.payer.key()
    );
    Ok(())
}
