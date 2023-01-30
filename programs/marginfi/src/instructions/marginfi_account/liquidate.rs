use crate::constants::{
    INSURANCE_VAULT_SEED, LIQUIDATION_INSURANCE_FEE, LIQUIDATION_LIQUIDATOR_FEE,
};
use crate::prelude::*;
use crate::state::marginfi_account::{
    calc_asset_quantity, calc_asset_value, get_price, RiskEngine, RiskRequirementType,
};
use crate::state::marginfi_group::{Bank, BankVaultType};
use crate::{
    bank_signer,
    constants::{LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED},
    state::marginfi_account::{BankAccountWrapper, MarginfiAccount},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer};
use fixed::types::I80F48;
use solana_program::clock::Clock;
use solana_program::sysvar::Sysvar;

/// Instruction liquidates a position owned by a margin account that is in a unhealthy state.
/// The liquidator can purchase discounted collateral from the unhealthy account, in exchange for paying its debt.
///
/// ### Liquidation math:
/// #### Symbol definitions:
/// - `L`: Liability token
/// - `A`: Asset token
/// - `q_ll`: Quantity of `L` paid by the liquidator
/// - `q_lf`: Quantity of `L` received by the liquidatee
/// - `q_a`: Quantity of `A` to be liquidated
/// - `p_l`: Price of `L`
/// - `p_a`: Price of `A`
/// - `f_l`: Liquidation fee
/// - `f_i`: Insurance fee
///
/// The liquidator invokes this instruction with `q_a` as input (the total amount of collateral to be liquidated).
/// This is done because `q_a` is the most bounded variable in this process, as if the `q_a` is larger than what the liquidatee has, the instruction will fail.
/// The liquidator can observe how much collateral the liquidatee has, and ensures that the liquidatee will have enough collateral regardless of price action.
///
/// Fees:
/// The liquidator fee is charged in the conversion between the market value of the collateral being liquidated and the liability being covered by the liquidator.
/// The value of the liability is discounted by the liquidation fee.
///
/// The insurance fee is taken from the difference between liability being paid by the liquidator and the liability being received by the liquidatee.
/// This difference is deposited into the insurance fund.
///
/// Accounting changes in the liquidation process:
/// 1. The liquidator removes `q_ll` of `L`
/// 2. The liquidatee receives `q_lf` of `L`
/// 3. The liquidatee removes `q_a` of `A`
/// 4. The liquidator receives `q_a` of `A`
/// 5. The insurance fund receives `q_ll - q_lf` of `L`
///
/// Calculations:
///  
/// `q_ll = q_a * p_a * (1 - f_l) / p_l`
/// `q_lf = q_a * p_a * (1 - (f_l + f_i)) / p_l`
///
/// Risk model
///
/// Assumptions:
///
/// The fundamental idea behind the risk model is that the liquidation always leaves the liquidatee account in a better health than before.
/// This can be achieved by ensuring that for each token the liability has a LTV > 1, each collateral has a risk haircut of < 1,
/// with this by paying down the liability with the collateral, the liquidatee account will always be in a better health,
/// assuming that the liquidatee liability token balance doesn't become positive (doesn't become counted as collateral),
/// and that the liquidatee collateral token balance doesn't become negative (doesn't become counted as liability).
///
pub fn lending_account_liquidate(
    ctx: Context<LendingAccountLiquidate>,
    asset_quantity: u64,
) -> MarginfiResult {
    let LendingAccountLiquidate {
        liquidator_marginfi_account,
        liquidatee_marginfi_account,
        ..
    } = ctx.accounts;

    let mut liquidator_marginfi_account = liquidator_marginfi_account.load_mut()?;
    let mut liquidatee_marginfi_account = liquidatee_marginfi_account.load_mut()?;

    {
        let clock = Clock::get()?;
        ctx.accounts
            .asset_bank
            .load_mut()?
            .accrue_interest(&clock)?;
        ctx.accounts.liab_bank.load_mut()?.accrue_interest(&clock)?;
    }

    let pre_liquidation_health = {
        let liquidatee_accounts_starting_pos =
            ctx.remaining_accounts.len() - liquidatee_marginfi_account.get_remaining_accounts_len();
        let liquidatee_remaining_accounts =
            &ctx.remaining_accounts[liquidatee_accounts_starting_pos..];

        RiskEngine::new(&liquidatee_marginfi_account, liquidatee_remaining_accounts)?
            .check_pre_liquidation_condition_and_get_account_health(&ctx.accounts.liab_bank.key())?
    };

    {
        // ##Accounting changes##

        let asset_quantity = I80F48::from_num(asset_quantity);

        let mut asset_bank = ctx.accounts.asset_bank.load_mut()?;
        let asset_price = {
            let asset_price_feed =
                asset_bank.load_price_feed_from_account_info(&ctx.remaining_accounts[0])?;

            get_price(&asset_price_feed)?
        };

        let mut liab_bank = ctx.accounts.liab_bank.load_mut()?;
        let liab_price = {
            let liab_price_feed =
                liab_bank.load_price_feed_from_account_info(&ctx.remaining_accounts[1])?;

            get_price(&liab_price_feed)?
        };

        let final_discount = I80F48::ONE - (LIQUIDATION_INSURANCE_FEE + LIQUIDATION_LIQUIDATOR_FEE);
        let liquidator_discount = I80F48::ONE - LIQUIDATION_LIQUIDATOR_FEE;

        // Quantity of liability to be paid off by liquidator
        let liab_quantity_liquidator = calc_asset_quantity(
            calc_asset_value(
                asset_quantity,
                asset_price,
                asset_bank.mint_decimals,
                Some(liquidator_discount),
            )?,
            liab_price,
            liab_bank.mint_decimals,
        )?;

        // Quantity of liability to be received by liquidatee
        let liab_quantity_final = calc_asset_quantity(
            calc_asset_value(
                asset_quantity,
                asset_price,
                asset_bank.mint_decimals,
                Some(final_discount),
            )?,
            liab_price,
            liab_bank.mint_decimals,
        )?;

        // Insurance fund fee
        let insurance_fund_fee = liab_quantity_liquidator - liab_quantity_final;

        assert!(
            insurance_fund_fee >= I80F48::ZERO,
            "Insurance fund fee cannot be negative"
        );

        msg!(
            "liab_quantity_liq: {}, liab_q_final: {}, asset_quantity: {}, insurance_fund_fee: {}",
            liab_quantity_liquidator,
            liab_quantity_final,
            asset_quantity,
            insurance_fund_fee
        );

        // Liquidator pays off liability
        BankAccountWrapper::find_or_create(
            &ctx.accounts.liab_bank.key(),
            &mut liab_bank,
            &mut liquidator_marginfi_account.lending_account,
        )?
        .decrease_balance(liab_quantity_liquidator)?;

        // Liquidator receives `asset_quantity` amount of collateral
        BankAccountWrapper::find_or_create(
            &ctx.accounts.asset_bank.key(),
            &mut asset_bank,
            &mut liquidator_marginfi_account.lending_account,
        )?
        .increase_balance(asset_quantity)?;

        // Liquidatee pays off `asset_quantity` amount of collateral
        BankAccountWrapper::find(
            &ctx.accounts.asset_bank.key(),
            &mut asset_bank,
            &mut liquidatee_marginfi_account.lending_account,
        )?
        .withdraw(asset_quantity)?;

        // Liquidatee receives liability payment
        let liab_bank_liquidity_authority_bump = liab_bank.liquidity_vault_authority_bump;

        let mut liquidatee_liab_bank_account = BankAccountWrapper::find_or_create(
            &ctx.accounts.liab_bank.key(),
            &mut liab_bank,
            &mut liquidatee_marginfi_account.lending_account,
        )?;

        liquidatee_liab_bank_account.increase_balance(liab_quantity_final)?;

        // ## SPL transfer ##
        // Insurance fund receives fee
        liquidatee_liab_bank_account.withdraw_spl_transfer(
            insurance_fund_fee.to_num(),
            Transfer {
                from: ctx.accounts.bank_liquidity_vault.to_account_info(),
                to: ctx.accounts.bank_insurance_vault.to_account_info(),
                authority: ctx
                    .accounts
                    .bank_liquidity_vault_authority
                    .to_account_info(),
            },
            ctx.accounts.token_program.to_account_info(),
            bank_signer!(
                BankVaultType::Liquidity,
                ctx.accounts.liab_bank.key(),
                liab_bank_liquidity_authority_bump
            ),
        )?;
    }

    // ## Risk checks ##

    let (liquidator_remaining_accounts, liquidatee_remaining_accounts) = ctx.remaining_accounts
        [2..]
        .split_at(liquidator_marginfi_account.get_remaining_accounts_len());
    // Verify liquidatee liquidation post health
    RiskEngine::new(&liquidatee_marginfi_account, liquidatee_remaining_accounts)?
        .check_post_liquidation_account_health(
            &ctx.accounts.liab_bank.key(),
            pre_liquidation_health,
        )?;

    // Verify liquidator account health
    RiskEngine::new(&liquidator_marginfi_account, liquidator_remaining_accounts)?
        .check_account_health(RiskRequirementType::Initial)?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingAccountLiquidate<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
    mut,
    constraint = asset_bank.load()?.group == marginfi_group.key()
    )]
    pub asset_bank: AccountLoader<'info, Bank>,

    #[account(
    mut,
    constraint = liab_bank.load()?.group == marginfi_group.key()
    )]
    pub liab_bank: AccountLoader<'info, Bank>,

    #[account(
    mut,
    constraint = liquidator_marginfi_account.load()?.group == marginfi_group.key()
    )]
    pub liquidator_marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
    address = liquidator_marginfi_account.load()?.authority
    )]
    pub signer: Signer<'info>,

    #[account(
    mut,
    constraint = liquidatee_marginfi_account.load()?.group == marginfi_group.key()
    )]
    pub liquidatee_marginfi_account: AccountLoader<'info, MarginfiAccount>,

    /// CHECK: Seed constraint
    #[account(
    mut,
    seeds = [
    LIQUIDITY_VAULT_AUTHORITY_SEED,
    liab_bank.key().as_ref(),
    ],
    bump = liab_bank.load()?.liquidity_vault_authority_bump
    )]
    pub bank_liquidity_vault_authority: AccountInfo<'info>,

    /// CHECK: Seed constraint
    #[account(
    mut,
    seeds = [
    LIQUIDITY_VAULT_SEED,
    liab_bank.key().as_ref(),
    ],
    bump = liab_bank.load()?.liquidity_vault_bump
    )]
    pub bank_liquidity_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: Seed constraint
    #[account(
    mut,
    seeds = [
    INSURANCE_VAULT_SEED,
    liab_bank.key().as_ref(),
    ],
    bump = liab_bank.load()?.insurance_vault_bump
    )]
    pub bank_insurance_vault: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}
