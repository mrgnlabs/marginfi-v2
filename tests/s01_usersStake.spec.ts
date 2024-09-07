import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import {
  Connection,
  LAMPORTS_PER_SOL,
  PublicKey,
  SYSVAR_CLOCK_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import { users, validators, verbose } from "./rootHooks";
import { StakingCollatizer } from "../target/types/staking_collatizer";
import {
  createStakeAccount,
  delegateStake,
  getStakeAccount,
  getStakeActivation,
} from "./utils/stake-utils";
import { assertBNEqual, assertKeysEqual } from "./utils/genericTests";
import { u64MAX_BN } from "./utils/types";

import path from "path";
import { BankrunProvider } from "anchor-bankrun";
import type { ProgramTestContext } from "solana-bankrun";
import { Clock, startAnchor } from "solana-bankrun";

describe("User stakes some native and creates an account", () => {
  const program = workspace.StakingCollatizer as Program<StakingCollatizer>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  let bankrunContext: ProgramTestContext;
  let bankRunProvider: BankrunProvider;
  let bankrunProgram: Program<StakingCollatizer>;

  let stakeAccount: PublicKey;

  it("(user 0) Create user stake account and stake to validator", async () => {
    stakeAccount = await createStakeAccount(
      users[0],
      provider,
      10 * LAMPORTS_PER_SOL
    );

    await delegateStake(
      users[0],
      provider,
      stakeAccount,
      validators[0].voteAccount,
      verbose,
      "user 0"
    );

    const epochBefore = (await provider.connection.getEpochInfo()).epoch;
    const stakeAccountInfo = await provider.connection.getAccountInfo(
      stakeAccount
    );
    const stakeAccBefore = getStakeAccount(stakeAccountInfo.data);
    const meta = stakeAccBefore.meta;
    const delegation = stakeAccBefore.stake.delegation;
    const rent = new BN(meta.rentExemptReserve.toString());

    assertKeysEqual(delegation.voterPubkey, validators[0].voteAccount);
    assertBNEqual(
      new BN(delegation.stake.toString()),
      new BN(10 * LAMPORTS_PER_SOL).sub(rent)
    );
    assertBNEqual(new BN(delegation.activationEpoch.toString()), epochBefore);
    assertBNEqual(new BN(delegation.deactivationEpoch.toString()), u64MAX_BN);

    const stakeStatusBefore = await getStakeActivation(
      provider.connection,
      stakeAccount
    );
    if (verbose) {
      console.log("It is now epoch: " + epochBefore);
      console.log(
        "Stake active: " +
          stakeStatusBefore.active.toLocaleString() +
          " inactive " +
          stakeStatusBefore.inactive.toLocaleString() +
          " status: " +
          stakeStatusBefore.status
      );
    }
  });

  it("Advance the epoch", async () => {
    // Load the necessary accounts and add them to the bankrun context
    const accountKeys = [
      stakeAccount,
      users[0].wallet.publicKey,
      validators[0].voteAccount,
    ];
    const accounts = await program.provider.connection.getMultipleAccountsInfo(
      accountKeys
    );
    const addedAccounts = accountKeys.map((address, index) => ({
      address,
      info: accounts[index],
    }));

    bankrunContext = await startAnchor(path.resolve(), [], addedAccounts);
    bankRunProvider = new BankrunProvider(bankrunContext);
    bankrunProgram = new Program(program.idl, provider);
    const client = bankrunContext.banksClient;

    bankrunContext.warpToEpoch(1n);

    let clock = await client.getAccount(SYSVAR_CLOCK_PUBKEY);
    // epoch is bytes 16-24
    let epoch = new BN(clock.data.slice(16, 24), 10, "le").toNumber();
    if (verbose) {
      console.log("Warped to epoch: " + epoch);
    }

    const stakeStatusAfter1 = await getStakeActivation(
      bankRunProvider.connection,
      stakeAccount,
      epoch
    );
    if (verbose) {
      console.log("It is now epoch: " + epoch);
      console.log(
        "Stake active: " +
          stakeStatusAfter1.active.toLocaleString() +
          " inactive " +
          stakeStatusAfter1.inactive.toLocaleString() +
          " status: " +
          stakeStatusAfter1.status
      );
    }

    bankrunContext.warpToEpoch(2n);

    clock = await client.getAccount(SYSVAR_CLOCK_PUBKEY);
    // epoch is bytes 16-24
    epoch = new BN(clock.data.slice(16, 24), 10, "le").toNumber();
    if (verbose) {
      console.log("Warped to epoch: " + epoch);
    }

    const stakeStatusAfter2 = await getStakeActivation(
      bankRunProvider.connection,
      stakeAccount,
      epoch
    );
    if (verbose) {
      console.log("It is now epoch: " + epoch);
      console.log(
        "Stake active: " +
          stakeStatusAfter2.active.toLocaleString() +
          " inactive " +
          stakeStatusAfter2.inactive.toLocaleString() +
          " status: " +
          stakeStatusAfter2.status
      );
    }
  });

  //   it("(user 0) Init user account - happy path", async () => {
  //     // TODO the stake program must be rewritten to use the bankrun provider...
  //     const epoch = (await provider.connection.getEpochInfo()).epoch;
  //     const stakeStatusAfter = await getStakeActivation(
  //       provider.connection,
  //       stakeAccount
  //     );
  //     if (verbose) {
  //       console.log("It is now epoch: " + epoch);
  //       console.log(
  //         "Stake active: " +
  //           stakeStatusAfter.active.toLocaleString() +
  //           " inactive " +
  //           stakeStatusAfter.inactive.toLocaleString() +
  //           " status: " +
  //           stakeStatusAfter.status
  //       );
  //     }

  //     let tx = new Transaction();

  //     tx.add(
  //       await program.methods
  //         .initUser()
  //         .accounts({
  //           payer: users[0].wallet.publicKey,
  //         })
  //         .instruction()
  //     );

  //     await users[0].userCollatizerProgram.provider.sendAndConfirm(tx);
  //   });
});
