# Kamino Integration Merge Fixes

## Overview
Tracking all fixes needed after merging `origin/main` into `kamino-integration` branch.

## Current Status
- [x] Program build failing - FIXED
- [x] Test compilation errors - FIXED  
- [ ] Tests likely failing at runtime
- [ ] Kamino-specific functionality needs verification

## Key Changes from Main to Adapt
1. **BankCache System** - All operations must call `bank.update_bank_cache(group)?`
2. **Oracle Migration** - Pyth Legacy → Pyth Push
3. **Account Transfer Redesign** - New transfer mechanism
4. **Multiple Admin Roles** - 5 admins instead of 1
5. **Bank Configuration Split** - New config functions

## Build Issues
### To investigate:
- [ ] Check current build errors
- [ ] Fix compilation issues
- [ ] Update Kamino operations for new architecture

## Test Issues
### Expected failures:
- [ ] Oracle setup changes (single key vs oracle+feed)
- [ ] Function signature changes
- [ ] New admin structure
- [ ] Bank cache updates

## Progress Log

### Build Fixes (COMPLETED)
1. **kamino/add_pool.rs** - Updated BankConfigCompact initialization:
   - Added `config_flags: 1` (bank created in 0.1.4+)
   - Added `oracle_max_confidence: 0` (use default 10%)
   - Changed `_pad0: [0; 6]` to `_pad0: [0; 5]`

2. **state/price.rs** - Fixed PythPushOraclePriceFeed::load_checked:
   - Changed `price_feed_id` to `Some(price_feed_id)` (now expects Option)

### Bank Cache Updates (COMPLETED)
1. **kamino/deposit.rs** - Added bank cache update after deposit:
   - Load group before deposit
   - Call `bank.update_bank_cache(group)?` after `bank_account.deposit_no_repay()`

2. **kamino/withdraw.rs** - Added bank cache update after withdraw:
   - Restructured to keep bank mutable in same scope
   - Call `bank.update_bank_cache(group)?` after `bank_account.withdraw()`

3. **Other Kamino operations** - No changes needed:
   - `harvest_reward` - Only harvests rewards, doesn't modify bank shares
   - `init_obligation` - Initializes obligation, doesn't modify marginfi bank shares

### Kamino Oracle Migration to New Pattern (COMPLETED)
Updated KaminoPythPush to use the new Pyth Push oracle pattern:

1. **price.rs - load_price_feed()**: 
   - Added oracle account validation: `require_keys_eq!(account_info.key, bank_config.oracle_keys[0])`
   - Changed to use `None` for feed_id parameter: `PythPushOraclePriceFeed::load_checked(account_info, None, clock, max_age)`
   - Removed the old `get_pyth_push_oracle_feed_id()` call

2. **price.rs - validate_bank_config()**:
   - Updated validation to check oracle account key matches
   - Added `load_price_update_v2_checked()` to validate it's a proper Pyth Push oracle

3. **marginfi_group.rs - get_pyth_push_oracle_feed_id()**:
   - Removed `KaminoPythPush` from the match pattern since it no longer stores feed ID in oracle_keys[0]

4. **No changes needed in add_pool.rs**:
   - Already sets `oracle_keys[0] = bank_config.oracle` (the oracle account address)

### Test Compilation Fixes (COMPLETED)
Fixed test compilation errors due to Bank struct changes:

1. **setup_bank.rs** - Updated test assertions:
   - Added checks for `kamino_reserve` and `kamino_obligation` fields
   - Changed `_padding_1` from size 19 to 15

2. **regression.rs** - Updated test assertions:
   - Added checks for new Kamino fields
   - Updated padding size

## Summary of All Program & Test Fixes

1. **Program Compilation** ✅
   - Fixed BankConfigCompact initialization
   - Fixed PythPushOraclePriceFeed::load_checked calls
   - Added bank cache updates to Kamino operations
   - Migrated Kamino oracle to new Pyth Push pattern

2. **Test Compilation** ✅
   - Updated test assertions for new Bank struct layout
   - Added Kamino field checks

The program now builds successfully with all Kamino integration changes adapted to the new marginfi architecture from main branch.

### TypeScript Test Fixes (COMPLETED)

#### Oracle References Fixed:
1. **Oracle Field Mappings**:
   - `oracles.*Pull.publicKey` → `oracles.*Oracle.publicKey` (e.g., `usdcPull` → `usdcOracle`)
   - `oracles.*PullOracleFeed.publicKey` → `oracles.*OracleFeed.publicKey`
   - `oracles.lstPull` → `oracles.pythPullLst`
   - `oracles.lstPullOracleFeed` → `oracles.pythPullLstOracleFeed`
   - `oracles.lstPrice` → `oracles.lstAlphaPrice`
   - `oracles.lstDecimals` → `oracles.lstAlphaDecimals`

2. **Confidence Value Handling**:
   - Removed: `oracles.confidenceValue` field
   - Replaced with: `ORACLE_CONF_INTERVAL = 0.01` constant (1% confidence)
   - **IMPORTANT Pattern**:
     - When using **raw price** (without `* 10 ** decimals`): Use `ORACLE_CONF_INTERVAL` directly
     - When using **native price** (already multiplied by `10 ** decimals`): Confidence is already scaled
     - Example: `setPythPullOraclePrice` expects raw price and applies decimals internally

