# Merge Review Checklist

Review of merge from `origin/main` into `kamino-integration` branch.

## Context

This merge is being performed to update the kamino-integration branch with the latest changes from main while preserving all Kamino-specific functionality. The command `git merge -X theirs origin/main --no-commit` was used, which prioritizes incoming changes from main when there are conflicts. This means we need to carefully review each file to ensure no Kamino integration code was inadvertently removed.

Current git state: The merge is staged but not committed, allowing us to review and fix any issues before finalizing.

## Review Procedure

For each file, follow these steps:

### Step 1: Read the FULL diff
```bash
# First, check the size of the diff
git diff --cached -- <file_path> | wc -l

# If under 200 lines, read it all at once:
git diff --cached -- <file_path>

# If over 200 lines, read in chunks:
git diff --cached -- <file_path> | head -200
git diff --cached -- <file_path> | tail -n +200 | head -200
# Continue until you've read the entire diff
```
READ THE ENTIRE DIFF IN CHUNKS - never use grep as a shortcut

### Step 2: Understand what's being changed
Look for:
- **Removed functionality** that might be Kamino-related (even without the word "kamino")
- **Oracle setups** (KaminoPythPush, KaminoSwitchboardPull)
- **Asset tags** (ASSET_TAG_KAMINO)
- **Bank validation logic** specific to Kamino banks
- **Special handling** for Kamino reserves/obligations
- **Imports** from kamino_mocks or other Kamino crates
- **Error handling** for Kamino operations
- **Any logic that checks** asset tags, oracle types, or bank types

### Step 3: Pattern Recognition
Based on what we've seen so far, Kamino integration includes:
- Error codes 6200-6211
- ASSET_TAG_KAMINO = 3
- kamino_reserve and kamino_obligation fields in Bank
- Kamino instructions in lib.rs
- MinimalReserve import for validation
- Special oracle setups for Kamino

### Step 4: If functionality is removed
1. Identify what was removed and why it matters
2. **FIX IT IMMEDIATELY** - Add the code back
3. Mark as **FIXED** after restoration
4. Update pattern list if we find new types of Kamino integration

### Important Notes
- Focus on ONE FILE at a time
- If Kamino code is removed, add it back to THAT FILE only
- Don't chase dependencies across files
- We'll handle integration issues when we get to those specific files

### Risk Categories
- **SAFE**: No Kamino code OR changes preserved correctly
- **REVIEW NEEDED**: Contains Kamino code that needs manual verification
- **ACTION REQUIRED**: Missing Kamino code or broken integration

### Notes
- Update this procedure if we find better approaches
- Add specific patterns to search for as we discover them
- DO NOT modify this Notes section with summaries or status updates - keep file-specific changes next to each file and high level descriptions of thing to be aware of when going through and making sure the kamino PR still works given all of these changes

## Important Architecture Changes to Adapt in Kamino PR

### 1. BankCache System (NEW)
- **File**: `programs/marginfi/src/state/bank_cache.rs`
- **Impact**: All deposit/withdraw/borrow/repay operations now call `bank.update_bank_cache(group)?`
- **TODO**: Kamino operations should also update bank cache after state changes

### 2. Oracle Migration (Pyth Push)
- **Change**: Legacy Pyth oracle replaced with Pyth Push oracle
- **Files affected**: All tests and price.rs
- **Key changes**:
  - Import: `pyth_solana_receiver_sdk` instead of `pyth-sdk-solana`
  - Oracle setup version: 1 → 3 (`ORACLE_SETUP_PYTH_LEGACY` → `ORACLE_SETUP_PYTH_PUSH`)
  - Price structure: `PriceAccount` → `PriceUpdateV2`
  - New migration flag: `is_pyth_push_migrated()`
  - **Oracle references simplified**: No longer need separate oracle + feed addresses, just one oracle key
- **TODO**: Ensure Kamino oracle validation works with new Pyth format
- **TODO**: Update Kamino tests to use single oracle reference instead of oracle + feed

