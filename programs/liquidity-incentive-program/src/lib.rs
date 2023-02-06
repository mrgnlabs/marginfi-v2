use anchor_lang::prelude::*;
use anchor_spl::token::close_account;
use anchor_spl::token::{transfer, Transfer};
use anchor_spl::token::{Token, TokenAccount};
use fixed::types::I80F48;
use marginfi::program::Marginfi;
use marginfi::state::marginfi_group::Bank;
use std::mem::size_of;

declare_id!("LipLzUxQftzq77XGVJW5c7UhxbS9ZLyZM9EGiF4Dxs4");

#[program]
pub mod liquidity_incentive_program {
    use anchor_spl::token::CloseAccount;

    use super::*;

    /// Creates a new liquidity incentive campaign (LIP).
    ///
    /// # Arguments
    /// * `ctx`: Context struct containing the relevant accounts for the campaign.
    /// * `lockup_period`: The length of time (in seconds) that a deposit must be locked up for in order to earn the full reward.
    /// * `max_deposits`: The maximum number of tokens that can be deposited into the campaign by liquidity providers.
    /// * `max_rewards`: The maximum amount of rewards that will be distributed to depositors, and also the amount of token rewards transferred into the vault by the campaign creator.
    ///
    /// # Returns
    /// * `Ok(())` if the campaign was successfully created, or an error otherwise.
    pub fn create_campaing(
        ctx: Context<CreateCampaign>,
        lockup_period: u64,
        max_deposits: u64,
        max_rewards: u64,
    ) -> Result<()> {
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.funding_account.to_account_info(),
                    to: ctx.accounts.campaign_reward_vault.to_account_info(),
                    authority: ctx.accounts.admin.to_account_info(),
                },
            ),
            max_rewards,
        )?;

        *ctx.accounts.campaign = Campaign {
            admin: ctx.accounts.admin.key(),
            lockup_period,
            active: true,
            max_deposits,
            outstanding_deposits: max_deposits,
            max_rewards,
            marginfi_bank_pk: ctx.accounts.marginfi_bank.key(),
        };

        msg!("Created campaing\n{:#?}", ctx.accounts.campaign);

        Ok(())
    }

    /// Creates a new deposit in an active liquidity incentive campaign (LIP).
    ///
    /// # Arguments
    /// * `ctx`: Context struct containing the relevant accounts for the new deposit
    /// * `amount`: The amount of tokens to be deposited.
    ///
    /// # Returns
    /// * `Ok(())` if the deposit was successfully made, or an error otherwise.
    ///
    /// # Errors
    /// * `LIPError::CampaignNotActive` if the relevant campaign is not active.
    /// * `LIPError::DepositAmountTooLarge` is the deposit amount exceeds the amount of remaining deposits that can be made into the campaign.
    pub fn create_deposit(ctx: Context<CreateDeposit>, amount: u64) -> Result<()> {
        require!(ctx.accounts.campaign.active, LIPError::CampaignNotActive);

        require_gte!(
            ctx.accounts.campaign.outstanding_deposits,
            amount,
            LIPError::DepositAmountTooLarge
        );

        msg!("User depositing {} tokens", amount);

        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.funding_account.to_account_info(),
                    to: ctx.accounts.temp_token_account.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info(),
                },
            ),
            amount,
        )?;

        let signer_seeds: &[&[&[u8]]] = &[&[
            DEPOSIT_MFI_AUTH_SIGNER_SEED.as_bytes(),
            ctx.accounts.deposit.key().as_ref(),
            &[*ctx.bumps.get("deposit_marginfi_pda_signer").unwrap()],
        ]];

        marginfi::cpi::marginfi_account_initialize(CpiContext::new_with_signer(
            ctx.accounts.marginfi_program.to_account_info(),
            marginfi::cpi::accounts::MarginfiAccountInitialize {
                marginfi_group: ctx.accounts.marginfi_group.to_account_info(),
                signer: ctx.accounts.deposit_marginfi_pda_signer.to_account_info(),
                marginfi_account: ctx.accounts.marginfi_account.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            signer_seeds,
        ))?;

        marginfi::cpi::lending_pool_deposit(
            CpiContext::new_with_signer(
                ctx.accounts.marginfi_program.to_account_info(),
                marginfi::cpi::accounts::LendingPoolDeposit {
                    marginfi_group: ctx.accounts.marginfi_group.to_account_info(),
                    marginfi_account: ctx.accounts.marginfi_account.to_account_info(),
                    signer: ctx.accounts.deposit_marginfi_pda_signer.to_account_info(),
                    bank: ctx.accounts.marginfi_bank.to_account_info(),
                    signer_token_account: ctx.accounts.temp_token_account.to_account_info(),
                    bank_liquidity_vault: ctx.accounts.marginfi_bank_vault.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.temp_token_account.to_account_info(),
                destination: ctx.accounts.signer.to_account_info(),
                authority: ctx.accounts.deposit_marginfi_pda_signer.to_account_info(),
            },
            signer_seeds,
        ))?;

        *ctx.accounts.deposit = Deposit {
            owner: ctx.accounts.signer.key(),
            campaign: ctx.accounts.campaign.key(),
            amount,
            start_time: Clock::get()?.unix_timestamp,
        };

        ctx.accounts.campaign.outstanding_deposits -= amount;

        Ok(())
    }

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
    pub fn close_deposit(ctx: Context<CloseDeposit>) -> Result<()> {
        // Solana clock isn't the most precise, but an offset of a few hours on a half year lockup is fine
        //
        // Check if the lockup period has passed
        require_gte!(
            Clock::get()?.unix_timestamp,
            ctx.accounts.deposit.start_time + ctx.accounts.campaign.lockup_period as i64,
            LIPError::DepositNotMature
        );

        msg!(
            "Redeeming {} shares from marginfi",
            ctx.accounts.deposit_shares_vault.amount
        );

        // Redeem the shares with marginfi
        marginfi::cpi::bank_redeem_shares(
            CpiContext::new_with_signer(
                ctx.accounts.marginfi_program.to_account_info(),
                marginfi::cpi::accounts::BankRedeemShares {
                    marginfi_group: ctx.accounts.marginfi_group.to_account_info(),
                    bank: ctx.accounts.marginfi_bank.to_account_info(),
                    liquidity_vault: ctx.accounts.marginfi_bank_vault.to_account_info(),
                    bank_liquidity_vault_authority: ctx
                        .accounts
                        .marginfi_bank_vault_authority
                        .to_account_info()
                        .clone(),
                    signer: ctx
                        .accounts
                        .deposit_shares_vault_authority
                        .to_account_info(),
                    user_shares_token_account: ctx.accounts.deposit_shares_vault.to_account_info(),
                    user_deposit_token_account: ctx
                        .accounts
                        .ephemeral_token_account
                        .to_account_info(),
                    shares_token_mint: ctx.accounts.marginfi_shares_mint.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                },
                &[&[
                    DEPOSIT_MFI_AUTH_SIGNER_SEED.as_bytes(),
                    ctx.accounts.deposit.key().as_ref(),
                    &[*ctx.bumps.get("deposit_shares_vault_authority").unwrap()],
                ]],
            ),
            ctx.accounts.deposit_shares_vault.amount,
        )?;

        ctx.accounts.ephemeral_token_account.reload()?;

        // Calulate additional rewards that need to be payed out, based on guaranteed yield.
        // This is done by calculating the difference between guaranteed yield and actual yield.
        let additional_reward_amount = {
            let base_yield =
                ctx.accounts.ephemeral_token_account.amount - ctx.accounts.deposit.amount;

            let max_reward_for_deposit = (I80F48::from_num(ctx.accounts.campaign.max_rewards)
                * I80F48::from_num(ctx.accounts.deposit.amount)
                / I80F48::from_num(ctx.accounts.campaign.max_deposits))
            .to_num::<u64>();

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
            transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.campaign_reward_vault.to_account_info(),
                        to: ctx.accounts.ephemeral_token_account.to_account_info(),
                        authority: ctx
                            .accounts
                            .campaign_reward_vault_authority
                            .to_account_info(),
                    },
                    &[&[
                        CAMPAIGN_AUTH_SEED.as_bytes(),
                        ctx.accounts.campaign.key().as_ref(),
                        &[*ctx.bumps.get("campaign_reward_vault_authority").unwrap()],
                    ]],
                ),
                additional_reward_amount,
            )?;

            ctx.accounts.ephemeral_token_account.reload()?;
        }

        msg!(
            "Transferring {} tokens to user",
            ctx.accounts.ephemeral_token_account.amount
        );

        // Transfer the total amount to the user
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.ephemeral_token_account.to_account_info(),
                    to: ctx.accounts.destination_account.to_account_info(),
                    authority: ctx
                        .accounts
                        .ephemeral_token_account_authority
                        .to_account_info(),
                },
                &[&[
                    EPHEMERAL_TOKEN_ACCOUNT_AUTH_SEED.as_bytes(),
                    ctx.accounts.deposit.key().as_ref(),
                    &[*ctx.bumps.get("ephemeral_token_account_authority").unwrap()],
                ]],
            ),
            ctx.accounts.ephemeral_token_account.amount,
        )?;

        // Close the ephemeral token account
        close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::CloseAccount {
                account: ctx.accounts.ephemeral_token_account.to_account_info(),
                destination: ctx.accounts.signer.to_account_info(),
                authority: ctx
                    .accounts
                    .ephemeral_token_account_authority
                    .to_account_info(),
            },
            &[&[
                EPHEMERAL_TOKEN_ACCOUNT_AUTH_SEED.as_bytes(),
                ctx.accounts.deposit.key().as_ref(),
                &[*ctx.bumps.get("ephemeral_token_account_authority").unwrap()],
            ]],
        ))?;

        close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::CloseAccount {
                account: ctx.accounts.deposit_shares_vault.to_account_info(),
                destination: ctx.accounts.signer.to_account_info(),
                authority: ctx
                    .accounts
                    .deposit_shares_vault_authority
                    .to_account_info(),
            },
            &[&[
                DEPOSIT_MFI_AUTH_SIGNER_SEED.as_bytes(),
                ctx.accounts.deposit.key().as_ref(),
                &[*ctx.bumps.get("deposit_shares_vault_authority").unwrap()],
            ]],
        ))?;

        Ok(())
    }
}

