use crate::events::{
    AccountEventHeader, KeeperCloseOrderEvent, MarginfiAccountCloseOrderEvent,
    MarginfiAccountPlaceOrderEvent, SetKeeperCloseFlagsEvent,
};
use crate::instructions::marginfi_account::liquidate_start::validate_instructions;
use crate::ix_utils::{
    get_discrim_hash, keys_sha256_hash, validate_not_cpi_by_stack_height, Hashable,
};
use crate::state::marginfi_account::RiskRequirementType;
use crate::{
    check,
    prelude::*,
    state::{
        marginfi_account::{LendingAccountImpl, MarginfiAccountImpl, RiskEngine},
        marginfi_group::MarginfiGroupImpl,
        order::{ExecuteOrderRecordImpl, OrderImpl},
    },
};
use crate::{check_eq, math_error};
use anchor_lang::system_program;
use anchor_lang::{prelude::*, solana_program::sysvar};
use bytemuck::Zeroable;
use fixed::types::I80F48;
use marginfi_type_crate::constants::{ix_discriminators, ORDER_ACTIVE_TAGS};
use marginfi_type_crate::types::{
    BalanceSide, ExecuteOrderRecord, HealthCache, OrderTriggerType, ACCOUNT_IN_ORDER_EXECUTION,
};
use marginfi_type_crate::{
    constants::{EXECUTE_ORDER_SEED, ORDER_SEED},
    types::{
        MarginfiAccount, MarginfiGroup, Order, OrderTrigger, ACCOUNT_DISABLED, ACCOUNT_IN_FLASHLOAN,
    },
};

pub fn place_order(
    ctx: Context<PlaceOrder>,
    bank_keys: Vec<Pubkey>,
    trigger: OrderTrigger,
) -> MarginfiResult {
    let PlaceOrder {
        marginfi_account: marginfi_account_loader,
        order: order_loader,
        ..
    } = &ctx.accounts;

    let mut marginfi_account = marginfi_account_loader.load_mut()?;

    // TODO: Transfer SOL flat fee to the order account for order creation.

    check!(
        !marginfi_account.get_flag(ACCOUNT_DISABLED),
        MarginfiError::AccountDisabled
    );

    check!(
        !marginfi_account.get_flag(ACCOUNT_IN_FLASHLOAN),
        MarginfiError::AccountInFlashloan
    );

    check!(
        bank_keys.len() == ORDER_ACTIVE_TAGS,
        MarginfiError::InvalidBalanceCount
    );

    check!(
        marginfi_account
            .emissions_destination_account
            .ne(&Pubkey::default()),
        MarginfiError::InvalidEmissionsDestinationAccount
    );

    // ORDER_ACTIVE_TAGS == 2
    let bank_key_1 = &bank_keys[0];
    let bank_key_2 = &bank_keys[1];

    check!(bank_key_1 != bank_key_2, MarginfiError::DuplicateBalance);

    let lending_account = &mut marginfi_account.lending_account;

    let balance_index_1 = lending_account
        .balances
        .binary_search_by(|balance| bank_key_1.cmp(&balance.bank_pk))
        .ok()
        .and_then(|index| lending_account.balances[index].is_active().then_some(index))
        .ok_or(MarginfiError::LendingAccountBalanceNotFound)?;

    let balance_index_2 = lending_account
        .balances
        .binary_search_by(|balance| bank_key_2.cmp(&balance.bank_pk))
        .ok()
        .and_then(|index| lending_account.balances[index].is_active().then_some(index))
        .ok_or(MarginfiError::LendingAccountBalanceNotFound)?;

    // Ensure we have one asset and one liability
    match (
        lending_account.balances[balance_index_1].get_side(),
        lending_account.balances[balance_index_2].get_side(),
    ) {
        (Some(BalanceSide::Assets), Some(BalanceSide::Liabilities)) => {}
        (Some(BalanceSide::Liabilities), Some(BalanceSide::Assets)) => {}
        _ => return err!(MarginfiError::InvalidAssetOrLiabilitiesCount),
    };

    let balance_1_needs_tag = lending_account.balances[balance_index_1].tag == 0;
    let balance_2_needs_tag = lending_account.balances[balance_index_2].tag == 0;

    let empty_tag_count = balance_1_needs_tag as usize + balance_2_needs_tag as usize;

    if empty_tag_count > 0 {
        let new_tags = lending_account.reserve_n_tags(empty_tag_count);
        let mut tag_index = 0;

        if balance_1_needs_tag {
            lending_account.balances[balance_index_1].tag = new_tags[tag_index];
            tag_index += 1;
        }

        if balance_2_needs_tag {
            lending_account.balances[balance_index_2].tag = new_tags[tag_index];
        }
    }

    let tags = [
        lending_account.balances[balance_index_1].tag,
        lending_account.balances[balance_index_2].tag,
    ];

    let marginfi_account_key = marginfi_account_loader.key();

    let order_bump = ctx.bumps.order;

    let mut order = order_loader.load_init()?;

    order.initialize(marginfi_account_key, trigger, tags, order_bump)?;

    emit!(MarginfiAccountPlaceOrderEvent {
        header: AccountEventHeader {
            signer: Some(ctx.accounts.authority.key()),
            marginfi_account: marginfi_account_loader.key(),
            marginfi_account_authority: marginfi_account.authority,
            marginfi_group: marginfi_account.group,
        },
        order: order_loader.key(),
        trigger: order.trigger,
        stop_loss: order.stop_loss,
        take_profit: order.take_profit,
        tags,
    });

    Ok(())
}

