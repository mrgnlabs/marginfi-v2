[View code on GitHub](https://github.com/mrgnlabs/marginfi-v2/.autodoc/docs/json/programs/marginfi/src/instructions/marginfi_group)

The `marginfi_group` folder contains code related to the MarginFi-v2 project's lending pool system. The `accrue_bank_interest.rs` file contains a function that calculates and accrues interest on a lending pool's associated bank account. The `add_pool.rs` file contains a function that adds a new bank to the lending pool. The `collect_bank_fees.rs` file contains a function that collects fees from the lending pool bank and distributes them to the appropriate vaults. The `configure.rs` file contains a function that configures a MarginFi group. The `configure_bank.rs` file contains a function that configures a bank for a lending pool. The `handle_bankruptcy.rs` file contains a function that handles bankrupt MarginFi accounts. The `initialize.rs` file contains a function that initializes a new MarginFi group.

These functions are all related to the lending pool system in the MarginFi-v2 project. They are used to manage the lending pool's associated bank accounts, add new banks to the pool, collect fees, configure the MarginFi group and bank settings, handle bankrupt accounts, and initialize new MarginFi groups. 

For example, the `accrue_bank_interest` function is likely called periodically to ensure that the bank account balance stays up-to-date with the current interest rates. The `add_pool` function is used to add a new bank to the lending pool, while the `configure_bank` function is used to configure the bank settings. The `handle_bankruptcy` function is used to handle bankrupt MarginFi accounts, while the `initialize` function is used to initialize new MarginFi groups.

Here is an example of how the `add_pool` function might be used:

```rust
use marginfi_v2::add_pool;

fn main() {
    // Add a new bank to the lending pool
    let bank_config = BankConfig {
        oracle: Some(oracle_pubkey),
        interest_rate: 0.05,
        // Other bank configuration options
    };
    let accounts = LendingPoolAddBank {
        marginfi_group: marginfi_group_account.load::<MarginfiGroup>()?,
        admin: admin_account,
        bank_mint: bank_mint_account,
        bank: bank_account.load_mut::<Bank>()?,
        liquidity_vault_authority: liquidity_vault_authority_account,
        liquidity_vault: liquidity_vault_account,
        insurance_vault_authority: insurance_vault_authority_account,
        insurance_vault: insurance_vault_account,
        fee_vault_authority: fee_vault_authority_account,
        fee_vault: fee_vault_account,
        rent: rent_account,
        token_program: token_program_account,
        system_program: system_program_account,
    };
    add_pool::lending_pool_add_bank(accounts, bank_config)?;
}
```

Overall, the code in the `marginfi_group` folder is an important part of the MarginFi-v2 project's lending pool system. These functions are used to manage the lending pool's associated bank
