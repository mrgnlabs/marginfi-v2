use crate::{prelude::MarginfiResult, MarginfiError};
use anchor_lang::prelude::*;

#[inline]
pub fn assert_within_one_token(actual: u64, expected: u64, err: MarginfiError) -> MarginfiResult {
    // Should be sufficient tolerance for 99.999999% of cases
    const TOKEN_AMOUNT_TOLERANCE: u64 = 1;

    let diff = actual.abs_diff(expected);
    if diff > TOKEN_AMOUNT_TOLERANCE {
        msg!(
            "Amount mismatch → actual: {}, expected: {}, diff: {}",
            actual,
            expected,
            diff
        );
        return Err(err.into());
    }
    Ok(())
}
