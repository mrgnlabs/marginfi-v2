use anchor_lang::prelude::*;

declare_id!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");

#[program]
pub mod brick {
    use super::*;

    pub fn noop(ctx: Context<Initialize>) -> Result<()> {
        msg!("This program is temporarily disabled.");

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
