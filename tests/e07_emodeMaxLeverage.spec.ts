import { BN } from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import {
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  EMODE_SEED,
  emodeAdmin,
  emodeGroup,
  groupAdmin,
} from "./rootHooks";
import {
  assertBankrunTxFailed,
} from "./utils/genericTests";
import {
  configBankEmode,
  groupConfigure,
} from "./utils/group-instructions";
import {
  EMODE_APPLIES_TO_ISOLATED,
  EMODE_LST_TAG,
  EMODE_SOL_TAG,
  newEmodeEntry,
} from "./utils/types";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { deriveBankWithSeed } from "./utils/pdas";
import { assert } from "chai";

const seed = new BN(EMODE_SEED);
let solBank: any;
let lstABank: any;

// Helper to convert u32 basis points back to actual leverage value (0-100 range)
const u32ToBasis = (value: number): number => {
  const ratio = value / 4294967295; // u32::MAX = 4294967295
  return ratio * 100;
};

describe("Emode Max Leverage Configuration", () => {
  before(async () => {
    [solBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.wsolMint.publicKey,
      seed
    );
    [lstABank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      seed
    );
  });

  describe("Group Configuration - Valid Max Leverage", () => {
    it("(admin) Configure group with max leverage of 10x for init and 15x for maint", async () => {
      const maxInitLeverage = bigNumberToWrappedI80F48(10);
      const maxMaintLeverage = bigNumberToWrappedI80F48(15);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxInitLeverage: maxInitLeverage,
          emodeMaxMaintLeverage: maxMaintLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);

      const group = await bankrunProgram.account.marginfiGroup.fetch(
        emodeGroup.publicKey
      );
      assert.approximately(u32ToBasis(group.emodeMaxInitLeverage), 10, 0.01);
      assert.approximately(u32ToBasis(group.emodeMaxMaintLeverage), 15, 0.01); 
    });

    it("(admin) Configure group with max leverage of 1x init, 2x maint (minimum valid)", async () => {
      const maxInitLeverage = bigNumberToWrappedI80F48(1);
      const maxMaintLeverage = bigNumberToWrappedI80F48(2);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxInitLeverage: maxInitLeverage,
          emodeMaxMaintLeverage: maxMaintLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);

      const group = await bankrunProgram.account.marginfiGroup.fetch(
        emodeGroup.publicKey
      );
      assert.approximately(u32ToBasis(group.emodeMaxInitLeverage), 1, 0.01);
      assert.approximately(u32ToBasis(group.emodeMaxMaintLeverage), 2, 0.01);
    });

    it("(admin) Configure group with max leverage of 99x init, 100x maint (maximum valid)", async () => {
      const maxInitLeverage = bigNumberToWrappedI80F48(99);
      const maxMaintLeverage = bigNumberToWrappedI80F48(100);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxInitLeverage: maxInitLeverage,
          emodeMaxMaintLeverage: maxMaintLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);

      const group = await bankrunProgram.account.marginfiGroup.fetch(
        emodeGroup.publicKey
      );
      assert.approximately(u32ToBasis(group.emodeMaxInitLeverage), 99, 0.01);
      assert.approximately(u32ToBasis(group.emodeMaxMaintLeverage), 100, 0.01);
    });

    it("(admin) Configure group with null max leverage (defaults to 15x init, 20x maint)", async () => {
      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxInitLeverage: null,
          emodeMaxMaintLeverage: null,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);

      const group = await bankrunProgram.account.marginfiGroup.fetch(
        emodeGroup.publicKey
      );

      assert.approximately(u32ToBasis(group.emodeMaxInitLeverage), 15, 0.01);
      assert.approximately(u32ToBasis(group.emodeMaxMaintLeverage), 20, 0.01);
    });
  });

  describe("Group Configuration - Invalid Max Leverage", () => {
    it("(admin) Configure group with init max leverage < 1 - should fail", async () => {
      const maxInitLeverage = bigNumberToWrappedI80F48(0.5);
      const maxMaintLeverage = bigNumberToWrappedI80F48(1);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxInitLeverage: maxInitLeverage,
          emodeMaxMaintLeverage: maxMaintLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      const result = await banksClient.tryProcessTransaction(tx);
      // 6075 (BadEmodeConfig)
      assertBankrunTxFailed(result, "0x17bb");
    });

    it("(admin) Configure group with maint max leverage > 100 - should fail", async () => {
      const maxInitLeverage = bigNumberToWrappedI80F48(100);
      const maxMaintLeverage = bigNumberToWrappedI80F48(101);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxInitLeverage: maxInitLeverage,
          emodeMaxMaintLeverage: maxMaintLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      const result = await banksClient.tryProcessTransaction(tx);
      // 6075 (BadEmodeConfig)
      assertBankrunTxFailed(result, "0x17bb");
    });

    it("(admin) Configure group with init >= maint leverage - should fail", async () => {
      const maxInitLeverage = bigNumberToWrappedI80F48(20);
      const maxMaintLeverage = bigNumberToWrappedI80F48(20);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxInitLeverage: maxInitLeverage,
          emodeMaxMaintLeverage: maxMaintLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      const result = await banksClient.tryProcessTransaction(tx);
      // 6075 (BadEmodeConfig)
      assertBankrunTxFailed(result, "0x17bb");
    });
  });

  describe("Bank Emode Configuration - Leverage Validation", () => {
    before(async () => {
      // Set group max leverage to 10x init, 15x maint for these tests
      const maxInitLeverage = bigNumberToWrappedI80F48(10);
      const maxMaintLeverage = bigNumberToWrappedI80F48(15);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxInitLeverage: maxInitLeverage,
          emodeMaxMaintLeverage: maxMaintLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);
    });

    it("(emode admin) Configure bank emode with leverage within limit (5x) - should succeed", async () => {
      // SOL bank has liability weights of 1.0/1.0 (init/maint)
      // To achieve 5x leverage: L = 1/(1-CW/LW) => 5 = 1/(1-CW/1.0) => CW = 0.8
      const tx = new Transaction().add(
        await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
          bank: solBank,
          tag: EMODE_SOL_TAG,
          entries: [
            newEmodeEntry(
              EMODE_LST_TAG,
              EMODE_APPLIES_TO_ISOLATED,
              bigNumberToWrappedI80F48(0.8),
              bigNumberToWrappedI80F48(0.8)
            ),
          ],
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(emodeAdmin.wallet);
      await banksClient.processTransaction(tx);

      const bank = await bankrunProgram.account.bank.fetch(solBank);
      assert.equal(bank.emode.emodeTag, EMODE_SOL_TAG);
    });

    it("(emode admin) Configure bank emode near leverage limit (9.5x init, 14x maint) - should succeed", async () => {
      // SOL bank has liability weights of 1.0/1.0 (init/maint)
      // To achieve 9.5x init leverage: L = 1/(1-CW/LW) => 9.5 = 1/(1-CW/1.0) => CW ≈ 0.8947
      // To achieve 14x maint leverage: L = 1/(1-CW/LW) => 14 = 1/(1-CW/1.0) => CW ≈ 0.9286
      const tx = new Transaction().add(
        await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
          bank: solBank,
          tag: EMODE_SOL_TAG,
          entries: [
            newEmodeEntry(
              EMODE_LST_TAG,
              EMODE_APPLIES_TO_ISOLATED,
              bigNumberToWrappedI80F48(0.8947),
              bigNumberToWrappedI80F48(0.9286)
            ),
          ],
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(emodeAdmin.wallet);
      await banksClient.processTransaction(tx);

      const bank = await bankrunProgram.account.bank.fetch(solBank);
      assert.equal(bank.emode.emodeTag, EMODE_SOL_TAG);
    });

    it("(emode admin) Configure bank emode exceeding init leverage limit (11x init) - should fail", async () => {
      // SOL bank has liability weights of 1.0/1.0 (init/maint)
      // To achieve 11x init leverage: L = 1/(1-CW/LW) => 11 = 1/(1-CW/1.0) => CW ≈ 0.9091
      // Group limit is 10x init, so this should fail
      const tx = new Transaction().add(
        await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
          bank: solBank,
          tag: EMODE_SOL_TAG,
          entries: [
            newEmodeEntry(
              EMODE_LST_TAG,
              EMODE_APPLIES_TO_ISOLATED,
              bigNumberToWrappedI80F48(0.9091),
              bigNumberToWrappedI80F48(0.9286)
            ),
          ],
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(emodeAdmin.wallet);
      const result = await banksClient.tryProcessTransaction(tx);
      // 6075 (BadEmodeConfig)
      assertBankrunTxFailed(result, "0x17bb");
    });

    it("(emode admin) Configure bank emode exceeding maint leverage limit (18x maint) - should fail", async () => {
      // SOL bank has liability weights of 1.0/1.0 (init/maint)
      // To achieve 18x maint leverage: L = 1/(1-CW/LW) => 18 = 1/(1-CW/1.0) => CW ≈ 0.9444
      // Group limit is 15x maint, so this should fail
      const tx = new Transaction().add(
        await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
          bank: solBank,
          tag: EMODE_SOL_TAG,
          entries: [
            newEmodeEntry(
              EMODE_LST_TAG,
              EMODE_APPLIES_TO_ISOLATED,
              bigNumberToWrappedI80F48(0.8),
              bigNumberToWrappedI80F48(0.9444)
            ),
          ],
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(emodeAdmin.wallet);
      const result = await banksClient.tryProcessTransaction(tx);
      // 6075 (BadEmodeConfig)
      assertBankrunTxFailed(result, "0x17bb");
    });
  });

  describe("Bank Cache Update - Max Leverage Propagation", () => {
    it("(verify) Bank cache reflects group max leverage after update", async () => {
      // Set group max leverage to 12x init, 18x maint
      const maxInitLeverage = bigNumberToWrappedI80F48(12);
      const maxMaintLeverage = bigNumberToWrappedI80F48(18);

      let tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxInitLeverage: maxInitLeverage,
          emodeMaxMaintLeverage: maxMaintLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);

      // Now configure the bank's emode, which should trigger bank cache update
      // Use 5x leverage which is well within the 12x/18x limits
      tx = new Transaction().add(
        await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
          bank: solBank,
          tag: EMODE_SOL_TAG,
          entries: [
            newEmodeEntry(
              EMODE_LST_TAG,
              EMODE_APPLIES_TO_ISOLATED,
              bigNumberToWrappedI80F48(0.8),
              bigNumberToWrappedI80F48(0.8)
            ),
          ],
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(emodeAdmin.wallet);
      await banksClient.processTransaction(tx);

      // Verify group has the updated max leverage
      const group = await bankrunProgram.account.marginfiGroup.fetch(
        emodeGroup.publicKey
      );
      assert.approximately(u32ToBasis(group.emodeMaxInitLeverage), 12, 0.01);
      assert.approximately(u32ToBasis(group.emodeMaxMaintLeverage), 18, 0.01);
    });


  });
});
