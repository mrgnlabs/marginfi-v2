use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{close_account, CloseAccount},
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use fixed::types::I80F48;
use marginfi::{program::Marginfi, state::marginfi_group::Bank};

use crate::{
    constants::{
        CAMPAIGN_AUTH_SEED, CAMPAIGN_SEED, DEPOSIT_MFI_AUTH_SIGNER_SEED, MARGINFI_ACCOUNT_SEED,
        TEMP_TOKEN_ACCOUNT_AUTH_SEED,
    },
    errors::LIPError,
    state::{Campaign, Deposit},
};

/// After a lockup period has ended, closes a deposit and returns the initial deposit + earned rewards from a liquidity incentive campaign back to the liquidity depositor.
///
/// # Arguments
/// * ctx: Context of the deposit to be closed
///
/// # Returns
/// * A Result object which is Ok(()) if the deposit is closed and tokens are transferred successfully.
///
/// # Errors
/// Returns an error if:
///
/// * Solana clock timestamp is less than the deposit start time plus the lockup period (i.e. the lockup has not been reached)
/// * Bank redeem shares operation fails
/// * Reloading ephemeral token account fails
/// * Transferring additional reward to ephemeral token account fails
/// * Reloading ephemeral token account after transfer fails
pub fn process<'info>(ctx: Context<'_, '_, '_, 'info, EndDeposit<'info>>) -> Result<()> {
    // Solana clock isn't the most precise, but an offset of a few hours on a half year lockup is fine
    //
    // Check if the lockup period has passed
    require_gte!(
        Clock::get()?.unix_timestamp,
        // Skipping checked math here as numbers should be small enough to not overflow
        ctx.accounts.deposit.start_time + ctx.accounts.campaign.lockup_period as i64,
        LIPError::DepositNotMature
    );

    let deposit_key = ctx.accounts.deposit.key().to_bytes();
    let signer_seeds: &[&[&[u8]]] = &[&[
        DEPOSIT_MFI_AUTH_SIGNER_SEED.as_bytes(),
        deposit_key.as_ref(),
        &[ctx.bumps.mfi_pda_signer],
    ]];
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.marginfi_program.to_account_info(),
        marginfi::cpi::accounts::LendingAccountWithdraw {
            group: ctx.accounts.marginfi_group.to_account_info(),
            marginfi_account: ctx.accounts.marginfi_account.to_account_info(),
            authority: ctx.accounts.mfi_pda_signer.to_account_info(),
            bank: ctx.accounts.marginfi_bank.to_account_info(),
            destination_token_account: ctx.accounts.temp_token_account.to_account_info(),
            liquidity_vault: ctx.accounts.marginfi_bank_vault.to_account_info(),
            bank_liquidity_vault_authority: ctx
                .accounts
                .marginfi_bank_vault_authority
                .to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
        signer_seeds,
    );
    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    marginfi::cpi::lending_account_withdraw(cpi_ctx, 0, Some(true))?;

    // Redeem the shares with marginfi
    ctx.accounts.temp_token_account.reload()?;

    // Calulate additional rewards that need to be payed out, based on guaranteed yield.
    // This is done by calculating the difference between guaranteed yield and actual yield.
    let additional_reward_amount = {
        let initial_deposit = ctx.accounts.deposit.amount;
        let end_deposit = ctx.accounts.temp_token_account.amount;

        let base_yield = end_deposit.saturating_sub(initial_deposit);

        let max_rewards_pre_campaign = I80F48::from_num(ctx.accounts.campaign.max_rewards);
        let max_deposits_pre_campaign = I80F48::from_num(ctx.accounts.campaign.max_deposits);
        let deposit_amount = I80F48::from_num(ctx.accounts.deposit.amount);

        let max_reward_for_deposit = deposit_amount
            .checked_div(max_deposits_pre_campaign)
            .unwrap()
            .checked_mul(max_rewards_pre_campaign)
            .unwrap()
            .checked_to_num::<u64>()
            .unwrap();

        msg!(
            "Base yield: {}, max reward for deposit: {}",
            base_yield,
            max_reward_for_deposit
        );

        max_reward_for_deposit.saturating_sub(base_yield)
    };

    msg!("Additional reward amount: {}", additional_reward_amount);

    // Transfer any additional rewards to the ephemeral token account
    if additional_reward_amount > 0 {
        let campaign_key = ctx.accounts.campaign.key();
        let campaign_auth_seeds: &[&[&[u8]]] = &[&[
            CAMPAIGN_AUTH_SEED.as_bytes(),
            campaign_key.as_ref(),
            &[ctx.bumps.campaign_reward_vault_authority],
        ]];
        anchor_spl::token_2022::spl_token_2022::onchain::invoke_transfer_checked(
            ctx.accounts.token_program.key,
            ctx.accounts.campaign_reward_vault.to_account_info(),
            ctx.accounts.asset_mint.to_account_info(),
            ctx.accounts.temp_token_account.to_account_info(),
            ctx.accounts
                .campaign_reward_vault_authority
                .to_account_info(),
            ctx.remaining_accounts,
            additional_reward_amount,
            ctx.accounts.asset_mint.decimals,
            campaign_auth_seeds,
        )?;

        ctx.accounts.temp_token_account.reload()?;
    }

    msg!(
        "Transferring {} tokens to user",
        ctx.accounts.temp_token_account.amount
    );

    // Transfer the total:: amount to the user
    let temp_token_seeds: &[&[&[u8]]] = &[&[
        TEMP_TOKEN_ACCOUNT_AUTH_SEED.as_bytes(),
        deposit_key.as_ref(),
        &[ctx.bumps.temp_token_account_authority],
    ]];
    anchor_spl::token_2022::spl_token_2022::onchain::invoke_transfer_checked(
        ctx.accounts.token_program.key,
        ctx.accounts.temp_token_account.to_account_info(),
        ctx.accounts.asset_mint.to_account_info(),
        ctx.accounts.destination_account.to_account_info(),
        ctx.accounts.temp_token_account_authority.to_account_info(),
        ctx.remaining_accounts,
        ctx.accounts.temp_token_account.amount,
        ctx.accounts.asset_mint.decimals,
        temp_token_seeds,
    )?;

    // Close the temp token account
    close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.temp_token_account.to_account_info(),
            destination: ctx.accounts.signer.to_account_info(),
            authority: ctx.accounts.temp_token_account_authority.to_account_info(),
        },
        &[&[
            TEMP_TOKEN_ACCOUNT_AUTH_SEED.as_bytes(),
            ctx.accounts.deposit.key().as_ref(),
            &[ctx.bumps.temp_token_account_authority],
        ]],
    ))?;

    Ok(())
}