pub fn close_order(ctx: Context<CloseOrder>) -> MarginfiResult {
    let CloseOrder {
        marginfi_account: marginfi_account_loader,
        authority,
        order: order_loader,
        ..
    } = &ctx.accounts;

    let marginfi_account = marginfi_account_loader.load()?;

    emit!(MarginfiAccountCloseOrderEvent {
        header: AccountEventHeader {
            signer: Some(authority.key()),
            marginfi_account: marginfi_account_loader.key(),
            marginfi_account_authority: marginfi_account.authority,
            marginfi_group: marginfi_account.group,
        },
        order: order_loader.key(),
    });

    Ok(())
}

pub fn keeper_close_order(ctx: Context<KeeperCloseOrder>) -> MarginfiResult {
    let KeeperCloseOrder {
        order: order_loader,
        marginfi_account,
        ..
    } = &ctx.accounts;

    let order = order_loader.load()?;
    let marginfi_account_info = marginfi_account.to_account_info();

    // Manual owner check: Only attempt to deserialize when the account is not closed
    let (authority_pk, group_pk, can_close) = if marginfi_account_info.owner.eq(&system_program::ID)
        && marginfi_account_info.data_is_empty()
    {
        (Pubkey::default(), Pubkey::default(), true)
    } else {
        // Deserialize manually using bytemuck to avoid lifetime issues
        let data = marginfi_account_info.try_borrow_data()?;

        // Check discriminator
        require!(
            data.len() >= 8 + std::mem::size_of::<MarginfiAccount>(),
            MarginfiError::InternalLogicError
        );

        let disc = &data[..8];
        check_eq!(
            disc,
            MarginfiAccount::DISCRIMINATOR,
            MarginfiError::InternalLogicError
        );

        let marginfi_account: &MarginfiAccount =
            bytemuck::from_bytes(&data[8..8 + std::mem::size_of::<MarginfiAccount>()]);

        let balances = &marginfi_account.lending_account.balances;
        let can_close = order.tags.iter().any(|tag| {
            !balances
                .iter()
                .any(|balance| balance.is_active() && balance.tag == *tag)
        });
        (
            marginfi_account.authority,
            marginfi_account.group,
            can_close,
        )
    };

    if !can_close {
        return err!(MarginfiError::LiquidatorOrderCloseNotAllowed);
    }

    emit!(KeeperCloseOrderEvent {
        header: AccountEventHeader {
            signer: None,
            marginfi_account: marginfi_account_info.key(),
            marginfi_account_authority: authority_pk,
            marginfi_group: group_pk,
        },
        order: order_loader.key(),
    });

    Ok(())
}

