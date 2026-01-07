# JupLend IDL Notes

## Modified: juplend_earn.json

The following account types were removed from the original JupLend `lending.json` IDL to fix compilation errors:

- `TokenReserve`
- `UserSupplyPosition`

**Reason:** These accounts use `"serialization": "bytemuck"` with `"repr": {"kind": "c", "packed": true}`. When Anchor's `declare_program!` macro generates Rust types for these, they fail to implement the `Pod` trait (`AnyBitPattern` not satisfied).

**Impact:** These accounts are not used by marginfi's JupLend integration - only the `Lending` account is needed for CPI operations (deposit, withdraw, update_rate).

**Remaining accounts:** `Lending`, `LendingAdmin`, `LendingRewardsRateModel`