#[derive(Accounts)]
pub struct EndDeposit<'info> {
    #[account(address = deposit.campaign)]
    pub campaign: Box<Account<'info, Campaign>>,

    #[account(
        mut,
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

    #[account(mut, address = deposit.owner)]
    pub signer: Signer<'info>,

    #[account(
        mut,
        close = signer,
    )]
    pub deposit: Box<Account<'info, Deposit>>,

    #[account(
        seeds = [
            DEPOSIT_MFI_AUTH_SIGNER_SEED.as_bytes(),
            deposit.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: Asserted by PDA derivation
    pub mfi_pda_signer: AccountInfo<'info>,

    #[account(
        init,
        payer = signer,
        token::mint = asset_mint,
        token::authority = temp_token_account_authority,
    )]
    pub temp_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        seeds = [
            TEMP_TOKEN_ACCOUNT_AUTH_SEED.as_bytes(),
            deposit.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: Asserted by PDA derivation
    pub temp_token_account_authority: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Asserted by token transfer
    pub destination_account: AccountInfo<'info>,

    #[account(address = marginfi_bank.load()?.mint)]
    /// CHECK: Asserted by constraint
    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [
            MARGINFI_ACCOUNT_SEED.as_bytes(),
            deposit.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: Asserted by PDA derivation
    pub marginfi_account: AccountInfo<'info>,

    /// CHECK: Asserted by CPI call
    pub marginfi_group: AccountInfo<'info>,

    #[account(
        mut,
        address = campaign.marginfi_bank_pk,
    )]
    pub marginfi_bank: AccountLoader<'info, Bank>,

    /// CHECK: Asserted by CPI call
    #[account(mut)]
    pub marginfi_bank_vault: AccountInfo<'info>,

    // /// CHECK: Asserted by CPI call
    // #[account()]
    // pub bank_mint: InterfaceAccount<'info, Mint>,
    /// CHECK: Asserted by CPI call
    #[account(mut)]
    pub marginfi_bank_vault_authority: AccountInfo<'info>,

    /// CHECK: Asserted by CPI call
    pub marginfi_program: Program<'info, Marginfi>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
