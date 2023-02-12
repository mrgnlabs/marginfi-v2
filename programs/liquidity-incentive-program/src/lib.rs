use anchor_lang::prelude::*;
use instructions::*;

cfg_if::cfg_if! {
    if #[cfg(feature = "mainnet-beta")] {
        declare_id!("LipsxuAkFkwa4RKNzn51wAsW7Dedzt1RNHMkTkDEZUW");
    } else if #[cfg(feature = "devnet")] {
        declare_id!("sexyDKo4Khm38YdJeiRdNNd5aMQqNtfDkxv7MnYNFeU");
    } else {
        declare_id!("Lip1111111111111111111111111111111111111111");
    }
}

pub mod constants;
pub mod errors;
mod instructions;
pub mod state;

#[program]
pub mod liquidity_incentive_program {
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
    pub fn create_campaign(
        ctx: Context<CreateCampaign>,
        lockup_period: u64,
        max_deposits: u64,
        max_rewards: u64,
    ) -> Result<()> {
        create_campaign::process(ctx, lockup_period, max_deposits, max_rewards)
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
        instructions::create_deposit::process(ctx, amount)
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
    pub fn end_deposit(ctx: Context<EndDeposit>) -> Result<()> {
        instructions::end_deposit::process(ctx)
    }
}