pub fn set_keeper_close_flags(
    ctx: Context<SetKeeperCloseFlags>,
    bank_keys_opt: Option<Vec<Pubkey>>,
) -> MarginfiResult {
    let SetKeeperCloseFlags {
        marginfi_account, ..
    } = &ctx.accounts;

    let mut marginfi_account = marginfi_account.load_mut()?;

    let balances = &mut marginfi_account.lending_account.balances;

    match bank_keys_opt {
        Some(ref keys) => {
            for bank_key in keys.iter() {
                let index = balances
                    .binary_search_by(|balance| bank_key.cmp(&balance.bank_pk))
                    .map_err(|_| error!(MarginfiError::LendingAccountBalanceNotFound))?;

                let balance = &mut balances[index];
                balance.tag = 0;
            }
        }
        None => {
            for balance in balances.iter_mut() {
                balance.tag = 0;
            }
        }
    }

    emit!(SetKeeperCloseFlagsEvent {
        header: AccountEventHeader {
            signer: Some(ctx.accounts.authority.key()),
            marginfi_account: ctx.accounts.marginfi_account.key(),
            marginfi_account_authority: marginfi_account.authority,
            marginfi_group: marginfi_account.group,
        },
        bank_keys: bank_keys_opt,
    });

    Ok(())
}

pub fn start_execute_order<'info>(
    ctx: Context<'_, '_, 'info, 'info, StartExecuteOrder<'info>>,
) -> MarginfiResult {
    let StartExecuteOrder {
        marginfi_account: marginfi_account_loader,
        fee_payer: _fee_payer,
        executor,
        order: order_loader,
        execute_record: execute_record_loader,
        instruction_sysvar,
        ..
    } = &ctx.accounts;

    let mut marginfi_account = marginfi_account_loader.load_mut()?;
    let mut order = order_loader.load_mut()?;

    marginfi_account.set_flag(ACCOUNT_IN_ORDER_EXECUTION, false);

    let mut health_cache = HealthCache::zeroed();

    let (order_assets_in_equity, order_liabs_in_equity, order_asset_count, order_liab_count) = {
        let risk_engine = RiskEngine::new(&marginfi_account, ctx.remaining_accounts)?;

        let (assets, liabs) = risk_engine.get_account_health_components(
            RiskRequirementType::Maintenance,
            &mut Some(&mut health_cache),
        )?;

        let account_health = assets.checked_sub(liabs).ok_or_else(math_error!())?;

        let healthy = account_health > I80F48::ZERO;

        check!(
            healthy, // If the account is not healthy it should be liquidated instead, regardless of the order.
            MarginfiError::AccountNotHealthy
        );

        risk_engine.get_tagged_account_health_components(&order.tags)?
    };

    check!(
        order_asset_count + order_liab_count == ORDER_ACTIVE_TAGS,
        MarginfiError::LendingAccountBalanceNotFound
    );

    health_cache.set_healthy(true); // We have checked the account to be healthy
    marginfi_account.health_cache = health_cache;

    let net = order_assets_in_equity
        .checked_sub(order_liabs_in_equity)
        .ok_or_else(math_error!())?;

    // Check trigger condition
    match order.trigger {
        OrderTriggerType::StopLoss => {
            let sl: I80F48 = order.stop_loss.into();
            check!(net <= sl, MarginfiError::OrderTriggerNotMet);
            order.stop_loss = net.into();
        }
        OrderTriggerType::TakeProfit => {
            let tp: I80F48 = order.take_profit.into();
            check!(net >= tp, MarginfiError::OrderTriggerNotMet);
        }
        OrderTriggerType::Both => {
            let sl: I80F48 = order.stop_loss.into();
            let tp: I80F48 = order.take_profit.into();
            check!(net <= sl || net >= tp, MarginfiError::OrderTriggerNotMet);
            // This is only used if the stop loss was hit, and in the case of the
            // take profit it serves as a guard, so the keeper would not rely on the
            // sl case instead of tp.
            order.stop_loss = net.into();
        }
    }

    // Create execution record
    let mut execute_record = execute_record_loader.load_init()?;

    // Store the order, executor as well as all the non-order balances.
    execute_record.initialize(
        order_loader.key(),
        executor.key(),
        &marginfi_account,
        &order.tags,
    )?;

    validate_instructions(
        instruction_sysvar,
        ctx.program_id,
        &ix_discriminators::START_EXECUTE_ORDER,
        &ix_discriminators::END_EXECUTE_ORDER,
    )
}

