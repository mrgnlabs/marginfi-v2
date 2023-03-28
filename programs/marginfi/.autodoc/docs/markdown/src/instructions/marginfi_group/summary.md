[View code on GitHub](https://github.com/mrgnlabs/marginfi-v2/.autodoc/docs/json/src/instructions/marginfi_group)

The `marginfi_group` folder contains code related to the Marginfi v2 project's margin group functionality. The `accrue_bank_interest.rs` file contains a function for accruing interest on a lending pool's bank account. The `add_pool.rs` file contains a function for adding a new bank to the lending pool. The `collect_bank_fees.rs` file contains a function for collecting fees from a lending pool bank and distributing them to the appropriate vaults. The `configure.rs` file contains a function for configuring a margin group. The `configure_bank.rs` file contains a function for configuring a lending pool bank. The `handle_bankruptcy.rs` file contains a function for handling bankrupt marginfi accounts. The `initialize.rs` file contains a function for initializing a new Marginfi group.

These files and functions are all related to the banking operations of the Marginfi v2 project. They provide functionality for accruing interest, adding and configuring banks, collecting fees, handling bankrupt accounts, and initializing the banking system. The `mod.rs` file re-exports these modules to provide a high-level interface for other parts of the project to access and use these banking operations.

For example, a function that allows users to deposit funds into the lending pool may use the `add_pool` function to add funds to the liquidity pool. The `accrue_bank_interest` function can be used to calculate and add interest to bank accounts. The `collect_bank_fees` function can be used to collect fees from bank accounts and distribute them to the appropriate vaults. The `configure` and `configure_bank` functions can be used to configure various aspects of the banking system. The `handle_bankruptcy` function can be used to handle bankrupt accounts and ensure that bad debt is covered and socialized between lenders if necessary.

Here is an example of how these functions might be used in the larger project:

```rust
use marginfi_v2::banking_operations::*;

fn main() {
    initialize_bank();
    configure_bank();
    add_to_liquidity_pool(1000);
    accrue_interest();
    collect_fees();
    handle_bankruptcy();
}
```

In this example, we import all of the banking operations from the `marginfi_v2` project and use them to initialize the bank, configure it, add funds to a liquidity pool, accrue interest, collect fees, and handle bankrupt accounts. This code demonstrates how these functions can be used to provide a high-level interface for banking operations in the larger project.
