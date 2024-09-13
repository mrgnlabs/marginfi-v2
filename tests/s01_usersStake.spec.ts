import { BN, Program, workspace } from "@coral-xyz/anchor";
import {
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  bankrunContext,
  bankRunProvider,
  users,
  validators,
  verbose,
  banksClient,
  bankrunProgram,
} from "./rootHooks";
import {
  createStakeAccount,
  delegateStake,
  getEpochAndSlot,
  getStakeAccount,
  getStakeActivation,
} from "./utils/stake-utils";
import { assertBNEqual, assertKeysEqual } from "./utils/genericTests";
import { u64MAX_BN } from "./utils/types";
import { SinglePoolProgram } from "@solana/spl-single-pool-classic";
import { getAssociatedTokenAddressSync } from "@mrgnlabs/mrgn-common";
import { depositToSinglePoolIxes } from "./utils/spl-staking-utils";

describe("User stakes some native and creates an account", () => {
  /** Users's validator 0 stake account */
  let stakeAccount: PublicKey;
  const stake = 10;

  it("(user 0) Create user stake account and stake to validator", async () => {
    let { createTx, stakeAccountKeypair } = createStakeAccount(
      users[0],
      stake * LAMPORTS_PER_SOL
    );
    createTx.recentBlockhash = bankrunContext.lastBlockhash;
    createTx.sign(users[0].wallet, stakeAccountKeypair);
    await banksClient.processTransaction(createTx);
    stakeAccount = stakeAccountKeypair.publicKey;

    if (verbose) {
      console.log("Create stake account: " + stakeAccount);
      console.log(
        " Stake: " +
          stake +
          " SOL (" +
          (stake * LAMPORTS_PER_SOL).toLocaleString() +
          " in native)"
      );
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

    let { epoch, slot } = await getEpochAndSlot(banksClient);
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
    assertBNEqual(new BN(delegation.activationEpoch.toString()), epoch);
    assertBNEqual(new BN(delegation.deactivationEpoch.toString()), u64MAX_BN);

    const stakeStatusBefore = await getStakeActivation(
      bankRunProvider.connection,
      stakeAccount,
      epoch
    );
    if (verbose) {
      console.log("It is now epoch: " + epoch + " slot " + slot);
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

  // User delegates to stake pool (this works fine)

  it("Advance the epoch", async () => {
    bankrunContext.warpToEpoch(1n);

    let { epoch: epochAfterWarp, slot: slotAfterWarp } = await getEpochAndSlot(
      banksClient
    );
    if (verbose) {
      console.log(
        "Warped to epoch: " + epochAfterWarp + " slot " + slotAfterWarp
      );
    }

    const stakeStatusAfter = await getStakeActivation(
      bankRunProvider.connection,
      stakeAccount,
      epochAfterWarp
    );
    if (verbose) {
      console.log(
        "Stake active: " +
          stakeStatusAfter.active.toLocaleString() +
          " inactive " +
          stakeStatusAfter.inactive.toLocaleString() +
          " status: " +
          stakeStatusAfter.status
      );
    }

    // Advance a few slots and send some dummy txes to end the rewards period

    // NOTE: ALL STAKE PROGRAM IXES ARE DISABLED DURING THE REWARDS PERIOD. THIS MUST OCCUR OR THE
    // STAKE PROGRAM CANNOT RUN

    for (let i = 0; i < 100; i++) {
      bankrunContext.warpToSlot(BigInt(i + slotAfterWarp + 1));
      const dummyTx = new Transaction();
      dummyTx.add(
        SystemProgram.transfer({
          fromPubkey: users[0].wallet.publicKey,
          toPubkey: bankrunProgram.provider.publicKey,
          lamports: i,
        })
      );
      dummyTx.recentBlockhash = bankrunContext.lastBlockhash;
      dummyTx.sign(users[0].wallet);
      await banksClient.processTransaction(dummyTx);
      if (i % 10 == 0) {
        console.log("Dummy ix: " + i);
        let { epoch, slot } = await getEpochAndSlot(banksClient);
        console.log("is now epoch: " + epoch + " slot " + slot);
      }
    }
  });

  // User runs StakeProgram.authorize (this fails)

  it("(user 0) Deposit stake to the LST pool", async () => {
    const userStakeAccount = users[0].accounts.get("v0_stakeacc");

    let tx = new Transaction();
    const ixes = await depositToSinglePoolIxes(
      bankRunProvider.connection,
      users[0].wallet.publicKey,
      validators[0].splPool,
      userStakeAccount,
      verbose
    );
    tx.add(...ixes);

    tx.recentBlockhash = bankrunContext.lastBlockhash;
    tx.sign(users[0].wallet);
    // @ts-ignore // Doesn't matter
    await banksClient.processTransaction(tx);
  });
});
