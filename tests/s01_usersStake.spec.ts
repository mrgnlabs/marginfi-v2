import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  workspace,
} from "@coral-xyz/anchor";
import {
  LAMPORTS_PER_SOL,
  PublicKey,
  SYSVAR_CLOCK_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import {
  bankrunProgram as bankrunProgram,
  bankrunContext,
  bankRunProvider,
  users,
  validators,
  verbose,
  banksClient,
} from "./rootHooks";
import { StakingCollatizer } from "../target/types/staking_collatizer";
import {
  createStakeAccount,
  delegateStake,
  getStakeAccount,
  getStakeActivation,
} from "./utils/stake-utils";
import { assertBNEqual, assertKeysEqual } from "./utils/genericTests";
import { u64MAX_BN } from "./utils/types";

import { deriveStakeUser } from "./utils/stakeCollatizer/pdas";

describe("User stakes some native and creates an account", () => {
  const program = workspace.StakingCollatizer as Program<StakingCollatizer>;

  let stakeAccount: PublicKey;

  it("(user 0) Create user stake account and stake to validator", async () => {
    let { createTx, stakeAccountKeypair } = createStakeAccount(
      users[0],
      10 * LAMPORTS_PER_SOL
    );
    createTx.recentBlockhash = bankrunContext.lastBlockhash;
    createTx.sign(users[0].wallet, stakeAccountKeypair);
    await banksClient.processTransaction(createTx);
    stakeAccount = stakeAccountKeypair.publicKey;

    if (verbose) {
      console.log("Create stake account: " + stakeAccount);
      console.log(" Stake: " + 10 / LAMPORTS_PER_SOL + " SOL");
    }
    users[0].accounts.set("v0_stakeacc", stakeAccountKeypair.publicKey);

    let delegateTx = delegateStake(
      users[0],
      stakeAccount,
      validators[0].voteAccount
    );
    delegateTx.recentBlockhash = bankrunContext.lastBlockhash;
    delegateTx.sign(users[0].wallet);
    await banksClient.processTransaction(delegateTx);

    if (verbose) {
      console.log("user 0 delegated to " + validators[0].voteAccount);
    }

    let clock = await banksClient.getAccount(SYSVAR_CLOCK_PUBKEY);
    // epoch is bytes 16-24
    let epochBefore = new BN(clock.data.slice(16, 24), 10, "le").toNumber();
    const stakeAccountInfo = await bankRunProvider.connection.getAccountInfo(
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
      bankRunProvider.connection,
      stakeAccount,
      epochBefore
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
    bankrunContext.warpToEpoch(1n);

    let clock = await banksClient.getAccount(SYSVAR_CLOCK_PUBKEY);
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
  });

  it("(user 0) Init user account - happy path", async () => {
    let tx = new Transaction();

    tx.add(
      await program.methods
        .initUser()
        .accounts({
          payer: users[0].wallet.publicKey,
        })
        .instruction()
    );

    const [stakeUserKey] = deriveStakeUser(
      program.programId,
      users[0].wallet.publicKey
    );
    tx.recentBlockhash = bankrunContext.lastBlockhash;
    tx.sign(users[0].wallet);
    await banksClient.processTransaction(tx);

    let userAcc = await bankrunProgram.account.stakeUser.fetch(stakeUserKey);
    assertKeysEqual(userAcc.key, stakeUserKey);
  });
});
