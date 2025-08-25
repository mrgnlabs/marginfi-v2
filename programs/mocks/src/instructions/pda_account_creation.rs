use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::InstructionData;

#[derive(Accounts)]
#[instruction(account_index: u32, third_party_id: Option<u32>)]
pub struct CreateMarginfiAccountPdaViaCpi<'info> {
    /// CHECK: This is the marginfi group account being passed to CPI
    #[account(mut)]
    pub marginfi_group: UncheckedAccount<'info>,

    /// CHECK: This will be the actual marginfi account PDA that gets created
    #[account(mut)]
    pub marginfi_account: UncheckedAccount<'info>,

    /// CHECK: This is the authority for the marginfi account
    pub authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Instructions sysvar account
    pub instructions_sysvar: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: The marginfi program that we're calling via CPI
    pub marginfi_program: UncheckedAccount<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + std::mem::size_of::<CpiCallLog>(),
    )]
    pub call_log: Account<'info, CpiCallLog>,
}

#[account]
pub struct CpiCallLog {
    pub caller_program: Pubkey,
    pub target_program: Pubkey,
    pub marginfi_group: Pubkey,
    pub marginfi_account: Pubkey,
    pub authority: Pubkey,
    pub account_index: u32,
    pub third_party_id: Option<u32>,
    pub call_successful: bool,
    pub timestamp: i64,
}

impl CreateMarginfiAccountPdaViaCpi<'_> {
    pub fn create_marginfi_account_pda_via_cpi(
        ctx: Context<CreateMarginfiAccountPdaViaCpi>,
        account_index: u32,
        third_party_id: Option<u32>,
    ) -> Result<()> {
        let clock = Clock::get()?;

        let call_log = &mut ctx.accounts.call_log;
        call_log.caller_program = crate::ID;
        call_log.target_program = ctx.accounts.marginfi_program.key();
        call_log.marginfi_group = ctx.accounts.marginfi_group.key();
        call_log.marginfi_account = ctx.accounts.marginfi_account.key();
        call_log.authority = ctx.accounts.authority.key();
        call_log.account_index = account_index;
        call_log.third_party_id = third_party_id;
        call_log.timestamp = clock.unix_timestamp;

        let _accounts = marginfi::accounts::MarginfiAccountInitializePda {
            marginfi_group: ctx.accounts.marginfi_group.key(),
            marginfi_account: ctx.accounts.marginfi_account.key(),
            authority: ctx.accounts.authority.key(),
            fee_payer: ctx.accounts.fee_payer.key(),
            instructions_sysvar: ctx.accounts.instructions_sysvar.key(),
            system_program: ctx.accounts.system_program.key(),
        };

        let instruction_data = marginfi::instruction::MarginfiAccountInitializePda {
            account_index,
            third_party_id,
        };

        let account_metas = vec![
            AccountMeta::new_readonly(ctx.accounts.marginfi_group.key(), false),
            AccountMeta::new(ctx.accounts.marginfi_account.key(), false),
            AccountMeta::new_readonly(ctx.accounts.authority.key(), false),
            AccountMeta::new(ctx.accounts.fee_payer.key(), true),
            AccountMeta::new_readonly(ctx.accounts.instructions_sysvar.key(), false),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        ];
        let instruction = Instruction {
            program_id: ctx.accounts.marginfi_program.key(),
            accounts: account_metas,
            data: instruction_data.data(),
        };

        anchor_lang::solana_program::program::invoke(
            &instruction,
            &[
                ctx.accounts.marginfi_group.to_account_info(),
                ctx.accounts.marginfi_account.to_account_info(),
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.fee_payer.to_account_info(),
                ctx.accounts.instructions_sysvar.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        call_log.call_successful = true;

        Ok(())
    }
}
