import { BN, IdlAccounts, Program, workspace } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  globalFeeWallet,
  groupAdmin,
  globalProgramAdmin,
  users,
  PROGRAM_FEE_FIXED,
  PROGRAM_FEE_RATE,
  INIT_POOL_ORIGINATION_FEE,
  bankrunProgram,
  bankrunContext,
  banksClient,
} from "./rootHooks";
import { assert, expect } from "chai";
import { deriveGlobalFeeState } from "./utils/pdas";
import {
  initGlobalFeeState,
  panicPause,
  panicUnpause,
  panicUnpausePermissionless,
} from "./utils/group-instructions";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import {
  assertBankrunTxFailed,
  assertBNApproximately,
  assertBNEqual,
} from "./utils/genericTests";
import { PAUSE_DURATION_SECONDS } from "./utils/types";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";

describe("Panic Mode state test (Bankrun)", () => {
  type FeeState = IdlAccounts<Marginfi>["feeState"];

  let feeStateKey: PublicKey;
  let feeState: FeeState;

  let firstTimestamp: BN;

  before(async () => {
    feeStateKey = deriveGlobalFeeState(bankrunProgram.programId)[0];

    // Initialize fee state if it doesn't exist
    try {
      feeState = await bankrunProgram.account.feeState.fetch(feeStateKey);
    } catch (err) {
      const tx = new Transaction();
      tx.add(
        await initGlobalFeeState(globalProgramAdmin.mrgnBankrunProgram, {
          payer: globalProgramAdmin.wallet.publicKey,
          admin: globalProgramAdmin.wallet.publicKey,
          wallet: globalFeeWallet,
          bankInitFlatSolFee: INIT_POOL_ORIGINATION_FEE,
          programFeeFixed: bigNumberToWrappedI80F48(PROGRAM_FEE_FIXED),
          programFeeRate: bigNumberToWrappedI80F48(PROGRAM_FEE_RATE),
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(globalProgramAdmin.wallet);
      await banksClient.processTransaction(tx);

      feeState = await bankrunProgram.account.feeState.fetch(feeStateKey);
    }
  });

  it("(fee admin) pause the protocol - happy path", async () => {
    const tx = new Transaction();
    tx.add(await panicPause(globalProgramAdmin.mrgnBankrunProgram, {}));
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(globalProgramAdmin.wallet);

    await banksClient.processTransaction(tx);

    const fs = await bankrunProgram.account.feeState.fetch(feeStateKey);
    assert.equal(fs.panicState.pauseFlags, 1);

    const now = Math.round(Date.now() / 1000);
    assertBNApproximately(fs.panicState.pauseStartTimestamp, now, 10);
    assertBNApproximately(fs.panicState.lastDailyResetTimestamp, now, 10);

    firstTimestamp = fs.panicState.lastDailyResetTimestamp;
    assert.equal(fs.panicState.dailyPauseCount, 1);
    assert.equal(fs.panicState.consecutivePauseCount, 1);
  });

  it("(fee admin) extends an existing pause - happy path", async () => {
    const tx = new Transaction();
    tx.add(
      await panicPause(globalProgramAdmin.mrgnBankrunProgram, {}),
      // Dummy tx to trick bankrun
      SystemProgram.transfer({
        fromPubkey: globalProgramAdmin.wallet.publicKey,
        toPubkey: users[0].wallet.publicKey,
        lamports: 54321,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(globalProgramAdmin.wallet);

    await banksClient.processTransaction(tx);

    const fs = await bankrunProgram.account.feeState.fetch(feeStateKey);
    assert.equal(fs.panicState.pauseFlags, 1);

    // Extension applies to the previous "start" time (which can be in the future).
    assertBNApproximately(
      firstTimestamp.addn(PAUSE_DURATION_SECONDS),
      fs.panicState.pauseStartTimestamp,
      10
    );
    // No change on reset
    assertBNEqual(fs.panicState.lastDailyResetTimestamp, firstTimestamp);
    assert.equal(fs.panicState.dailyPauseCount, 2);
    assert.equal(fs.panicState.consecutivePauseCount, 2);
  });

  // TODO deposit/borrow/withdraw/repay now fail as expected while paused
  // TODO another extend should fail due to pause limit
  // TODO bankrun into the future and show we can pause again

  it("(attacker) tries to pause - should fail", async () => {
    const tx = new Transaction();
    tx.add(
      await panicPause(users[0].mrgnBankrunProgram, {}),
      // Dummy tx to trick bankrun
      SystemProgram.transfer({
        fromPubkey: users[0].wallet.publicKey,
        toPubkey: users[1].wallet.publicKey,
        lamports: 654321,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(users[0].wallet);

    const result = await banksClient.tryProcessTransaction(tx);
    // generic has_one violation (fee state admin doesn't match fee state)
    assertBankrunTxFailed(result, 2001);
  });

  it("(fee admin) tries to pause beyond daily pause limits - should fail", async () => {
    const tx = new Transaction();
    tx.add(await panicPause(globalProgramAdmin.mrgnBankrunProgram, {}));
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(globalProgramAdmin.wallet);

    const result = await banksClient.tryProcessTransaction(tx);
    // PauseLimitExceeded
    assertBankrunTxFailed(result, 6082);
  });

  it("(fee admin) admin unpause - happy path", async () => {
    const tx = new Transaction();
    tx.add(await panicUnpause(globalProgramAdmin.mrgnBankrunProgram, {}));
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(globalProgramAdmin.wallet);

    await banksClient.processTransaction(tx);

    const fs = await bankrunProgram.account.feeState.fetch(feeStateKey);
    assert.equal(fs.panicState.pauseFlags, 0);
    assertBNEqual(fs.panicState.pauseStartTimestamp, 0);
    // No change to reset timestamp
    assertBNEqual(fs.panicState.lastDailyResetTimestamp, firstTimestamp);
    assert.equal(fs.panicState.consecutivePauseCount, 0);
  });

  it("(fee admin) admin unpause when not paused - should fail", async () => {
    const tx = new Transaction();
    tx.add(
      await panicUnpause(globalProgramAdmin.mrgnBankrunProgram, {}),
      // Dummy tx to trick bankrun
      SystemProgram.transfer({
        fromPubkey: globalProgramAdmin.wallet.publicKey,
        toPubkey: users[1].wallet.publicKey,
        lamports: 456783,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(globalProgramAdmin.wallet);

    const result = await banksClient.tryProcessTransaction(tx);
    // ProtocolNotPaused
    assertBankrunTxFailed(result, 6083);
  });

  it("(attacker) non-admin tries to call admin unpause - should fail", async () => {
    const tx = new Transaction();
    tx.add(await panicUnpause(users[0].mrgnBankrunProgram, {}));
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(users[0].wallet);

    const result = await banksClient.tryProcessTransaction(tx);
    // generic has_one violation (fee state admin doesn't match fee state)
    assertBankrunTxFailed(result, 2001);
  });

  it("(permissionless) permissionless unpause when not paused - should fail", async () => {
    const tx = new Transaction();
    tx.add(await panicUnpausePermissionless(users[0].mrgnBankrunProgram, {}));
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(users[0].wallet);

    const result = await banksClient.tryProcessTransaction(tx);
    // ProtocolNotPaused
    assertBankrunTxFailed(result, 6083);
  });
});
