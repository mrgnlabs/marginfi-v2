use crate::{
    bank_signer, check,
    constants::{
        FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, INSURANCE_VAULT_AUTHORITY_SEED,
        INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED, LIQUIDITY_VAULT_SEED,
    },
    prelude::MarginfiError,
    state::{
        marginfi_account::{BankAccountWrapper, MarginfiAccount, RiskEngine},
        marginfi_group::{
            load_pyth_price_feed, Bank, BankConfig, BankConfigOpt, BankVaultType, GroupConfig,
            MarginfiGroup,
        },
    },
    MarginfiResult,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};
use fixed::types::I80F48;
use std::cmp::{max, min};

pub fn initialize(ctx: Context<InitializeMarginfiGroup>) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_init()?;

    marginfi_group.set_initial_configuration(ctx.accounts.admin.key());

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeMarginfiGroup<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + std::mem::size_of::<MarginfiGroup>(),
    )]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Configure margin group
///
/// Admin only
pub fn configure(ctx: Context<ConfigureMarginfiGroup>, config: GroupConfig) -> MarginfiResult {
    let marginfi_group = &mut ctx.accounts.marginfi_group.load_mut()?;

    marginfi_group.configure(config)?;

    Ok(())
}

#[derive(Accounts)]
pub struct ConfigureMarginfiGroup<'info> {
    #[account(mut)]
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,
}

/// Add a bank to the lending pool
///
/// Admin only
///
/// TODO: Allow for different oracle configurations
pub fn lending_pool_add_bank(
    ctx: Context<LendingPoolAddBank>,
    bank_config: BankConfig,
) -> MarginfiResult {
    let LendingPoolAddBank {
        bank_mint,
        liquidity_vault,
        insurance_vault,
        fee_vault,
        pyth_oracle,
        ..
    } = ctx.accounts;

    let mut bank = ctx.accounts.bank.load_init()?;

    // Verify that pyth oracle
    load_pyth_price_feed(pyth_oracle)?;

    let liquidity_vault_bump = *ctx.bumps.get("liquidity_vault").unwrap();
    let liquidity_vault_authority_bump = *ctx.bumps.get("liquidity_vault_authority").unwrap();
    let insurance_vault_bump = *ctx.bumps.get("insurance_vault").unwrap();
    let insurance_vault_authority_bump = *ctx.bumps.get("insurance_vault_authority").unwrap();
    let fee_vault_bump = *ctx.bumps.get("fee_vault").unwrap();
    let fee_vault_authority_bump = *ctx.bumps.get("fee_vault_authority").unwrap();

    *bank = Bank::new(
        ctx.accounts.marginfi_group.key(),
        bank_config,
        bank_mint.key(),
        bank_mint.decimals,
        liquidity_vault.key(),
        insurance_vault.key(),
        fee_vault.key(),
        Clock::get()?.unix_timestamp,
        liquidity_vault_bump,
        liquidity_vault_authority_bump,
        insurance_vault_bump,
        insurance_vault_authority_bump,
        fee_vault_bump,
        fee_vault_authority_bump,
    );

    bank.config.validate()?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(bank_config: BankConfig)]
pub struct LendingPoolAddBank<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    pub bank_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        space = 8 + std::mem::size_of::<Bank>(),
        payer = admin,
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED,
            bank.key().as_ref(),
        ],
        bump
    )]
    pub liquidity_vault_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = admin,
        token::mint = bank_mint,
        token::authority = liquidity_vault_authority,
        seeds = [
            LIQUIDITY_VAULT_SEED,
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub liquidity_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED,
            bank.key().as_ref(),
        ],
        bump
    )]
    pub insurance_vault_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = admin,
        token::mint = bank_mint,
        token::authority = insurance_vault_authority,
        seeds = [
            INSURANCE_VAULT_SEED,
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub insurance_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            FEE_VAULT_AUTHORITY_SEED,
            bank.key().as_ref(),
        ],
        bump
    )]
    pub fee_vault_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = admin,
        token::mint = bank_mint,
        token::authority = fee_vault_authority,
        seeds = [
            FEE_VAULT_SEED,
            bank.key().as_ref(),
        ],
        bump,
    )]
    pub fee_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(address = bank_config.pyth_oracle)]
    pub pyth_oracle: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn lending_pool_configure_bank(
    ctx: Context<LendingPoolConfigureBank>,
    bank_config: BankConfigOpt,
) -> MarginfiResult {
    let mut bank = ctx.accounts.bank.load_mut()?;

    if let Some(pyth_oracle) = bank_config.pyth_oracle {
        check!(
            pyth_oracle == ctx.accounts.pyth_oracle.key(),
            MarginfiError::InvalidPythAccount
        );

        load_pyth_price_feed(&ctx.accounts.pyth_oracle)?;
    }

    bank.configure(bank_config)?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolConfigureBank<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        address = marginfi_group.load()?.admin,
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// Set only if pyth oracle is being changed otherwise can be a random account.
    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    pub pyth_oracle: UncheckedAccount<'info>,
}