#[constant]
pub const CAMPAIGN_SEED: &str = "campaign";
#[constant]
pub const CAMPAIGN_AUTH_SEED: &str = "campaign_auth";
#[constant]
pub const DEPOSIT_MFI_AUTH_SIGNER_SEED: &str = "deposit_mfi_auth";
#[constant]
pub const EPHEMERAL_TOKEN_ACCOUNT_SEED: &str = "ephemeral_token_account";
#[constant]
pub const EPHEMERAL_TOKEN_ACCOUNT_AUTH_SEED: &str = "ephemeral_token_account_auth";

#[derive(Accounts)]
pub struct CreateCampaign<'info> {
    #[account(
        init,
        payer = admin,
        space = size_of::<Campaign>() + 8,
    )]
    pub campaign: Account<'info, Campaign>,
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
    pub campaign_reward_vault: Account<'info, TokenAccount>,
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
    /// CHECK: Asserted by constraint
    pub asset_mint: AccountInfo<'info>,
    pub marginfi_bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: Asserted by token check
    #[account(mut)]
    pub funding_account: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateDeposit<'info> {
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = size_of::<Deposit>() + 8,
    )]
    pub deposit: Account<'info, Deposit>,
    #[account(
        seeds = [
            DEPOSIT_MFI_AUTH_SIGNER_SEED.as_bytes(),
            deposit.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: Asserted by PDA derivation
    pub deposit_marginfi_pda_signer: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: Asserted by token transfer
    pub funding_account: AccountInfo<'info>,
    #[account(
        init,
        payer = signer,
        token::mint = asset_mint,
        token::authority = deposit_marginfi_pda_signer,
    )]
    pub temp_token_account: Account<'info, TokenAccount>,
    /// CHECK: Asserted by mfi cpi call
    pub asset_mint: AccountInfo<'info>,
    pub marginfi_group: AccountInfo<'info>,
    #[account(
        mut,
        address = campaign.marginfi_bank_pk,
    )]
    /// CHECK: Asserted by stored address
    pub marginfi_bank: AccountInfo<'info>,
    /// CHECK: Asserted by CPI call
    pub marginfi_account: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: Asserted by CPI call
    pub marginfi_bank_vault: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: Asserted by CPI call
    pub marginfi_program: Program<'info, Marginfi>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseDeposit<'info> {
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
    pub campaign_reward_vault: Box<Account<'info, TokenAccount>>,
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
        mut,
        seeds = [
            DEPOSIT_SEED.as_bytes(),
            deposit.key().as_ref(),
        ],
        bump,
    )]
    pub deposit_shares_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        seeds = [
            DEPOSIT_MFI_AUTH_SIGNER_SEED.as_bytes(),
            deposit.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: Asserted by PDA derivation
    pub deposit_shares_vault_authority: AccountInfo<'info>,
    #[account(
        init,
        payer = signer,
        token::mint = asset_mint,
        token::authority = ephemeral_token_account_authority,
        seeds = [
            EPHEMERAL_TOKEN_ACCOUNT_SEED.as_bytes(),
            deposit.key().as_ref(),
        ],
        bump,
    )]
    pub ephemeral_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        seeds = [
            EPHEMERAL_TOKEN_ACCOUNT_AUTH_SEED.as_bytes(),
            deposit.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: Asserted by PDA derivation
    pub ephemeral_token_account_authority: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: Asserted by token transfer
    pub destination_account: AccountInfo<'info>,
    #[account(address = marginfi_bank.load()?.mint)]
    /// CHECK: Asserted by constraint
    pub asset_mint: AccountInfo<'info>,
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
    /// CHECK: Asserted by CPI call
    #[account(mut)]
    pub marginfi_bank_vault_authority: AccountInfo<'info>,
    /// CHECK: Asserted by CPI call
    #[account(mut)]
    pub marginfi_shares_mint: AccountInfo<'info>,
    pub marginfi_program: Program<'info, Marginfi>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(Debug)]
pub struct Campaign {
    pub admin: Pubkey,
    pub lockup_period: u64,
    pub active: bool,
    pub max_deposits: u64,
    pub outstanding_deposits: u64,
    pub max_rewards: u64,
    pub marginfi_bank_pk: Pubkey,
}

#[account]
pub struct Deposit {
    pub owner: Pubkey,
    pub amount: u64,
    pub start_time: i64,
    pub campaign: Pubkey,
}

#[error_code]
pub enum LIPError {
    #[msg("Campaign is not active")]
    CampaignNotActive,
    #[msg("Deposit amount is to large")]
    DepositAmountTooLarge,
    #[msg("Deposit hasn't matured yet")]
    DepositNotMature,
}