### 3. Account Transfer Redesign
- **Old**: `transfer_account_authority` (just changed authority)
- **New**: `transfer_account` (creates new account, disables old)
- **Key fields**: 
  - `migrated_from` tracks old account
  - Old account marked with `ACCOUNT_DISABLED` flag
  - Charges `ACCOUNT_TRANSFER_FEE` (5M lamports)
- **TODO**: Consider if Kamino accounts need special handling during transfer

### 4. Group Admin Structure
- **Old**: Single admin for all operations
- **New**: Separate admins: `admin`, `emode_admin`, `curve_admin`, `limit_admin`, `emissions_admin`
- **Impact**: Functions like `marginfiGroupConfigure` now take 5 admin parameters instead of 1
- **TODO**: Determine which admin controls Kamino operations

### 5. Interest Tracking Enhancement
- **New fields in BankCache**:
  - `base_rate`, `lending_rate`, `borrowing_rate`
  - `interest_accumulated_for` (seconds)
  - `accumulated_since_last_update` (token amount)
- **TODO**: Kamino yield calculations may benefit from these cached values

### 6. Oracle Confidence Checks
- **New param**: `oracle_max_confidence` in bank config (u32, 0 = use default 10%)
- **New error**: `OracleMaxConfidenceExceeded`
- **TODO**: Kamino price feeds should respect confidence limits

### 7. Bank Configuration Split
- **New functions**: 
  - `configure_bank_interest_only` (controlled by `curve_admin`)
  - `configure_bank_limits_only` (controlled by `limit_admin`)
- **Old function**: `configure_bank` still exists for full config
- **TODO**: Determine which admin should control Kamino bank configs

### 8. Default Bank Flags
- **New**: Banks created with `CLOSE_ENABLED_FLAG` by default
- **Impact**: Banks can be closed when no positions remain
- **TODO**: Consider if Kamino banks should have this flag

### 9. Bank Position Tracking
- **New fields**: `lending_position_count`, `borrowing_position_count`
- **Impact**: Banks track active positions, updated on deposit/borrow/withdraw/repay
- **TODO**: Kamino operations should update these counts appropriately

### 10. Balance Active Field Type Change
- **Old**: `active: bool` (false)
- **New**: `active: u8` (0)
- **Impact**: Type change in marginfi account balance slots

## Key Constants and Code Preserved
- ✓ `ASSET_TAG_KAMINO = 3` - Still present in types.ts
- ✓ Error codes 6200-6211 - Re-added to errors.rs
- ✓ Kamino instructions - Re-added to lib.rs
- ✓ `kamino_reserve`/`kamino_obligation` fields - Re-added to Bank struct
- ✓ `MinimalReserve` import - Re-added to price.rs

## Test Updates Needed for Kamino Tests

### Critical Function Signature Changes
1. **createMockAccount** - Changed parameters:
   ```typescript
   // Old: (program, space, wallet, bankrunContext?)
   // New: (program, space, wallet, keypair?, bankrunContext?)
   ```
   
2. **expectFailedTxWithError** - Now requires error number:
   ```typescript
   // Old: expectFailedTxWithError(txFn, "ErrorCode")
   // New: expectFailedTxWithError(txFn, "ErrorCode", errorNumber)
   ```

3. **marginfiGroupConfigure** - Now takes 5 admins:
   ```typescript
   // Old: marginfiGroupConfigure(admin, feeAdmin, paused)
   // New: marginfiGroupConfigure(admin, feeAdmin, curveAdmin, limitAdmin, emissionsAdmin, paused)
   ```

### Oracle System Changes
1. **Oracle Setup Type**: `ORACLE_SETUP_PYTH_LEGACY` (1) → `ORACLE_SETUP_PYTH_PUSH` (3)
2. **Oracle References**: No longer separate oracle + feed, just single oracle key
3. **Removed Functions**:
   - `updatePriceAccount` - no longer exists
   - Legacy Pyth oracle creation functions removed
4. **Oracle Confidence**: 
   - Removed `confidenceValue` parameter from oracle setup
   - Now uses `ORACLE_CONF_INTERVAL` constant (0.01)