pub fn end_execute_order<'info>(
    ctx: Context<'_, '_, 'info, 'info, EndExecuteOrder<'info>>,
) -> MarginfiResult {
    let EndExecuteOrder {
        marginfi_account: marginfi_account_loader,
        order: order_loader,
        execute_record: execute_record_loader,
        ..
    } = &ctx.accounts;

    validate_not_cpi_by_stack_height()?;

    let mut marginfi_account = marginfi_account_loader.load_mut()?;
    let order = order_loader.load()?;
    let execute_record = execute_record_loader.load()?;

    let mut health_cache = HealthCache::zeroed();

    let (order_assets_in_equity, _order_liabs_in_equity, _order_asset_count, order_liab_count) = {
        let risk_engine = RiskEngine::new(&marginfi_account, ctx.remaining_accounts)?;

        let (assets, liabs) = risk_engine.get_account_health_components(
            RiskRequirementType::Maintenance,
            &mut Some(&mut health_cache),
        )?;

        let account_health = assets.checked_sub(liabs).ok_or_else(math_error!())?;

        let healthy = account_health > I80F48::ZERO;

        check!(
            healthy, // The account should remain healthy
            MarginfiError::AccountNotHealthy
        );

        risk_engine.get_tagged_account_health_components(&order.tags)?
    };

    check!(
        order_liab_count.eq(&0), // All order liabilities should be closed
        MarginfiError::OrderLiabilityNotClosed
    );

    health_cache.set_healthy(true); // We have checked the account to be healthy
    marginfi_account.health_cache = health_cache;

    let net = order_assets_in_equity;

    // Check that the liquidator did not over-withdraw.
    match order.trigger {
        OrderTriggerType::StopLoss => {
            let sl: I80F48 = order.stop_loss.into();
            check!(net >= sl, MarginfiError::OrderTriggerNotMet); // This check is different from the trigger check.
        }
        OrderTriggerType::TakeProfit => {
            let tp: I80F48 = order.take_profit.into();
            // If the liquidator cacthes it as `net > tp`, then they can keep at most `net - tp`.
            check!(net >= tp, MarginfiError::OrderTriggerNotMet);
        }
        OrderTriggerType::Both => {
            let sl: I80F48 = order.stop_loss.into();
            let tp: I80F48 = order.take_profit.into();
            check!(net >= sl || net >= tp, MarginfiError::OrderTriggerNotMet); // Same as in both comments above.
        }
    }

    // Only one asset and liab are currently involved in a balance, with the single liability being closed.
    let closed_order_balances_count = 1;

    // Check that the non-order balances remain unchanged, including inactive ones.
    execute_record.verify_unchanged(&marginfi_account, closed_order_balances_count)?;

    // At this point we know that all non order balances were not touched and the order
    // balances that were touched:-
    // 1) Is still above or equal to the trigger price(in equity terms).
    // 2) Did not make the account unhealthy as it was healthy at the
    //    start of this execution process, so it should not have changed.

    marginfi_account.unset_flag(ACCOUNT_IN_ORDER_EXECUTION, false);

    Ok(())
}