pub fn lending_pool_bank_accrue_interest(
    ctx: Context<LendingPoolBankAccrueInterest>,
) -> MarginfiResult {
    let LendingPoolBankAccrueInterest {
        liquidity_vault_authority,
        insurance_vault,
        fee_vault,
        token_program,
        liquidity_vault,
        ..
    } = ctx.accounts;

    let clock = Clock::get()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let (protocol_fee, insurance_fee) = bank.accrue_interest(&clock)?;

    msg!("Protocol fee: {}", protocol_fee);

    bank.withdraw_spl_transfer(
        protocol_fee,
        Transfer {
            from: liquidity_vault.to_account_info(),
            to: fee_vault.to_account_info(),
            authority: liquidity_vault_authority.to_account_info(),
        },
        token_program.to_account_info(),
        bank_signer!(
            BankVaultType::Liquidity,
            ctx.accounts.bank.key(),
            bank.liquidity_vault_authority_bump
        ),
    )?;

    msg!("Insurance fee: {}", insurance_fee);

    bank.withdraw_spl_transfer(
        insurance_fee,
        Transfer {
            from: liquidity_vault.to_account_info(),
            to: insurance_vault.to_account_info(),
            authority: liquidity_vault_authority.to_account_info(),
        },
        token_program.to_account_info(),
        bank_signer!(
            BankVaultType::Liquidity,
            ctx.accounts.bank.key(),
            bank.liquidity_vault_authority_bump
        ),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolBankAccrueInterest<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED,
            bank.key().as_ref(),
        ],
        bump = bank.load()?.liquidity_vault_authority_bump
    )]
    pub liquidity_vault_authority: AccountInfo<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_SEED,
            bank.key().as_ref(),
        ],
        bump = bank.load()?.liquidity_vault_bump
    )]
    pub liquidity_vault: AccountInfo<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            INSURANCE_VAULT_SEED,
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_bump
    )]
    pub insurance_vault: AccountInfo<'info>,

    /// CHECK: ⋐ ͡⋄ ω ͡⋄ ⋑
    #[account(
        mut,
        seeds = [
            FEE_VAULT_SEED,
            bank.key().as_ref(),
        ],
        bump = bank.load()?.fee_vault_bump
    )]
    pub fee_vault: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

/// Handle a bankrupt marginfi account.
/// 1. Verify account is bankrupt, and lending account belonging to account contains bad debt.
/// 2. Determine the amount of bad debt covered by the insurance fund and the amount socialized between depositors.
/// 3. Cover the bad debt of the bankrupt account.
/// 4. Transfer the insured amount from the insurance fund.
/// 5. Socialize the loss between lenders if any.
pub fn lending_pool_handle_bankruptcy(ctx: Context<LendingPoolHandleBankruptcy>) -> MarginfiResult {
    let LendingPoolHandleBankruptcy {
        marginfi_account: marginfi_account_loader,
        insurance_vault,
        token_program,
        bank: bank_loader,
        ..
    } = ctx.accounts;

    let mut marginfi_account = marginfi_account_loader.load_mut()?;

    RiskEngine::new(&marginfi_account, ctx.remaining_accounts)?.check_account_bankrupt()?;

    let mut bank = bank_loader.load_mut()?;

    let lending_account_balance =
        marginfi_account
            .lending_account
            .balances
            .iter_mut()
            .find(|balance| {
                if let Some(balance) = balance {
                    balance.bank_pk == bank_loader.key()
                } else {
                    false
                }
            });

    check!(
        lending_account_balance.is_some(),
        MarginfiError::LendingAccountBalanceNotFound
    );

    let lending_account_balance = lending_account_balance.unwrap().as_mut().unwrap();

    let bad_debt = bank.get_liability_amount(lending_account_balance.liability_shares.into())?;

    check!(bad_debt > I80F48::ZERO, MarginfiError::BalanceNotBadDebt);

    let (covered_by_insurance, socialized_loss) = {
        let available_insurance_funds = I80F48::from_num(insurance_vault.amount);

        let covered_by_insurance = min(bad_debt, available_insurance_funds);
        let socialized_loss = max(bad_debt - covered_by_insurance, I80F48::ZERO);

        (covered_by_insurance, socialized_loss)
    };

    // Cover bad debt with insurance funds.
    bank.withdraw_spl_transfer(
        covered_by_insurance.to_num(),
        Transfer {
            from: ctx.accounts.insurance_vault.to_account_info(),
            to: ctx.accounts.liquidity_vault.to_account_info(),
            authority: ctx.accounts.insurance_vault_authority.to_account_info(),
        },
        token_program.to_account_info(),
        bank_signer!(
            BankVaultType::Insurance,
            bank_loader.key(),
            bank.insurance_vault_authority_bump
        ),
    )?;

    // Socialize bad debt among depositors.
    bank.socialize_loss(socialized_loss)?;

    // Settle bad debt.
    // The liabilities of this account and global total liabilities are reduced by `bad_debt`
    BankAccountWrapper::find_or_create(
        &bank_loader.key(),
        &mut bank,
        &mut marginfi_account.lending_account,
    )?
    .account_deposit(bad_debt)?;

    Ok(())
}

#[derive(Accounts)]
pub struct LendingPoolHandleBankruptcy<'info> {
    pub marginfi_group: AccountLoader<'info, MarginfiGroup>,

    #[account(address = marginfi_group.load()?.admin)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        constraint = bank.load()?.group == marginfi_group.key(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        mut,
        constraint = marginfi_account.load()?.group == marginfi_group.key(),
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(
        mut,
        seeds = [
            LIQUIDITY_VAULT_SEED,
            bank.key().as_ref(),
        ],
        bump = bank.load()?.liquidity_vault_bump
    )]
    pub liquidity_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [
            INSURANCE_VAULT_SEED,
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_bump
    )]
    pub insurance_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [
            INSURANCE_VAULT_AUTHORITY_SEED,
            bank.key().as_ref(),
        ],
        bump = bank.load()?.insurance_vault_authority_bump
    )]
    pub insurance_vault_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}
