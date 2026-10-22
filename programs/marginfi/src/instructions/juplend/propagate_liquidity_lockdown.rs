use anchor_lang::prelude::*;
use juplend_mocks::state::{LiquidityStatus, MinimalLiquidity};
use marginfi_type_crate::{
    constants::{ASSET_TAG_JUPLEND, JUPLEND_LIQUIDITY_LOCKDOWN},
    pdas::{JUPLEND_LIQUIDITY_PDA, JUPLEND_LIQUIDITY_PROGRAM_ID},
    types::Bank,
};

use crate::{events::JuplendLiquidityLockdownChangedEvent, MarginfiError, MarginfiResult};

pub fn propagate_juplend_liquidity_lockdown(
    ctx: Context<PropagateJuplendLiquidityLockdown>,
) -> MarginfiResult {
    let liquidity = ctx.accounts.liquidity.to_account_info();
    let mut bank = ctx.accounts.bank.load_mut()?;
    refresh_juplend_liquidity_lockdown(&mut bank, ctx.accounts.bank.key(), &liquidity)
}

pub fn refresh_juplend_liquidity_lockdown(
    bank: &mut Bank,
    bank_key: Pubkey,
    liquidity: &AccountInfo<'_>,
) -> MarginfiResult {
    let borrowed = liquidity.try_borrow_data();
    let data: &[u8] = match &borrowed {
        Ok(data) => &data[..],
        Err(_) => &[],
    };
    let previous_flags = bank.flags;
    let previous_locked = previous_flags & JUPLEND_LIQUIDITY_LOCKDOWN != 0;
    bank.flags = propagated_bank_flags(
        bank.config.asset_tag,
        previous_flags,
        liquidity.key,
        liquidity.owner,
        data,
    )?;
    let is_locked = bank.flags & JUPLEND_LIQUIDITY_LOCKDOWN != 0;
    if previous_locked != is_locked {
        emit!(JuplendLiquidityLockdownChangedEvent {
            bank: bank_key,
            was_locked: previous_locked,
            is_locked,
        });
    }
    Ok(())
}

fn propagated_bank_flags(
    asset_tag: u8,
    flags: u64,
    liquidity_key: &Pubkey,
    liquidity_owner: &Pubkey,
    liquidity_data: &[u8],
) -> MarginfiResult<u64> {
    require_eq!(
        asset_tag,
        ASSET_TAG_JUPLEND,
        MarginfiError::AssetTagMismatch
    );
    require_keys_eq!(
        *liquidity_key,
        JUPLEND_LIQUIDITY_PDA,
        MarginfiError::JuplendLiquidityValidationFailed
    );

    let unlocked = *liquidity_owner == JUPLEND_LIQUIDITY_PROGRAM_ID
        && matches!(
            MinimalLiquidity::read_status(liquidity_data),
            Some(LiquidityStatus::Active)
        );
    Ok(if unlocked {
        flags & !JUPLEND_LIQUIDITY_LOCKDOWN
    } else {
        flags | JUPLEND_LIQUIDITY_LOCKDOWN
    })
}

#[derive(Accounts)]
pub struct PropagateJuplendLiquidityLockdown<'info> {
    /// CHECK: JupLend liquidity account; validation is performed in refresh_juplend_liquidity_lockdown
    pub liquidity: UncheckedAccount<'info>,

    #[account(mut)]
    pub bank: AccountLoader<'info, Bank>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use marginfi_type_crate::constants::ASSET_TAG_DEFAULT;

    #[test]
    fn propagation_refreshes_flags_and_rejects_invalid_targets() {
        let flags = 1 << 20;
        let key = JUPLEND_LIQUIDITY_PDA;
        let owner = JUPLEND_LIQUIDITY_PROGRAM_ID;
        let active = MinimalLiquidity::account_data(0);
        let locked = MinimalLiquidity::account_data(1);

        let unlocked_flags = propagated_bank_flags(
            ASSET_TAG_JUPLEND,
            flags | JUPLEND_LIQUIDITY_LOCKDOWN,
            &key,
            &owner,
            &active,
        )
        .unwrap();
        assert_eq!(unlocked_flags, flags);

        let locked_flags =
            propagated_bank_flags(ASSET_TAG_JUPLEND, flags, &key, &owner, &locked).unwrap();
        assert_ne!(locked_flags & JUPLEND_LIQUIDITY_LOCKDOWN, 0);

        let wrong_owner_flags = propagated_bank_flags(
            ASSET_TAG_JUPLEND,
            flags,
            &key,
            &Pubkey::new_unique(),
            &active,
        )
        .unwrap();
        assert_ne!(wrong_owner_flags & JUPLEND_LIQUIDITY_LOCKDOWN, 0);

        assert_eq!(
            propagated_bank_flags(
                ASSET_TAG_JUPLEND,
                flags,
                &Pubkey::new_unique(),
                &owner,
                &active,
            )
            .unwrap_err(),
            error!(MarginfiError::JuplendLiquidityValidationFailed)
        );
        assert_eq!(
            propagated_bank_flags(ASSET_TAG_DEFAULT, flags, &key, &owner, &active).unwrap_err(),
            error!(MarginfiError::AssetTagMismatch)
        );
    }
}
