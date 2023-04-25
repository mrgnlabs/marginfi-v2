[View code on GitHub](https://github.com/mrgnlabs/marginfi-v2/.autodoc/docs/json/programs/liquidity-incentive-program/src/instructions)

The `instructions` folder in the `liquidity-incentive-program` subdirectory of the `marginfi-v2` project contains code files that are responsible for creating and managing liquidity incentive campaigns. 

The `create_campaign.rs` file contains a function that creates a new campaign for the MarginFi-v2 project. This function initializes a new campaign with certain parameters and creates the necessary accounts for it. The `create_deposit.rs` file contains a function that allows users to deposit tokens into active liquidity incentive campaigns. The function ensures that the campaign is active and that the deposit amount is valid before transferring the tokens and creating a new Marginfi account. The `end_deposit.rs` file contains a function that handles the closing of deposits and the transfer of tokens back to depositors after a lockup period has ended. 

These code files are important parts of the MarginFi-v2 project as they allow for the creation and management of liquidity incentive campaigns. They work together with other parts of the project to provide a complete set of features for users who want to participate in liquidity incentive campaigns and earn rewards for providing liquidity. 

For example, a developer might use the `create_campaign` function to create a new campaign and then use the `create_deposit` function to allow users to deposit tokens into the campaign. The `end_deposit` function can then be used to handle the closing of deposits and the transfer of tokens back to depositors after a lockup period has ended. 

Here is an example of how a developer might use these functions:

```rust
use marginfi_v2::liquidity_incentive_program::instructions::{create_campaign, create_deposit, end_deposit};

let ctx = Context::default();
let lockup_period = 60;
let max_deposits = 1000;
let max_rewards = 10000;

create_campaign::process(ctx, lockup_period, max_deposits, max_rewards)?;

let deposit_amount = 100;
let deposit_ctx = Context::default();
create_deposit::process(deposit_ctx, deposit_amount)?;

let end_deposit_ctx = Context::default();
end_deposit::process(end_deposit_ctx)?;
```

In this example, the developer first uses the `create_campaign` function to create a new campaign with a lockup period of 60 seconds, a maximum of 1000 deposits, and a maximum of 10000 rewards. They then use the `create_deposit` function to allow a user to deposit 100 tokens into the campaign. Finally, they use the `end_deposit` function to handle the closing of the deposit and the transfer of tokens back to the user after the lockup period has ended. 

Overall, the code in these files provides a convenient way to create and manage liquidity incentive campaigns in the MarginFi-v2 project.