5. **Oracle Field Renames**:
   - `lstPrice` → `lstAlphaPrice` in Oracles type
   - `lstDecimals` → `lstAlphaDecimals`
6. **setupPythOracles Function**:
   - No longer takes confidence parameter
   - Returns simplified oracle structure without separate feed references
   
### Type/Field Changes
1. **Balance Active Field**: `active: bool` → `active: u8` (0/1)
2. **Bank Config New Fields**:
   - `oracleMaxConfidence: number`
   - `configFlags: number`
3. **Bank New Fields**:
   - `lending_position_count`
   - `borrowing_position_count`
4. **Health Cache Version**: `HEALTH_CACHE_PROGRAM_VERSION_0_1_3` → `HEALTH_CACHE_PROGRAM_VERSION_0_1_4`

### Removed/Changed Constants
1. **Removed**:
   - `ORACLE_SETUP_PYTH_LEGACY`
   - `ONE_WEEK_IN_SECONDS`
   - Oracle `confidenceValue` field
2. **Added**:
   - `CLOSE_ENABLED_FLAG = 16`
   - `ACCOUNT_DISABLED = 1`
   - `ACCOUNT_TRANSFER_FEE = 5_000_000`
   - `ORACLE_CONF_INTERVAL = 0.01`

### Import Changes
```typescript
// Old: import { ... } from "@pythnetwork/client"
// Old: import { parsePriceData } from "@pythnetwork/client"
// New: Uses pyth_solana_receiver_sdk instead
```

### Bank Setup Changes
1. **addBank/addBankWithSeed** - New required field:
   - Must include `configFlags` in bank config
2. **Bank Flags**:
   - All new banks automatically get `CLOSE_ENABLED_FLAG`
   - Kamino banks using `ASSET_TAG_KAMINO` will need this flag
3. **Bank Cache Updates**:
   - All deposit/withdraw/borrow/repay must call `bank.update_bank_cache(group)?`

### Error Handling Changes
1. **Error Codes**: Many tests now use numeric error codes instead of strings
2. **assertBankrunTxFailed**: Now often uses numeric codes (e.g., 6052 instead of "0x17a4")

### Test Infrastructure
1. **Oracles Type** - Changed fields for consistency:
   ```typescript
   // These Kamino tests likely need updates:
   // - k05_kaminoBankInit.spec.ts (uses ASSET_TAG_KAMINO)
   // - Tests using kamino-utils.ts functions
   ```
2. **Mock Account Creation**:
   - Any Kamino test using `createMockAccount` needs the new signature
3. **Group Configuration**:
   - Any test creating groups needs to handle 5 admin parameters

## Modified Files

### Configuration Files
- [x] `.vscode/tasks.json` - **SAFE** - No Kamino-specific changes, standard VS Code tasks for tests 
- [x] `Anchor.toml` - **SAFE** - Only adds pre_migration_bank1 test fixtures, no Kamino code changes 
- [x] `Cargo.lock` - **SAFE** - Standard dependency updates, kamino-mocks reference already existed 
- [x] `Cargo.toml` - **SAFE** - Adds marginfi-cli to workspace, downgrades some SPL versions, comments out pythnet-sdk 

### Client Files
- [x] `clients/rust/marginfi-cli/Cargo.toml` - **SAFE** - Version bump, removes liquidity-incentive-program and pyth-sdk-solana deps, uses workspace deps 
- [x] `clients/rust/marginfi-cli/src/entrypoint.rs` - **SAFE** - Removes LIP feature, adds new admin fields (curve_admin, limit_admin, emissions_admin), adds oracle_max_confidence 
- [x] `clients/rust/marginfi-cli/src/macros.rs` - **SAFE** - Only changes ToString to Display trait 
- [x] `clients/rust/marginfi-cli/src/processor/group.rs` - **SAFE** - Only updates Solana import paths 
- [x] `clients/rust/marginfi-cli/src/processor/mod.rs` - **SAFE** - Updates oracle setup, adds new admin fields, no Kamino code 
- [x] `clients/rust/marginfi-cli/src/processor/oracle.rs` - **SAFE** - Minor import updates and method rename 
- [x] `clients/rust/marginfi-cli/src/utils.rs` - **SAFE** - Updates Pyth constant name, adds migration check 

