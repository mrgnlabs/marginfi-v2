# Marginfi v2

## Roadmap

1. mrgn lend
2. ...

## Architecture

### Margin Group

Margin group is a parent component that configures, manages and determines rules for access to resources for its own margin accounts.

### Lending Pool

The lending pool is a sub components of the margin group.
The lending pool and the margin group have a one-to-one relationship, and its managed by the margin group admin.
The risk of the lending pool is managed by the margin group.

* Sum component of the margin group.
* Has one-to-one relationship with the margin group.
* Same admin authorization with the margin group.
* Risk is managed by the margin group.

Each lending pool controls one or more _Banks_

```rs
struct Banks {

}
```

### Banks

Represents a single asset in the pool. Records data to calculate the interest rate.

```rs
struct Banks {
    asset_mint: Pubkey,
    vault_token_account: Pubkey,
    total_deposits: u64,
    total_borrows: u64,
    ...
}
```

```
                     ┌────────────┐       ┌────────────┐       ┌───────────┐       ┌──────────┐
                     │ Margin/Risk│       │ Lending    │       │           │       │ Price    │
                     │ Group      │1─────1│ Pool       │1─────n│ Banks     │m─────n│ Oracle   │
                     │            │       │            │       │           │       │          │
                     └────────────┘       └────────────┘       └───────────┘       └──────────┘
                           1                    1
                           │                    │
                           │                    │
                           n                    1
┌───────────┐        ┌────────────┐       ┌────────────┐
│           │        │ Margin     │       │ Lending    │
│ Signer    │1──────n│ Account    │1─────1│ Account    │
│           │        │            │       │            │
└───────────┘        └────────────┘       └────────────┘
```

## Open Questions

Q: Should Lending Pool be and independent account, that is connected to the Margin Group?

Including the _Lending Pool_ in the _Margin Group_ account would reduce the tx size, however moving it out allows us to potentially reuse the _Lending Reserve_ down the line.