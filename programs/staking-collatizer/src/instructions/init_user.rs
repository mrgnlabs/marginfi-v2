use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitUser<'info> {
    pub payer: Signer<'info>,
}

pub fn init_user(ctx: Context<InitUser>) -> Result<()> {
    msg!(
        "Nothing was done. Signed by: {:?}",
        ctx.accounts.payer.key()
    );
    Ok(())
}
