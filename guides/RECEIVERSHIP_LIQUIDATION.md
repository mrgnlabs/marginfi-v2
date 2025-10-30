## Summary

Receivership liquidation is what we call the `start_liquidation` and `end_liquidation` instructions.
Classic liquidation uses `lending_account_liquidate` instead.

In this approach, the liquidator takes control of an unhealthy account, enabling them to withdraw
and repay as if they are the user. At the end of liquidation we simply validate that they actually
improved account health.

Within the tx, the liquidator can withdraw, perform e.g. a jup swap, and repay the debt, keeping
whatever is leftover for themselves. This enables to the liquidator to operate without holding any
funds or creating a mrgn account at all.

We use transaction introspection to ensure that every liquidate_start has a corresponding
liquidate_end.

The process generally looks like this:

```
* ComputeBudgetProgram (optional)
* Liquidate Start Instruction
* Withdraw A
* Swap A to B via jup or similar
* Repay B
* Liquidate End Instruction
```

## Why Use This Instead of Classic Liquidation?

Classic liquidation requires the liquidator themselves to have an account, and maintain collateral
in it. This approach requires no account and no collateral: you can start liquidating with zero
dollars (as long as you have SOL to pay gas fees).

Maximum profits are higher with receivership liquidation, up to 10% as of November 2025, where
classic only nets a 2.5% profit.

No exposure to slippage or illiquid markets if swapping within the same tx: if there is no
profitable route between A and B, simply abort the entire tx.

## Notable Rules:

- ComputeBudgetProgram is the only kind of top-level instruction that can come before Start.

- There must be exactly one liquidate Start, and it must come first in the tx (other than Compute
  Budget)

- There must be exactly one liquidate End, and it must come last in the tx. No other instruction can
  come after End.

- The only other mrgn instructions that can be called are Withdraw and Repay.

- Account health must improve between Start and End. All health checks are ignored during withdraw
  and repay: only the final health matters. This means you can withdraw BEFORE repaying.

- The liquidator cannot profit more than percentage configured by the global fee admin (default
  10%), see `FeeState.liquidation_max_fee` for the current max profit.

- The liquidator pays a nominal fee in lamports to use this service. This is automatically billed to
  the global fee wallet at End. See `FeeState.liquidation_flat_sol_fee` to see the fee in lamps.

- The first liquidator to use this service for any account first inits a "liquidation record", and
  pays a nominal amount of rent to create that record (records 1:1 for that account). This record
  stores historical information about the last few liquidation events on that account. Currently, the
  rent for this account cannot be recovered.

- The liquidator can freely fully withdraw or full repay positions, but cannot close them. A
  position withdrawn or repaid in full therefore still requires passing the oracle for that
  position at End.

- The liquidator can withdraw/repay from as many positions as they'd like, but cannot open new
  positions

- Isolated positions, positions with an asset weight of zero, and positions with a price of 0 cannot
  be withdrawn during liquidation.
