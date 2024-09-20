# Fuzz Tests

These fuzz tests test the soundness of the accounting of the marginfi protocol.
The tests work by interacting directly with the smart contract instructions and at the end comparing if the internal accounting matches the real balances in the token accounts.

The tests simulates all actions that are involved in internal accounting:

- Depositing
- Withdrawing
- Liquidate
- Accrue Interest
- Update Price Oracle
- Handle Bankruptcy

## How it works?

### Program Interaction

The framework directly invokes the functions that are normally invoked by the onchain program entry point, and stubs the cpi invoke calls to directly invoke the spl token instructions.

### State

The framework uses a bump allocator for account storage. All `AccountInfo` objects are referencing data in the bump allocator.

When an instruction is invoked we direct pass in the `AccountInfo` objects referencing allocated state.
Before the invoke we also copy to a local cache and revert the state if the instructions fail.

### Actions

The framework uses the arbitrary library to generate a random sequence of actions that are then processed on the same state.

### How to Run

Run `python3 ./generate_corpus.py`. You may use python if you don't have python3 installed, or you may need to install python.

Build with `cargo build`. 

If this fails, you probably need to update your Rust toolchain: 

`rustup install nightly-2024-06-05`

And possibly: 

`rustup component add rust-src --toolchain nightly-2024-06-05-x86_64-unknown-linux-gnu`

Run with `cargo +nightly-2024-06-05 fuzz run lend -Zbuild-std --strip-dead-code --no-cfg-fuzzing -- -max_total_time=300`

To rerun some tests after a failure: `cargo +nightly-2024-06-05 fuzz run -Zbuild-std lend artifacts/lend/crash-ae5084b9433152babdaf7dcd75781eacd7ea55c7`, replacing  the hash after crash- with the one you see in the terminal.
