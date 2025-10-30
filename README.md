# Project Zero: Built on the Marginfi v2 Program

See more specific guides for developers, emode, fees, bankrupcies, liquidation, and more in the /guides folder.

## Overview

The Marginfi program is a decentralized borrow-lending platform that enables undercollateralized lending against a variety of assets, including cryptocurrencies such as SOL, USDC, USDT, etc, natively staked SOL, Liquid Staking Tokens (LST), and even lending positions on other platforms such as Kamino.

## Glossary

- **Group** - A collection of banks. Groups have a single adminstrator (typically a secure
  governance MS) who has broad authority over them, and several delegate admins (limit, emode, etc)
  that can perform less risky modifications. All the assets you see at app.0.xyz are in a single
  group overseen by the foundation.
- **Bank** - Each asset available to borrow and lend on mrgnlend has a Bank, this account controls
  all the settings for the asset. Many banks might exist for the same asset. Every asset on
  app.0.xyz is a Bank.
- **Account** - Users can create as many Accounts as they want. Accounts are per `Group`, and each
  account can have up to 16 positions in any `Bank`. The Account contains various user-specific
  settings and cached values in addition to their `LendingAccount`, where `Balances` are stored.
- **Balance** - A Balance is an asset or liability on a single `Bank`. Users cannot have both an
  asset and liability on the same Bank, and can have at most one Balance per `Bank`. Each `Account`
  has a `LendingAccount`, which is a collection of 16 Balances. Balances can be blank/unused, and
  are always sorted in byte order by the corresponding `Bank`'s Public Key.
- **Asset Weight** - Each asset available to lend has two rates: The "Initial" and "Maintenance"
  rates. The Maintenance rate is always higher. If attempting to execute a borrow, collateral is
  valued at (price x initial rate). If a liquidator is attempting a liquidation, collateral is
  valued at (price x maintenance rate). The range between these is sometimes called the health
  buffer. For example, if a user has collateral worth \$10, and init/maint rates are 50\% and 60\%
  respectively, the user can borrow \$10 x .5 = \$5 in collateral. For liquidation purposes, their
  collateral is worth \$10 x .6 = \$6. The LTV you see on app.0.xyz is the "Initial" Weight, while
  the health displayed on the portfolio page uses the "Maintenance" Weight.
- **Liabiility Weight** - Each asset also has a liability weight! Like the `Asset Weight`, this is
  split into "Initial" and "Maintenance", where the Maintenance rate is always lower. If attempting
  to execute a borrow, liabilities are valued at (price x initial rate). If a liquidator is
  attempting a liquidation, liabilities are valued at (price x maintenance rate). Liability Weights
  are typically hidden on the front end to avoid ux complexity, but can be read on-chain.
- **Oracle** - Each `Bank` has an oracle it uses to determine the price of the asset it transacts
  in. The `Group` admin is responsible for picking and maintaing the Oracle. Typically, Switchboard
  is the orale provider, but Pyth is also supported, and some banks have a Fixed price. An Oracle
  may use multiple accounts, for example a Kamino bank uses a price source and the Kamino reserve.
- **Oracle Confidence Interval** - Some Oracles report a price with Confidence, e.g. P +/- c. When
  pricing assets, we use $P - P*c$, and when pricing liabilities, we use $P + P*c$. For example, if
  Oracles report A is $20 +/- $1, assets in A are priced at $19, while liablities are priced at $21.
  The confidence interval will be no higher than 5%. If the Oracle reports a confidence interval
  higher than 5%, we may clamp it to 5% or abort the transaction.

## Architecture at a Glance

```
┌────────────┐       ┌───────────┐       ┌──────────┐
│ Marginfi   |       │           │       │          │
│ Group      │1─────n│ Bank      │1─────1│ Oracle   │
│            │       │           │       │          │
└────────────┘       └───────────┘       └──────────┘
      1                    1
      |                    │
      |                    │
      n                    1
┌───────────┐       ┌────────────┐
│ Margin    │       │            │
│ Account   │1──<=16│  Balance   │
│           │       │            │
└───────────┘       └────────────┘
```

