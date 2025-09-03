use fixtures::prelude::*;
use marginfi_type_crate::types::{FeeState, MarginfiGroup, PanicState};
use pretty_assertions::assert_eq;
use solana_program_test::*;

#[tokio::test]
async fn test_panic_pause_and_unpause_instructions() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    let marginfi_group = &test_f.marginfi_group;

    let fee_state: FeeState = test_f.load_and_deserialize(&marginfi_group.fee_state).await;
    assert!(!fee_state.panic_state.is_paused_flag());
    assert_eq!(fee_state.panic_state.daily_pause_count, 0);
    assert_eq!(fee_state.panic_state.consecutive_pause_count, 0);

    marginfi_group.try_panic_pause().await?;

    let fee_state: FeeState = test_f.load_and_deserialize(&marginfi_group.fee_state).await;
    assert!(fee_state.panic_state.is_paused_flag());
    assert_eq!(fee_state.panic_state.daily_pause_count, 1);
    assert_eq!(fee_state.panic_state.consecutive_pause_count, 1);
    assert_eq!(fee_state.panic_state.pause_start_timestamp, 0);

    marginfi_group.try_panic_unpause().await?;

    let fee_state: FeeState = test_f.load_and_deserialize(&marginfi_group.fee_state).await;
    assert!(!fee_state.panic_state.is_paused_flag());
    assert_eq!(fee_state.panic_state.consecutive_pause_count, 0);
    assert_eq!(fee_state.panic_state.pause_start_timestamp, 0);
    assert_eq!(fee_state.panic_state.daily_pause_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_panic_daily_limits_with_instructions() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    let marginfi_group = &test_f.marginfi_group;

    marginfi_group.try_panic_pause().await?;

    let fee_state: FeeState = test_f.load_and_deserialize(&marginfi_group.fee_state).await;
    assert_eq!(fee_state.panic_state.daily_pause_count, 1);
    assert_eq!(fee_state.panic_state.consecutive_pause_count, 1);
    assert!(fee_state.panic_state.is_paused_flag());

    marginfi_group.try_panic_unpause().await?;

    let fee_state: FeeState = test_f.load_and_deserialize(&marginfi_group.fee_state).await;
    assert_eq!(fee_state.panic_state.daily_pause_count, 1);
    assert_eq!(fee_state.panic_state.consecutive_pause_count, 0);
    assert!(!fee_state.panic_state.is_paused_flag());

    Ok(())
}

#[tokio::test]
async fn test_panic_state_cache_with_instructions() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    let marginfi_group = &test_f.marginfi_group;

    let marginfi_group_state: MarginfiGroup =
        test_f.load_and_deserialize(&marginfi_group.key).await;
    assert!(!marginfi_group_state.panic_state_cache.is_paused());

    marginfi_group.try_panic_pause().await?;

    let fee_state: FeeState = test_f.load_and_deserialize(&marginfi_group.fee_state).await;
    assert!(fee_state.panic_state.is_paused_flag());

    let current_time = fee_state.panic_state.pause_start_timestamp;
    assert!(!fee_state
        .panic_state
        .is_expired(current_time + PanicState::PAUSE_DURATION_SECONDS - 1));
    assert!(fee_state
        .panic_state
        .is_expired(current_time + PanicState::PAUSE_DURATION_SECONDS));

    Ok(())
}

#[tokio::test]
async fn test_protocol_pause_check_with_instructions() -> anyhow::Result<()> {
    let test_f = TestFixture::new(None).await;
    let marginfi_group = &test_f.marginfi_group;

    let group_state: MarginfiGroup = test_f.load_and_deserialize(&marginfi_group.key).await;
    assert!(!group_state.panic_state_cache.is_paused());

    marginfi_group.try_panic_pause().await?;

    // Get the fee state and test expiration logic
    let fee_state: FeeState = test_f.load_and_deserialize(&marginfi_group.fee_state).await;
    assert!(fee_state.panic_state.is_paused_flag());

    let current_time = fee_state.panic_state.pause_start_timestamp;
    let expired_time = current_time + PanicState::PAUSE_DURATION_SECONDS + 1;
    assert!(fee_state.panic_state.is_expired(expired_time));

    Ok(())
}