### Documentation
- [x] `guides/DEPLOY_GUIDE.md` - **SAFE** - Updates deployment instructions, adds IDL and verification info
- [x] `guides/GETTING_STARTED_DEV.md` - **SAFE** - Adds CPU benchmark, troubleshooting for I80F48 and metadata issues 

### Observability
- [x] `observability/indexer/Cargo.toml` - **SAFE** - Removes pyth-sdk-solana dependency
- [x] `observability/indexer/src/common.rs` - **SAFE** - Removes legacy Pyth oracle code
- [x] `observability/indexer/src/utils/metrics.rs` - **SAFE** - Removes legacy Pyth/Switchboard V2 handling
- [x] `observability/indexer/src/utils/snapshot.rs` - **SAFE** - Oracle migration cleanup, no Kamino code 

### Core Program Files
- [x] `programs/marginfi/Cargo.toml` - **SAFE** - Version bump to 0.1.4, removes pyth-sdk-solana and pythnet-sdk deps 
- [x] `programs/marginfi/src/constants.rs` - **FIXED** - Re-added ASSET_TAG_KAMINO constant that was removed 
- [x] **`programs/marginfi/src/errors.rs`** (POTENTIAL CONFLICT - was UU) - **FIXED** - Re-added all Kamino error codes (6200-6211) that were removed in merge 
- [x] `programs/marginfi/src/events.rs` - **SAFE** - Renames transfer authority event, adds old_account field 
- [x] **`programs/marginfi/src/lib.rs`** (POTENTIAL CONFLICT - was UU) - **FIXED** - Re-added all Kamino instructions that were removed 

### Instructions - MarginFi Account
- [x] `programs/marginfi/src/instructions/marginfi_account/borrow.rs` - **SAFE** - Adds bank cache update 
- [x] `programs/marginfi/src/instructions/marginfi_account/close_balance.rs` - **SAFE** - Minor refactoring 
- [x] `programs/marginfi/src/instructions/marginfi_account/deposit.rs` - **SAFE** - Adds bank cache update 
- [x] `programs/marginfi/src/instructions/marginfi_account/liquidate.rs` - **SAFE** - Adds bank cache updates 
- [x] `programs/marginfi/src/instructions/marginfi_account/mod.rs` - **SAFE** - Renames transfer_authority to transfer_account 
- [x] `programs/marginfi/src/instructions/marginfi_account/repay.rs` - **SAFE** - Adds bank cache update 
- [x] **`programs/marginfi/src/instructions/marginfi_account/transfer_account.rs`** (NEW) - **SAFE** - New file for account transfers, no Kamino code 
- [x] **`programs/marginfi/src/instructions/marginfi_account/transfer_authority.rs`** (DELETED) - **SAFE** - No Kamino code in deleted file 
- [x] `programs/marginfi/src/instructions/marginfi_account/withdraw.rs` - **SAFE** - Refactoring and bank cache updates 

### Instructions - MarginFi Group
- [x] `programs/marginfi/src/instructions/marginfi_group/accrue_bank_interest.rs` - **SAFE** 
- [x] `programs/marginfi/src/instructions/marginfi_group/add_pool_common.rs` - **SAFE** 
- [x] `programs/marginfi/src/instructions/marginfi_group/add_pool_permissionless.rs` - **SAFE** 
- [x] **`programs/marginfi/src/instructions/marginfi_group/close_bank.rs`** (NEW) - **SAFE** - New bank closure functionality 
- [x] `programs/marginfi/src/instructions/marginfi_group/config_bank_emode.rs` - **SAFE** 
- [x] `programs/marginfi/src/instructions/marginfi_group/config_bank_oracle.rs` - **SAFE** 
- [x] `programs/marginfi/src/instructions/marginfi_group/configure.rs` - **SAFE** 
- [x] `programs/marginfi/src/instructions/marginfi_group/configure_bank.rs` - **SAFE** 
- [x] **`programs/marginfi/src/instructions/marginfi_group/configure_bank_lite.rs`** (NEW) - **SAFE** - Lite bank configuration 
- [x] `programs/marginfi/src/instructions/marginfi_group/edit_stake_settings.rs` - **SAFE** 
- [x] `programs/marginfi/src/instructions/marginfi_group/handle_bankruptcy.rs` - **SAFE** 
- [x] **`programs/marginfi/src/instructions/marginfi_group/migrate_pyth_push_oracle.rs`** (NEW) - **SAFE** - Oracle migration 
- [x] `programs/marginfi/src/instructions/marginfi_group/mod.rs` - **SAFE** 
- [x] `programs/marginfi/src/instructions/marginfi_group/propagate_staked_settings.rs` - **SAFE** 

