import { Program, workspace } from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  globalFeeWallet,
  groupAdmin,
  globalProgramAdmin,
  users,
  PROGRAM_FEE_FIXED,
  PROGRAM_FEE_RATE,
  INIT_POOL_ORIGINATION_FEE,
} from "./rootHooks";
import { assert, expect } from "chai";
import { deriveGlobalFeeState } from "./utils/pdas";
import { initGlobalFeeState } from "./utils/group-instructions";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";

describe("Panic Mode state test", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  let feeStateKey: PublicKey;
  let feeState: any;

  before(async () => {
    feeStateKey = deriveGlobalFeeState(program.programId)[0];
    
    // Initialize fee state if it doesn't exist
    try {
      feeState = await program.account.feeState.fetch(feeStateKey);
    } catch (err) {
      // Fee state doesn't exist, need to initialize it first
      const tx = new Transaction();
      tx.add(
        await initGlobalFeeState(program, {
          payer: globalProgramAdmin.wallet.publicKey,
          admin: globalProgramAdmin.wallet.publicKey,
          wallet: globalFeeWallet,
          bankInitFlatSolFee: INIT_POOL_ORIGINATION_FEE,
          programFeeFixed: bigNumberToWrappedI80F48(PROGRAM_FEE_FIXED),
          programFeeRate: bigNumberToWrappedI80F48(PROGRAM_FEE_RATE),
        })
      );
      await globalProgramAdmin.mrgnProgram!.provider.sendAndConfirm(tx);
      feeState = await program.account.feeState.fetch(feeStateKey);
    }
  });

  describe("Panic Pause", () => {
    it("(admin) Should successfully pause the protocol", async () => {
      const tx = new Transaction();
      
      tx.add(
        await program.methods
          .panicPause()
          .accounts({
            globalFeeAdmin: globalProgramAdmin.wallet.publicKey,
            feeState: feeStateKey,
          })
          .instruction()
      );

      await globalProgramAdmin.mrgnProgram!.provider.sendAndConfirm(tx);

      const updatedFeeState = await program.account.feeState.fetch(feeStateKey);
      
      assert.equal(updatedFeeState.panicState.isPaused, 1, "Protocol should be paused");
      assert.isAbove(updatedFeeState.panicState.pauseStartTimestamp.toNumber(), 0, "Pause start timestamp should be set");
      assert.equal(updatedFeeState.panicState.dailyPauseCount, 1, "Daily pause count should be 1");
      assert.equal(updatedFeeState.panicState.consecutivePauseCount, 1, "Consecutive pause count should be 1");
    });

    it("Should fail when trying to pause an already paused protocol", async () => {
      const tx = new Transaction();
      
      tx.add(
        await program.methods
          .panicPause()
          .accounts({
            globalFeeAdmin: globalProgramAdmin.wallet.publicKey,
            feeState: feeStateKey,
          })
          .instruction()
      );

      try {
        await globalProgramAdmin.mrgnProgram!.provider.sendAndConfirm(tx);
        assert.fail("Should have failed to pause already paused protocol");
      } catch (err: any) {
        expect(err.message).to.include("ProtocolAlreadyPaused");
      }
    });

    it("Should fail when non-admin tries to pause", async () => {
      await unpauseProtocol();

      const tx = new Transaction();
      
      tx.add(
        await program.methods
          .panicPause()
          .accounts({
            globalFeeAdmin: users[0].wallet.publicKey,
            feeState: feeStateKey,
          })
          .instruction()
      );

      try {
        await users[0].mrgnProgram!.provider.sendAndConfirm(tx, [users[0].wallet]);
        assert.fail("Should have failed with unauthorized user");
      } catch (err: any) {
        expect(err.message).to.include("ConstraintHasOne");
      }
    });

    it("Should enforce daily pause limits", async () => {
      await ensureUnpaused();
      
      for (let i = 0; i < 2; i++) {
        await pauseProtocol();
        await unpauseProtocol();
      }

      // Try to pause a 4th time (should fail due to daily limit of 3)
      const tx = new Transaction();
      
      tx.add(
        await program.methods
          .panicPause()
          .accounts({
            globalFeeAdmin: globalProgramAdmin.wallet.publicKey,
            feeState: feeStateKey,
          })
          .instruction()
      );

      try {
        await globalProgramAdmin.mrgnProgram!.provider.sendAndConfirm(tx);
        assert.fail("Should have failed due to daily pause limit");
      } catch (err: any) {
        expect(err.message).to.include("PauseLimitExceeded");
      }
    });


  });

  describe("Panic Unpause (Admin)", () => {

    it("Should fail when trying to unpause when not paused", async () => {
      await ensureUnpaused();

      const tx = new Transaction();
      
      tx.add(
        await program.methods
          .panicUnpause()
          .accounts({
            globalFeeAdmin: globalProgramAdmin.wallet.publicKey,
            feeState: feeStateKey,
          })
          .instruction()
      );

      try {
        await globalProgramAdmin.mrgnProgram!.provider.sendAndConfirm(tx);
        assert.fail("Should have failed to unpause non-paused protocol");
      } catch (err: any) {
        expect(err.message).to.include("ProtocolNotPaused");
      }
    });

    it("Should fail when non-admin tries to call unpause", async () => {
      const tx = new Transaction();
      
      tx.add(
        await program.methods
          .panicUnpause()
          .accounts({
            globalFeeAdmin: users[0].wallet.publicKey,
            feeState: feeStateKey,
          })
          .instruction()
      );

      try {
        await users[0].mrgnProgram!.provider.sendAndConfirm(tx, [users[0].wallet]);
        assert.fail("Should have failed with unauthorized user");
      } catch (err: any) {
        expect(err.message).to.include("ConstraintHasOne");
      }
    });
  });

  describe("Panic Unpause Permissionless", () => {

    it("Should fail when trying to unpause when not paused", async () => {
      await ensureUnpaused();

      const tx = new Transaction();
      
      tx.add(
        await program.methods
          .panicUnpausePermissionless()
          .accounts({
            signer: users[0].wallet.publicKey,
          })
          .instruction()
      );

      try {
        await users[0].mrgnProgram!.provider.sendAndConfirm(tx, [users[0].wallet]);
        assert.fail("Should have failed to unpause non-paused protocol");
      } catch (err: any) {
        expect(err.message).to.include("ProtocolNotPaused");
      }
    });
  });




  // Helper functions
  async function pauseProtocol() {
    const feeState = await program.account.feeState.fetch(feeStateKey);
    if (feeState.panicState.isPaused === 1) {
      return; // Already paused
    }

    const tx = new Transaction();
    tx.add(
      await program.methods
        .panicPause()
        .accounts({
          globalFeeAdmin: globalProgramAdmin.wallet.publicKey,
          feeState: feeStateKey,
        })
        .instruction()
    );
    await globalProgramAdmin.mrgnProgram!.provider.sendAndConfirm(tx);
  }

  async function unpauseProtocol() {
    const feeState = await program.account.feeState.fetch(feeStateKey);
    if (feeState.panicState.isPaused === 0) {
      return; 
    }

    const tx = new Transaction();
    tx.add(
      await program.methods
        .panicUnpause()
        .accounts({
          globalFeeAdmin: globalProgramAdmin.wallet.publicKey,
          feeState: feeStateKey,
        })
        .instruction()
    );
    await globalProgramAdmin.mrgnProgram!.provider.sendAndConfirm(tx);
  }

  async function ensureUnpaused() {
    const feeState = await program.account.feeState.fetch(feeStateKey);
    if (feeState.panicState.isPaused === 1) {
      await unpauseProtocol();
    }
  }

});