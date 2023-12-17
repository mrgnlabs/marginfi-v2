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