#[derive(Accounts)]
#[instruction(bank_keys: Vec<Pubkey>)]
pub struct PlaceOrder<'info> {
    #[account(
        constraint = (!group.load()?.is_protocol_paused()) @ MarginfiError::ProtocolPaused
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroup,
        has_one = authority @ MarginfiError::Unauthorized
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + std::mem::size_of::<Order>(),
        seeds = [
            ORDER_SEED.as_bytes(),
            marginfi_account.key().as_ref(),
            &keys_sha256_hash(&bank_keys) // This ensures each combination of balances has only one order.
        ],
        bump
    )]
    pub order: AccountLoader<'info, Order>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseOrder<'info> {
    #[account(
        mut,
        has_one = authority @ MarginfiError::Unauthorized
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = marginfi_account,
        close = fee_recipient
    )]
    pub order: AccountLoader<'info, Order>,

    /// CHECK: no checks whatsoever, marginfi account authority decides this without restriction
    #[account(mut)]
    pub fee_recipient: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct KeeperCloseOrder<'info> {
    /// CHECK: This uses an unchecked account here so the instruction can be called even when the
    /// marginfi account was closed.
    /// The ownership check is checked in the handler or/and type checks are made in the handler.
    #[account(mut)]
    pub marginfi_account: UncheckedAccount<'info>,

    /// CHECK: no checks whatsoever, keeper decides this without restriction
    #[account(mut)]
    pub fee_recipient: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = marginfi_account,
        close = fee_recipient
    )]
    pub order: AccountLoader<'info, Order>,
}

#[derive(Accounts)]
pub struct SetKeeperCloseFlags<'info> {
    #[account(
        mut,
        has_one = authority @ MarginfiError::Unauthorized
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct StartExecuteOrder<'info> {
    #[account(
        constraint = (!group.load()?.is_protocol_paused()) @ MarginfiError::ProtocolPaused
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    /// The account owning the order
    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroup,
        constraint = {
            let acc = marginfi_account.load()?;
            !acc.get_flag(ACCOUNT_IN_ORDER_EXECUTION)
                && !acc.get_flag(ACCOUNT_IN_FLASHLOAN)
                && !acc.get_flag(ACCOUNT_DISABLED)
        } @MarginfiError::UnexpectedOrderExecutionState
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// This account will have the authority to withdraw/repay as if they are the user authority
    /// until the end of the tx.
    ///
    /// CHECK: no checks whatsoever, executor decides this without restriction
    pub executor: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = marginfi_account
    )]
    pub order: AccountLoader<'info, Order>,

    /// This keeps track of the relevant state to be checked at the end of execution.
    #[account(
        init,
        payer = fee_payer,
        space = 8 + std::mem::size_of::<ExecuteOrderRecord>(),
        seeds = [
            EXECUTE_ORDER_SEED.as_bytes(),
            order.key().as_ref()
        ],
        bump
    )]
    pub execute_record: AccountLoader<'info, ExecuteOrderRecord>,

    /// CHECK: validated against known hard-coded sysvar key
    #[account(
        address = sysvar::instructions::id()
    )]
    pub instruction_sysvar: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

impl Hashable for StartExecuteOrder<'_> {
    fn get_hash() -> [u8; 8] {
        get_discrim_hash("global", "marginfi_account_start_execute_order")
    }
}

#[derive(Accounts)]
pub struct EndExecuteOrder<'info> {
    #[account(
        constraint = (!group.load()?.is_protocol_paused()) @ MarginfiError::ProtocolPaused
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    /// The account owning the order
    #[account(
        mut,
        has_one = group @ MarginfiError::InvalidGroup,
        constraint = {
            let acc = marginfi_account.load()?;
            acc.get_flag(ACCOUNT_IN_ORDER_EXECUTION)
                && !acc.get_flag(ACCOUNT_IN_FLASHLOAN)
                && !acc.get_flag(ACCOUNT_DISABLED)
        } @MarginfiError::UnexpectedOrderExecutionState
    )]
    pub marginfi_account: AccountLoader<'info, MarginfiAccount>,

    /// The executioner ☠️
    pub executor: Signer<'info>,

    /// CHECK: no checks whatsoever, executor decides this without restriction
    #[account(mut)]
    pub fee_recipient: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = marginfi_account,
        close = fee_recipient
    )]
    pub order: AccountLoader<'info, Order>,

    /// This keeps track of the relevant state to be checked at the end of execution.
    #[account(
        mut,
        has_one = order,
        has_one = executor,
        close = fee_recipient
    )]
    pub execute_record: AccountLoader<'info, ExecuteOrderRecord>,
}

impl Hashable for EndExecuteOrder<'_> {
    fn get_hash() -> [u8; 8] {
        get_discrim_hash("global", "marginfi_account_end_execute_order")
    }
}
