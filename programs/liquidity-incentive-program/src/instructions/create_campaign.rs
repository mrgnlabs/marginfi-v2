use crate::{
    constants::{CAMPAIGN_AUTH_SEED, CAMPAIGN_SEED},
    state::Campaign,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use marginfi::state::bank::Bank;
use std::mem::size_of;

pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateCampaign<'info>>,
    lockup_period: u64,
    max_deposits: u64,
    max_rewards: u64,
) -> Result<()> {
    require_gt!(max_deposits, 0);

    anchor_spl::token_2022::spl_token_2022::onchain::invoke_transfer_checked(
        ctx.accounts.token_program.key,
        ctx.accounts.funding_account.to_account_info(),
        ctx.accounts.asset_mint.to_account_info(),
        ctx.accounts.campaign_reward_vault.to_account_info(),
        ctx.accounts.admin.to_account_info(),
        ctx.remaining_accounts,
        max_rewards,
        ctx.accounts.asset_mint.decimals,
        &[], // seeds
    )?;

    // Get new balance. This will account for any fees
    ctx.accounts.campaign_reward_vault.reload()?;

    ctx.accounts.campaign.set_inner(Campaign {
        admin: ctx.accounts.admin.key(),
        lockup_period,
        active: true,
        max_deposits,
        remaining_capacity: max_deposits,
        max_rewards: ctx.accounts.campaign_reward_vault.amount,
        marginfi_bank_pk: ctx.accounts.marginfi_bank.key(),
        _padding: [0; 16],
    });

    Ok(())
}

#[derive(Accounts)]
pub struct CreateCampaign<'info> {
    #[account(
        init,
        payer = admin,
        space = size_of::<Campaign>() + 8,
    )]
    pub campaign: Box<Account<'info, Campaign>>,
    #[account(
        init,
        payer = admin,
        token::mint = asset_mint,
        token::authority = campaign_reward_vault_authority,
        seeds = [
            CAMPAIGN_SEED.as_bytes(),
            campaign.key().as_ref(),
        ],
        bump,
    )]
    pub campaign_reward_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        seeds = [
            CAMPAIGN_AUTH_SEED.as_bytes(),
            campaign.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: Asserted by PDA derivation
    pub campaign_reward_vault_authority: AccountInfo<'info>,
    #[account(
        address = marginfi_bank.load()?.mint,
    )]
    /// CHECK: Must match the mint of the marginfi bank,
    /// asserted by comparing the mint of the marginfi bank
    pub asset_mint: InterfaceAccount<'info, Mint>,
    pub marginfi_bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: Asserted by token check
    #[account(mut)]
    pub funding_account: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
