# Summary

Want to learn about all the various fees p0 users might pay? Read on!

## Program Fees

These fees are charged at the smart contract level.

### Deposit Fees

There are never any fees to deposit into p0.

### Withdraw fees

There are never any fees to withdraw from p0.

### Borrow Fees

Borrowers pay interest to lenders. For most Banks, a small portion of borrow interest paid is a fee
that goes to the administrator. A small portion also goes to the Bank's insurance fund.

The program supports borrow origination fees, but as of November 2025, these have always been zero.

### Repay Fees

There are never any fees to repay a debt on p0.

### Flashloan Fees

There are never any fees to perform a flashloan on p0.

### Liquidation Fee

If an Account is partially liquidated, it can lose up to ten percent of the cash equity value
liquidated. For example, let's say an account has $100 in assets and $90 in debt (net worth $10). A
liquidator seizes $10 of collateral and repays $9 in debt. The account now has $90 in assets and $81
in debt: net worth declined by 10%.

Accounts with a net worth of less than ten dollars can be fully liquidated: all remaining collateral
on the Account will be seized and all debt repaid, closing out the Account.

## Front End Fees

These are fees charged to front end users, you can avoid these by using a different front end or
calling the program directly through scripts, cpi, etc.

### Slippage

When swapping asset A for asset B, the value of B you will get is slightly smaller than the value of
A you give up. This is called slippage. You can configure your max tolerated slippage for all
operations that swap assets on p0 in settings. Remember that different asset pairs have different
slippage expectations.

### Swap Fees

When swapping asset A to asset B, for features such as collateral repay, a small fee is charged. You
can see the fee in the preview breakdown before approving the tx. Typically this is around 5bps
(0.05%).

### Looping Fees

A "loop" is when a user lends A, borrows B, swaps B -> A, and repeats. Doing this infinite times is
the "max leverage" you can obtain by looping. While there is no fee to loop, since a swap is
involved, there is a swap fee (and slippage) incurred when opening any loop. For example

```
User has $10 in Asset A
Asset Weight of A is 90%, Liability Weight of B is 100%

* User deposits $10 in A (worth $9 in collateral), borrows $9 of B
* User swaps B -> A, incurring slippage + fees, getting ~$9 in A
* User deposits $9 in A and repeats
```

We see that the more times you loop, the more swap fees will be incurred. Note that the theoretical
max leverage of a pair A, B, given Collateral Weight of A (CW) and Liability Weight of B (LW), is:

```
L = 1 / (1 - CW/LW)
```

In our above example, the max leverage is 10x.

Under the hood, a loop is actually built in one step. Let's say in the above example, the user wants
7x leverage:
```
Start a flashloan
Borrow ~ $60 of B
Swap B -> A, netting ~$60 after fees
Deposit A (initial $10 + $60 = $70)
End the flashloan
```
The user ends up with \$70 deposited, and \$60 in debt. The portfolio net value is still \$10. We
incurred just one swap + slippage fee, when swapping the \$60 of B for A. We will also incur another
set of swap fees when we close this position.

### Strategy Fees

Just like loops, there is no fee to open a strategy, except the swap fee and slippage incurred to
create the looped position. Consider that swapping from A/B to C/D may incur two sets of swap fees:
one to close out the A/B loop, and another to open the C/D loop.

## Fees From Other Venues

Currently, none of the venues integrated with p0 charge any notable fees.