3. **Files Updated**:
   - k05_kaminoBankInit.spec.ts
   - k10_kaminoLiquidate.spec.ts  
   - k12_BorrowPostInterest.spec.ts
   - genericSetups.ts
   - rootHooks.ts
   - bankrun-oracles.ts
   - e03_emodeBorrow.spec.ts

## Oracle Migration Guide

### Oracle Reference Changes
The oracle system was simplified in main branch:

1. **Field Removals**:
   - No more separate `*Pull` and `*PullOracleFeed` fields
   - `confidenceValue` field removed from Oracles type

2. **Field Mappings**:
   ```typescript
   // Old → New
   oracles.usdcPull → oracles.usdcOracle
   oracles.usdcPullOracleFeed → oracles.usdcOracleFeed
   oracles.tokenAPull → oracles.tokenAOracle
   oracles.tokenAPullOracleFeed → oracles.tokenAOracleFeed
   oracles.lstPull → oracles.pythPullLst
   oracles.lstPullOracleFeed → oracles.pythPullLstOracleFeed
   oracles.lstPrice → oracles.lstAlphaPrice
   oracles.lstDecimals → oracles.lstAlphaDecimals
   ```

3. **Confidence Value Pattern**:
   ```typescript
   // Old:
   oracles.confidenceValue // was 0.01
   
   // New:
   import { ORACLE_CONF_INTERVAL } from "./types"; // 0.01
   
   // Usage depends on context:
   // Raw price context (price without decimals):
   setPythPullOraclePrice(oracle, feed, price, decimals, ORACLE_CONF_INTERVAL)
   
   // Native price context (price * 10 ** decimals):
   const nativePrice = price * 10 ** decimals;
   const confidence = nativePrice * ORACLE_CONF_INTERVAL;
   ```

## Approach for Fixing TypeScript Compilation Errors

When fixing merge conflicts and compilation errors, we need to:

1. **Run TypeScript compilation** to identify all errors
   ```bash
   # Also check for compilation errors in the files themselves, not sure how you do this

   # For running tests (use anchor test)
   anchor test --skip-build
   ```

2. **Check git diff** to see what changed in the merge
   ```bash
   # IMPORTANT: First find the actual merge commit hash
   git log --oneline -20 | grep -E "(merge|Merge)"
   # Example output: a06c1c2 Merge remote-tracking branch 'origin/main' into kamino-integration
   # The commit before merge: e9a1f5f
   
   # Then use the actual commit hashes (not HEAD~1 which may be wrong after multiple commits)
   git diff e9a1f5f a06c1c2 -- tests/utils/pyth_mocks.ts
   
   # See the staged changes from the merge
   git diff --cached -- tests/utils/mocks.ts
   
   # Compare with the main branch
   git diff origin/main -- tests/k05_kaminoBankInit.spec.ts
   ```

3. **Look at previous versions** to understand the original intent
   ```bash
   # Show the file before the merge (use actual commit hash)
   git show e9a1f5f:tests/utils/mocks.ts | head -50
   
   # Show the file from main branch
   git show origin/main:tests/utils/mocks.ts | head -50
   
   # Show specific function implementations
   git show e9a1f5f:tests/utils/group-instructions.ts | grep -A10 "marginfiGroupConfigure"
   
   # Find when a function signature changed
   git log -p --grep="createMockAccount" -- tests/utils/
   ```

4. **Apply changes systematically** based on the merge review checklist
   - Update function calls to match new signatures
   - Add missing required fields
   - Update imports and constants

### Example Fixes:

#### createMockAccount signature change
```typescript
// OLD signature (before merge):
createMockAccount(program, space, wallet, bankrunContext?)

// NEW signature (after merge):
createMockAccount(program, space, wallet, keypair?, bankrunContext?)

// Find all uses to fix:
grep -n "createMockAccount" tests/*.ts tests/**/*.ts
```

#### expectFailedTxWithError now requires error code
```typescript
// OLD (before merge):
expectFailedTxWithError(txFn, "InvalidOracleSetup")

// NEW (after merge):
expectFailedTxWithError(txFn, "InvalidOracleSetup", 6003)

// Find the error code:
grep -n "InvalidOracleSetup" programs/marginfi/src/errors.rs
```

#### marginfiGroupConfigure now takes 5 admins
```typescript
// OLD:
marginfiGroupConfigure(ctx, admin, feeAdmin, { paused: false })

// NEW:
marginfiGroupConfigure(ctx, admin, feeAdmin, admin, admin, admin, { paused: false })
//                           ^      ^         ^curveAdmin ^limitAdmin ^emissionsAdmin
```

### Key Changes to Look For:
- Function signature changes (e.g., createMockAccount, expectFailedTxWithError)
- Oracle system changes (Pyth Push migration)
- New required fields in configs (configFlags, oracleMaxConfidence)
- Changed constants and imports
- Error handling changes (numeric codes vs strings)

### Next Steps