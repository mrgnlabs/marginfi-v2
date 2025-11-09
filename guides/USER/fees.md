# Summary

Want to learn about all the various fees p0 users might pay? Read on!

## Program Fees

These fees are charged at the smart contract level.

### Deposit Fees

There are never any fees to deposit into p0.

### Withdraw fees

There are never any fees to withdraw from p0.

### Borrow Fees

Borrowers pay interest to lenders. For most Banks, a small portion of borrow interest paid is a fee that goes to the administrator. A small portion also goes to the Bank's insurance fund.

The program supports borrow origination fees, but as of November 2025, these have always been zero.

### Repay Fees

There are never any fees to repay a debt on p0.

### Liquidation Fee

If an Account is partially liquidated, it can lose up to ten percent of the cash equity value liquidated. For example, let's say an account has $100 in assets and $90 in debt (net worth $10). A liquidator seizes $10 of collateral and repays $9 in debt. The account now has $90 in assets and $81 in debt: net worth declined by 10%.

Accounts with a net worth of less than ten dollars can be fully liquidated: all remaining collateral on the Account will be seized and all debt repaid, closing out the Account.

## Front End Fees

These are fees charged to front end users, you can avoid these by using a different front end or calling the program directly through scripts, cpi, etc.

### Slippage

When swapping asset A for asset B, the value of B you will get is slightly smaller than the value of A you give up. This is called slippage. You can configure your max tolerated slippage for all operations that swap assets on p0 in settings. Remember that different asset pairs have different slippage expectations.

### Swap Fees

When swapping asset A to asset B, for features such as collateral repay, a small fee is charged. You can see the fee in the preview breakdown before approving the tx. Typically this is around 5bps (0.05%).

## Fees From Other Venues

Currently, none of the venues integrated with p0 charge any notable fees.