### State Files
- [x] **`programs/marginfi/src/state/bank_cache.rs`** (NEW) - **SAFE** - New cache implementation, no Kamino code 
- [x] `programs/marginfi/src/state/emode.rs` - **SAFE** 
- [x] `programs/marginfi/src/state/health_cache.rs` - **SAFE** 
- [x] `programs/marginfi/src/state/marginfi_account.rs` - **SAFE** 
- [x] **`programs/marginfi/src/state/marginfi_group.rs`** (CONFLICT) - **FIXED** - Re-added Kamino fields (kamino_reserve, kamino_obligation) at end of Bank struct 
- [x] `programs/marginfi/src/state/mod.rs` - **SAFE** 
- [x] **`programs/marginfi/src/state/price.rs`** (CONFLICT) - **FIXED** - Re-added MinimalReserve import (all Kamino oracle handling code intact) 
- [x] `programs/marginfi/src/state/staked_settings.rs` - **SAFE** 

### Fuzz Testing
- [x] `programs/marginfi/fuzz/Cargo.lock` - **SAFE** - Dependency updates 
- [x] `programs/marginfi/fuzz/Cargo.toml` - **SAFE** - Version updates 
- [x] `programs/marginfi/fuzz/src/account_state.rs` - **SAFE** - Oracle migration changes only 
- [x] `programs/marginfi/fuzz/src/bank_accounts.rs` - **SAFE** - Oracle migration changes only 
- [x] `programs/marginfi/fuzz/src/lib.rs` - **SAFE** - Oracle migration, added new error types 

### Tests - Admin Actions
- [x] `programs/marginfi/tests/admin_actions/account_transfer.rs` - **SAFE** - Account transfer mechanism updated (not Kamino-related) 
- [x] `programs/marginfi/tests/admin_actions/bankruptcy_auth.rs` - **SAFE** - Admin params update only 
- [x] `programs/marginfi/tests/admin_actions/interest_accrual.rs` - **SAFE** - New BankCache system added (beneficial for Kamino) 
- [x] **`programs/marginfi/tests/admin_actions/setup_bank.rs`** (CONFLICT) - **SAFE** - Tests updated for new admin structure and bank cache 
- [x] `programs/marginfi/tests/admin_actions/withdraw_fees.rs` - **SAFE** - Admin params update only 

### Tests - Misc
- [x] `programs/marginfi/tests/misc/bank_ignore_stale_isolated_banks.rs` - **SAFE** - Test name typo fix only 
- [x] `programs/marginfi/tests/misc/real_oracle_data.rs` - **SAFE** - Removed legacy Pyth oracle test, kept Pyth Push version 
- [x] **`programs/marginfi/tests/misc/regression.rs`** (CONFLICT) - **SAFE** - Updated for struct padding changes and new fields 
- [x] `programs/marginfi/tests/misc/risk_engine_flexible_oracle_checks.rs` - **SAFE** - Better error handling for stale oracles 

### Tests - User Actions
- [x] `programs/marginfi/tests/user_actions/borrow.rs` - **SAFE** - Price test value adjustment only 
- [x] `programs/marginfi/tests/user_actions/mod.rs` - **SAFE** - Removed unused imports and account flags test 

