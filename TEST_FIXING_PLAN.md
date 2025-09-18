# Test Fixing Plan for Kamino Integration

## Overview
This document outlines a systematic approach to fix all tests after merging `origin/main` into `kamino-integration`. We'll test each suite independently to isolate and fix issues.

## Test Suites
1. **Basic Tests** (01-13): Core marginfi functionality
2. **Emode Tests** (e*): Enhanced mode features
3. **Kamino Tests** (k*): Kamino integration specific
4. **Staking Tests** (s*): Staked collateral features
5. **Pyth Tests** (p*): Pyth oracle functionality
6. **Limits Tests** (m*): Account limits testing
7. **Compound Interest** (z*): Interest accrual tests

## Current Status
- ✅ TypeScript compilation: No errors found
- ✅ Program builds successfully
- ✅ Basic tests (01-13): ALL PASSING
- ✅ Emode tests (e*): ALL PASSING
- ✅ Kamino tests (k*): ALL PASSING
- ✅ Pyth tests (p*): ALL PASSING
- ✅ Limits tests (m*): ALL PASSING
- ✅ Compound interest test (z*): ALL PASSING
- ⚠️  Staking tests (s*): BLOCKED - crash with accounts_db panic when running with other tests

## Test Execution Plan

### Phase 1: Basic Tests (01-13)
```toml
test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/0*.spec.ts tests/1*.spec.ts --exit --require tests/rootHooks.ts"
```
**Expected Issues:**
- Oracle reference changes (see MERGE_REVIEW_CHECKLIST.md)
- Function signature changes (marginfiGroupConfigure, createMockAccount, etc.)
- New admin structure (5 admins instead of 1)

### Phase 2: Emode Tests (e*)
```toml
test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/e*.spec.ts --exit --require tests/rootHooks.ts"
```
**Expected Issues:**
- Oracle confidence calculations
- ComputeBudgetProgram requirements

### Phase 3: Kamino Tests (k*) [CURRENT]
```toml
test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/k*.spec.ts --exit --require tests/rootHooks.ts"
```
**Expected Issues:**
- Bank cache updates needed
- Oracle migration patterns
- Kamino-specific functionality preservation

### Phase 4: Staking Tests (s*)
```toml
test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/s*.spec.ts --exit --require tests/rootHooks.ts"
```
**Known Issues:**
- accounts_db panic with validator 1
- May need to investigate validator setup

### Phase 5: Pyth Tests (p*)
```toml
test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/p*.spec.ts --exit --require tests/rootHooks.ts"
```
**Expected Issues:**
- Pyth Push oracle migration
- Feed ID handling changes

### Phase 6: Limits Tests (m*)
```toml
test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/m*.spec.ts --exit --require tests/rootHooks.ts"
```
**Expected Issues:**
- Account limits with new structure
- Kamino integration limits

### Phase 7: Compound Interest (z*)
```toml
test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/z*.spec.ts --exit --require tests/rootHooks.ts"
```
**Expected Issues:**
- Bank cache system integration
- Interest tracking changes

## Fix Patterns from Merge

### 1. Oracle References
```typescript
// OLD → NEW
oracles.usdcPull → oracles.usdcOracle
oracles.usdcPullOracleFeed → oracles.usdcOracleFeed
oracles.lstPull → oracles.pythPullLst
oracles.lstPrice → oracles.lstAlphaPrice
```

### 2. Confidence Values
```typescript
// OLD: oracles.confidenceValue
// NEW: import { ORACLE_CONF_INTERVAL } from "./types"; // 0.01
```

### 3. Function Signatures
```typescript
// createMockAccount
OLD: (program, space, wallet, bankrunContext?)
NEW: (program, space, wallet, keypair?, bankrunContext?)

// expectFailedTxWithError
OLD: (txFn, "ErrorCode")
NEW: (txFn, "ErrorCode", errorNumber)

// marginfiGroupConfigure
OLD: (admin, feeAdmin, paused)
NEW: (admin, feeAdmin, curveAdmin, limitAdmin, emissionsAdmin, paused)
```

### 4. Bank Configuration
- Add `configFlags: 1` for banks created in 0.1.4+
- Add `oracleMaxConfidence: 0` (use default 10%)
- Update bank cache after operations

## Debugging Strategy

1. **Run individual test file**: When a suite fails, run individual test files
   ```bash
   yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/k01_kaminoInit.spec.ts --exit --require tests/rootHooks.ts
   ```

2. **Check diagnostics**: Use MCP IDE tool for TypeScript errors
   ```
   mcp__ide__getDiagnostics with uri parameter
   ```

3. **Review git diff**: Check what changed in merge
   ```bash
   git diff --cached -- tests/failing_test.spec.ts
   ```

4. **Compare with main**: See how main branch handles it
   ```bash
   git show origin/main:tests/failing_test.spec.ts | grep -A10 "failing_function"
   ```

