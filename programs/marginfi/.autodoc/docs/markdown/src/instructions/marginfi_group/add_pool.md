[View code on GitHub](https://github.com/mrgnlabs/marginfi-v2/src/instructions/marginfi_group/add_pool.rs)

The `lending_pool_add_bank` function is used to add a new bank to the lending pool. This function is only accessible to the admin of the lending pool. The function takes in a `BankConfig` struct as an argument, which contains the configuration for the new bank. 

The function first loads the accounts required for the bank creation, including the bank account itself, the liquidity vault, insurance vault, and fee vault. It then loads the existing bank account and initializes a new `Bank` struct with the provided configuration. The new bank is then saved to the bank account.

The function also creates new token accounts for the liquidity vault, insurance vault, and fee vault. These accounts are initialized with the provided bank mint, and the respective vault authorities are set as the token authorities. The vault authorities are derived from the bank account and are created using a seed value and a bump value. 

Finally, the function emits a `LendingPoolBankCreateEvent` event, which contains information about the newly created bank, including the bank account and mint account. 

This function is a crucial part of the lending pool, as it allows the admin to add new banks to the pool. Banks are used to provide liquidity to the pool and earn interest on deposited funds. By adding new banks, the lending pool can increase its liquidity and provide more lending opportunities to borrowers. 

Example usage:

```rust
let bank_config = BankConfig {
    min_borrow_amount: 100,
    optimal_borrow_amount: 1000,
    max_borrow_amount: 10000,
    reserve_factor: 0.1,
    optimal_utilization_rate: 0.8,
    liquidation_threshold: 0.7,
    liquidation_penalty: 0.05,
    interest_rate_model: InterestRateModel::Fixed {
        rate: 0.1,
    },
};

let lending_pool_add_bank_accounts = LendingPoolAddBank {
    marginfi_group: marginfi_group_account.load()?,
    admin: admin_account,
    bank_mint: bank_mint_account.into(),
    bank: bank_account.load()?,
    liquidity_vault_authority: liquidity_vault_authority_account,
    liquidity_vault: liquidity_vault_account.into(),
    insurance_vault_authority: insurance_vault_authority_account,
    insurance_vault: insurance_vault_account.into(),
    fee_vault_authority: fee_vault_authority_account,
    fee_vault: fee_vault_account.into(),
    rent: Rent::default(),
    token_program: token_program_account.to_account_info(),
    system_program: system_program_account.to_account_info(),
};

lending_pool_add_bank(ctx, lending_pool_add_bank_accounts, bank_config)?;
```
## Questions: 
 1. What is the purpose of the `lending_pool_add_bank` function?
- The `lending_pool_add_bank` function adds a new bank to the lending pool and requires admin privileges.

2. What are the different accounts required for adding a new bank to the lending pool?
- The accounts required for adding a new bank to the lending pool include the `marginfi_group` account, `admin` account, `bank_mint` account, `bank` account, `liquidity_vault_authority` account, `liquidity_vault` account, `insurance_vault_authority` account, `insurance_vault` account, `fee_vault_authority` account, `fee_vault` account, `rent` account, `token_program` account, and `system_program` account.

3. What is the purpose of the `Bank` struct and what information does it store?
- The `Bank` struct stores information about a bank in the lending pool, including its configuration, mint, vaults, and authority seeds. It also has functions for validating the bank's configuration and oracle setup.