### Test Utils
- [x] `test-utils/Cargo.toml` - **SAFE** - Oracle dependency updates 
- [x] **`test-utils/data/pyth_legacy_sol_price.bin`** (DELETED) - **SAFE** - Part of oracle migration 
- [x] **`test-utils/data/pyth_legacy_usdc_price.bin`** (DELETED) - **SAFE** - Part of oracle migration 
- [x] **`test-utils/data/pyth_push_usdc_price.bin`** (NEW) - **SAFE** - Replacement for legacy oracle data 
- [x] `test-utils/src/bank.rs` - **SAFE** - Added oracle confidence param, updated admin fields 
- [x] `test-utils/src/marginfi_account.rs` - **SAFE** - Updated for new transfer mechanism and oracle handling 
- [x] `test-utils/src/marginfi_group.rs` - **SAFE** - Added new admin roles and bank config functions 
- [x] `test-utils/src/test.rs` - **SAFE** - Oracle migration to Pyth Push format, no Kamino changes 
- [x] `test-utils/src/utils.rs` - **SAFE** - Removed legacy Pyth oracle creation functions 

### Typescript Tests
- [x] `tests/01_initGroup.spec.ts` - **SAFE** - Formatting changes only 
- [x] `tests/02_configGroup.spec.ts` - **SAFE** - Updated for multiple admin roles structure
- [x] `tests/03_addBank.spec.ts` - **SAFE** - Oracle migration to Pyth Push, adds oracle_max_confidence
- [x] `tests/04_configureBank.spec.ts` - **SAFE** - Oracle type updates, new CLOSE_ENABLED_FLAG
- [x] `tests/05_setupEmissions.spec.ts` - **SAFE** - Adds emissions_admin role support
- [x] `tests/06_initUser.spec.ts` - **SAFE** - Minor change: active field from boolean to numeric
- [x] `tests/07_deposit.spec.ts` - **SAFE** - Adds lendingPositionCount tracking
- [x] `tests/08_borrow.spec.ts` - **SAFE** - Removes oracle refresh test, adds borrowingPositionCount
- [x] `tests/09_withdraw_repay.spec.ts` - **SAFE** - Removes oracle refresh test, standard updates
- [x] `tests/10_liquidate.spec.ts` - **SAFE** - Removes oracle refresh test, adds error codes
- [x] `tests/11_health_pulse.spec.ts` - **SAFE** - Updates for new health cache version and oracle confidence
- [x] **`tests/12_transfer_account.spec.ts`** (NEW) - **SAFE** - New account transfer mechanism test
- [x] **`tests/13_closebank.spec.ts`** (NEW) - **SAFE** - New bank closing functionality test 
- [x] **`tests/e01_initGroup.spec.ts`** (CONFLICT) - **SAFE** - Oracle setup simplification, no Kamino code removed
- [x] `tests/e03_emodeBorrow.spec.ts` - **SAFE** - Updates for new oracle confidence calculations 
- [x] **`tests/e04_emodeLiquidation.spec.ts`** (CONFLICT) - **SAFE** - Adds ComputeBudgetProgram, adjusts emode rates 
- [x] `tests/e05_gappyPositions.spec.ts` - **SAFE** - Removes unused imports and constants 
- [x] **`tests/fixtures/pre_migration_bank1.json`** (NEW) - **SAFE** - Test fixture data for oracle migration
- [x] **`tests/fixtures/pre_migration_bank1_vault.json`** (NEW) - **SAFE** - Test fixture data for oracle migration 
- [x] **`tests/genericSetups.ts`** (CONFLICT) - **SAFE** - Oracle setup simplification (removed separate feed oracle) 
- [x] `tests/m01_accountLimits.spec.ts` - **SAFE** - Removes unused imports 
- [x] `tests/m02_limitsWithEmode.spec.ts` - **SAFE** - Standard test updates
- [x] **`tests/p01_initPythPull.spec.ts`** (CONFLICT) - **SAFE** - Updates for Pyth oracle system, uses fixed group seed 
- [x] **`tests/p02_pythMigrated.spec.ts`** (NEW) - **SAFE** - New test for Pyth oracle migration functionality 
- [x] `tests/rootHooks.ts` - **SAFE** - Adds pre-migration bank constants, removes oracle confidence parameter 
- [x] `tests/s01_usersStake.spec.ts` - **SAFE** - Adds support for multiple validators (validator 1) 
- [x] `tests/s02_addBank.spec.ts` - **SAFE** - Oracle migration to Pyth Push, updates error codes 
- [x] `tests/s03_deposit.spec.ts` - **SAFE** - Standard staking test updates
- [x] `tests/s05_solAppreciates.spec.ts` - **SAFE** - Standard staking test updates
- [x] `tests/s06_propagateSets.spec.ts` - **SAFE** - Standard staking test updates
- [x] `tests/s08_liquidate.spec.ts` - **SAFE** - Standard staking test updates
- [x] **`tests/s09_emissions.spec.ts`** (CONFLICT) - **SAFE** - Standard emissions test updates 
- [x] `tests/utils/genericTests.ts` - **SAFE** - Adds error number support to expectFailedTxWithError 
- [x] `tests/utils/group-instructions.ts` - **SAFE** - Updated for new bank config fields (oracleMaxConfidence, configFlags) 
- [x] **`tests/utils/mocks.ts`** (CONFLICT) - **FIXED** - Re-added KAMINO_METADATA and KAMINO_OBLIGATION constants 
- [x] **`tests/utils/pyth-pull-mocks.ts`** (CONFLICT) - **SAFE** - Updates function signatures for new createMockAccount pattern 
- [x] **`tests/utils/pyth_mocks.ts`** (CONFLICT) - **SAFE** - Removed legacy Pyth oracle code, now only uses Pyth Pull oracles 
- [x] **`tests/utils/stakeCollatizer/pdas.ts`** (DELETED) - **SAFE** - Stake collateralizer utility file, not Kamino-related 
- [x] **`tests/utils/types.ts`** (CONFLICT) - **SAFE** - ASSET_TAG_KAMINO preserved, oracle system updates
- [x] `tests/utils/user-instructions.ts` - **SAFE** - Adds new transferAccountAuthorityIx and migratePythArgs functions 
- [x] **`tests/z01_compoundInterest.spec.ts`** (CONFLICT) - **SAFE** - Fixes typo (arrueInterest → accrueInterest), adds cache testing 