## Interest Accumulation

Interest accumulates when users borrow an asset. Borrowers are assessed interest, while lenders earn
it. A portion of interest paid by borrowers also goes to fees. Interest accumulates on your
position, entirely within the program: no interaction is neccessary beyond depositing, borrowing,
withdrawing, and repaying.

Example:

```
      * Sally deposits $20 in A
      * Sally borrows $10 in B
      * Sally's $10 debt today accrues interest and grows to $11.
      * The $1 difference is not debited until Sally repays: i.e. she will have to repay $11 to close this debt.
      * Sally's net balance and borrowing power has declined from 20-10 = $10 to 20-11 = $9
      * If she cannot repay, she may not be able to withdraw all of her A.

      * Bob deposits $10 in B
      * Thanks to Sally's interest, Bob's B is now worth $11.
      * Bob doesn't realize his $1 gain until he withdraws.
      * Bob's net balance and borrowing power has increased from $10 to $11
      * Bob can withdraw $11 any time
```

## Risk Engine

To maintain account health, Marginfi uses a deterministic risk engine that ensures that borrowing
and lending activities are within acceptable risk parameters. If a user's account falls below the
minimum required health factor, they may be subject to liquidation to protect the integrity of the
lending pool and other users' accounts.

In a nutshell, health is caclulated as the sum of assets minus the sum of liabilities, after all
applicable weights and confidence intervals have been applied. An Account cannot borrow more once
its health is less than zero using Initial weights, and is subject to liquidation if health is less
than zero using Maintenance Weights.

Example:

```
Token A is worth $10 with conf $0.212 (worth $9.788 low, $10.212 high)
USDC is worth $1 with conf $0.0212 (worth $0.9788 low, $1.0212 high)
Token A has a Maintenance Asset Weight of 10% (0.1)
USDC has a Maintenance Liability Weight of 100% (1)

User has:
ASSETS
   [0] 2 Token A (worth $20)
DEBTS
   [1] 5.05 USDC (worth $5.05)

$5.05 is 25.25% of $20, which is more than 10%, so liquidation is allowed!
Health calculation: (2 * 9.788 * .1) - (5.05 * 1.0212 * 1) = -3.19946
```

In the above example, a partial liquidation can restore the user's health:
```
Liquidator fee = 2.5%
Insurance fee = 2.5%

Liquidator tries to repay .2 token A (worth $2) of liquidatee's debt
Liquidator must pay
 value of A minus liquidator fee (low bias within the confidence interval): .2 * (1 - 0.025) * 9.788 = $1.90866
 USDC equivalent (high bias): 1.90866 / 1.0212 = $1.869036
Liquidatee receives
 value of A minus (liquidator fee + insurance) (low bias): .2 * (1 - 0.025 - 0.025) * 9.788 = $1.8608
 USDC equivalent (high bias): 1.8608 / 1.0212 = $1.822457
(Insurance fund and liquidator each collected a 2.5% fee: the $0.4679 difference)

Health after: ((2-0.2) * 9.788 * .1) - ((5.05-1.8608) * 1.0212 * 1) = -1.49497104
```


Users can send a `lending_account_pulse_health` to read the Risk Engine's assessment of their account at this moment for borrowing, liquidation, and bankruptcy purposes.

Whenever a user Borrows or Withdraws, the Risk Engine determines if the user would be within acceptable risk paramters after the tx completes, rejecting the tx if not.

Deposits and Repays require no Risk Engine check, as they can only improve health.

### Third Party Liquidation

Liquidation is open to third parties, and encouraged! An account that is unhealthy can be
liquidated, protecting the solvency of depositors and netting a small profit (2.5-10%) for the liquidator in
exchange for performing this service.
