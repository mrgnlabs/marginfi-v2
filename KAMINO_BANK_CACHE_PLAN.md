# Kamino Bank Cache Update Plan

## Understanding the Bank Cache System

### What is the Bank Cache?
The bank cache (introduced in the merge) stores computed values that are expensive to calculate:
- Interest rates (base rate, lending rate, borrowing rate)
- Interest accumulation tracking
- Last update timestamp

### When to Update Bank Cache
The cache needs updating when:
1. **Bank shares change** - This affects utilization ratio, which affects interest rates
2. **After operations that modify bank state**:
   - Deposits (increase asset shares)
   - Withdraws (decrease asset shares)
   - Borrows (increase liability shares)
   - Repays (decrease liability shares)

### Current Issue with Kamino Operations

Looking at the Kamino withdraw code more carefully:

1. **First block (lines 68-94)**: 
   - Loads bank as mutable
   - Calls `bank_account.withdraw()` which modifies bank shares
   - But then the bank goes out of scope
   - Returns only the collateral amount

2. **After the CPI call**:
   - Bank is loaded again as immutable (line 130)
   - Only used for reading mint info for the event

## The Problem

The bank cache update needs to happen **after** the bank shares are modified but **while we still have a mutable reference to the bank**. Currently:

1. The bank is modified in the first scope
2. The mutable reference is dropped
3. We do the Kamino CPI
4. We load the bank again as immutable

This means we can't update the cache after the withdraw because we don't have a mutable bank reference anymore.

## Proposed Solutions

### Option 1: Update Cache in First Block (Current Approach)
```rust
let collateral_amount = {
    let mut bank = ctx.accounts.bank.load_mut()?;
    let group = &ctx.accounts.group.load()?;
    
    // ... withdraw logic ...
    
    // Update cache before dropping mutable reference
    bank.update_bank_cache(group)?;
    
    amount
};
```

**Pros**: Simple, follows the pattern of updating after state change
**Cons**: Cache is updated before the Kamino CPI (which could fail)

### Option 2: Restructure to Keep Bank Mutable
```rust
// Load bank and group once
let mut bank = ctx.accounts.bank.load_mut()?;
let mut marginfi_account = ctx.accounts.marginfi_account.load_mut()?;
let group = &ctx.accounts.group.load()?;

// Do the withdraw
let collateral_amount = /* ... */;

// Do the Kamino CPI
ctx.accounts.cpi_kamino_withdraw(collateral_amount)?;

// Update cache after successful CPI
bank.update_bank_cache(group)?;

// Emit event using bank data
emit!(/* ... */);
```

**Pros**: Cache updated after successful CPI
**Cons**: Requires larger refactoring

### Option 3: No Cache Update for Kamino Banks
Since Kamino banks don't accrue interest, maybe they don't need cache updates?

**Questions**:
- Do Kamino banks use interest rates at all?
- Is the cache used for anything besides interest calculation?
- Will other parts of the system expect the cache to be updated?

## Recommendation

I think **Option 1** is the safest approach because:
1. It ensures the cache reflects the actual bank state
2. Even if the CPI fails, the bank state was already modified by the withdraw
3. It's consistent with how regular operations work

The key insight is that `bank_account.withdraw()` already modifies the bank's shares, so the cache should be updated to reflect that change, regardless of whether the subsequent Kamino CPI succeeds.