### Tools
- [x] `tools/alerting/Cargo.toml` - **SAFE** - Removes old pyth dependency
- [x] `tools/alerting/src/main.rs` - **SAFE** - Removes deprecated switchboard v2 oracle checks

### Type Crate
- [x] `type-crate/Cargo.lock` - **SAFE** - Dependency updates
- [x] `type-crate/Cargo.toml` - **SAFE** - Version and dependency updates
- [x] `type-crate/src/constants.rs` - **SAFE** - Adds new flag constants (CLOSE_ENABLED_FLAG, PYTH_PUSH_MIGRATED_FLAG)
- [x] `type-crate/src/types/bank.rs` - **FIXED** - Re-added kamino_reserve and kamino_obligation fields
- [x] **`type-crate/src/types/bank_cache.rs`** (NEW) - **SAFE** - New BankCache type
- [x] `type-crate/src/types/emode.rs` - **SAFE** - No changes
- [x] `type-crate/src/types/group.rs` - **SAFE** - Adds new admin fields
- [x] `type-crate/src/types/health_cache.rs` - **SAFE** - Version update
- [x] `type-crate/src/types/mod.rs` - **SAFE** - Adds BankCache export
- [x] `type-crate/src/types/user_account.rs` - **SAFE** - Minor type updates 

### Other
- [x] **`MERGE_CONFLICT_ANALYSIS.md`** (UNTRACKED) - **N/A** - Our own analysis file 

## Summary

Total files: 105
- Modified: 86
- New files: 12
- Deleted files: 4
- Files with conflicts: 17

## Notes

Key areas to review carefully:
1. Files marked with **(CONFLICT)** had merge conflicts
2. Files marked with **(NEW)** are new additions from main
3. Files marked with **(DELETED)** were removed in main
4. Pay special attention to the Kamino integration related changes that might have been overwritten