## Progress Tracking

### Basic Tests (01-13) ✅ ALL PASSING!
- [x] 01_initGroup.spec.ts
- [x] 02_configGroup.spec.ts
- [x] 03_addBank.spec.ts
- [x] 04_configureBank.spec.ts
- [x] 05_setupEmissions.spec.ts
- [x] 06_initUser.spec.ts
- [x] 07_deposit.spec.ts
- [x] 08_borrow.spec.ts
- [x] 09_withdraw_repay.spec.ts
- [x] 10_liquidate.spec.ts
- [x] 11_health_pulse.spec.ts
- [x] 12_transfer_account.spec.ts
- [x] 13_closebank.spec.ts

### Emode Tests ✅ ALL PASSING!
- [x] e01_initGroup.spec.ts
- [x] e02_configEmode.spec.ts
- [x] e03_emodeBorrow.spec.ts
- [x] e04_emodeLiquidation.spec.ts
- [x] e05_gappyPositions.spec.ts

### Kamino Tests ✅ ALL PASSING!
- [x] k01_kaminoInit.spec.ts
- [x] k02_kaminoUser.spec.ts
- [x] k03_kaminoDeposit.spec.ts
- [x] k04_kaminoMrgnUser.spec.ts
- [x] k05_kaminoBankInit.spec.ts - Fixed oracle key mismatch
- [x] k06_MrgnKaminoDeposit.spec.ts
- [x] k07_kaminoWithdraw.spec.ts
- [x] k08_kaminoBorrow.spec.ts
- [x] k09_kaminoWithdrawInterest.spec.ts
- [x] k10_kaminoLiquidate.spec.ts
- [x] k11_DepositPostInterest.spec.ts
- [x] k12_BorrowPostInterest.spec.ts - Fixed ORACLE_CONF_INTERVAL import
- [x] k13_KfarmsHarvestReward.spec.ts
- [x] k14_limitsWithKamino.spec.ts
- [x] k15_kaminoMrgnDeposits.spec.ts

### Staking Tests
- [ ] s01_usersStake.spec.ts
- [ ] s02_addBank.spec.ts
- [ ] s03_deposit.spec.ts
- [ ] s04_borrow.spec.ts
- [ ] s05_solAppreciates.spec.ts
- [ ] s06_propagateSets.spec.ts
- [ ] s07_withdraw_repay.spec.ts
- [ ] s08_liquidate.spec.ts
- [ ] s09_emissions.spec.ts
- [ ] s10_permissionless_claim.spec.ts

### Pyth Tests ✅ ALL PASSING!
- [x] p01_initPythPull.spec.ts
- [x] p02_pythMigrated.spec.ts

### Limits Tests ✅ ALL PASSING!
- [x] m01_accountLimits.spec.ts
- [x] m02_limitsWithEmode.spec.ts

### Compound Interest Test ✅ ALL PASSING!
- [x] z01_compoundInterest.spec.ts

## Summary of Fixes Applied

### Kamino Tests Fixes:
1. **k05_kaminoBankInit.spec.ts**: Fixed oracle key mismatch
   - Changed `oracles.usdcOracleFeed.publicKey` to `oracles.usdcOracle.publicKey`

2. **genericSetups.ts**: Fixed "tasks is not defined" error
   - Replaced `tasks.push(addGenericBank(...))` with `await addGenericBank(...)`

3. **k12_BorrowPostInterest.spec.ts**: Fixed missing import
   - Added `import { CONF_INTERVAL_MULTIPLE, ORACLE_CONF_INTERVAL } from "./utils/types";`
   - Replaced `oracles.confidenceValue` with `ORACLE_CONF_INTERVAL`

### Staking Tests Issue:

**When run independently (s* only):**
- 51 passing, 7 failing
- Tests run to completion
- Main error: "Account does not exist or has no data AGde6MvBGqaXswau99YYghUrFHfDM7RmWXb4vb8bEAjQ"
- This is the marginfiGroup account that would be created by basic tests
- The failing tests are in s09_emissions.spec.ts which expects the marginfiGroup to exist

**When run with all tests together:**
- Tests crash with bankrun panic: "calculate_accounts_hash_with_verify mismatch"
- Crash occurs after "(user 1/2/3) Stakes and delegates too" test in s01_usersStake.spec.ts
- This is a deeper infrastructure issue where bankrun's account verification fails
- The crash prevents any subsequent tests from running

**Root cause:**
- Staking tests depend on `marginfiGroup` from basic tests
- `marginfiGroup.publicKey` is not in `copyKeys` array in `rootHooks.ts`
- When run alone: tests fail gracefully with missing account error
- When run together: bankrun infrastructure crashes during validator staking operations

## Remaining Work
1. Investigate and fix staking test bankrun crash
2. Consider adding `marginfiGroup.publicKey` to `copyKeys` for independent staking test runs
3. Run all tests together once staking issue is resolved