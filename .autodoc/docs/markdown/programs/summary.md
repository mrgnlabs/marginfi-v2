[View code on GitHub](https://github.com/mrgnlabs/marginfi-v2/.autodoc/docs/json/programs)

The `liquidity-incentive-program` folder in the MarginFi-v2 project contains code that provides the foundational elements for creating and managing liquidity incentive campaigns. This code includes constants, error handling, program logic, and account structures that developers can use to build out the functionality of the project.

The `src` folder contains several important files. The `constants.rs` file defines several constant strings that are used as seeds for various accounts and authorizations in the project. The `errors.rs` file defines an error enum called `LIPError` using the Rust programming language and the Anchor framework. The `LIPError` enum has three variants, each representing a different error condition that can occur in the larger project. The `lib.rs` file defines a Solana program for a liquidity incentive campaign in the MarginFi-v2 project. The program includes three functions: `create_campaign`, `create_deposit`, and `end_deposit`. These functions provide the functionality for creating and managing liquidity incentive campaigns in the project. The `state.rs` file defines two structs, `Campaign` and `Deposit`, which are used to represent accounts in the MarginFi-v2 project.

Developers can use these constants, error handling, program logic, and account structures to build out the functionality of the project. For example, a developer might use the `create_campaign` function to create a new campaign and then use the `create_deposit` function to allow users to deposit tokens into the campaign. The `end_deposit` function can then be used to handle the closing of deposits and the transfer of tokens back to depositors after a lockup period has ended.

Here is an example of how a developer might use the `create_campaign` function:

```rust
use liquidity_incentive_program::state::Campaign;

// create a new campaign
let campaign = Campaign::new();

// call the create_campaign function to create the campaign account on the blockchain
let campaign_account = create_campaign(&program_id, &campaign)?;
```

In this example, the `Campaign` struct is used to create a new campaign, and the `create_campaign` function is called to create the campaign account on the blockchain.

Overall, the code in this folder provides an important part of the functionality for creating and managing liquidity incentive campaigns in the MarginFi-v2 project. Developers can use this code to build out the features of the project and provide users with a way to participate in liquidity incentive campaigns and earn rewards for providing liquidity.
