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
  assertI80F48Approx,
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
    it("(admin) Configure group with max leverage of 10x", async () => {
      const maxLeverage = bigNumberToWrappedI80F48(10);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxLeverage: maxLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);

      const group = await bankrunProgram.account.marginfiGroup.fetch(
        emodeGroup.publicKey
      );
      assertI80F48Approx(group.emodeMaxLeverageCache, 10);
    });

    it("(admin) Configure group with max leverage of 1x (minimum valid)", async () => {
      const maxLeverage = bigNumberToWrappedI80F48(1);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxLeverage: maxLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);

      const group = await bankrunProgram.account.marginfiGroup.fetch(
        emodeGroup.publicKey
      );
      assertI80F48Approx(group.emodeMaxLeverageCache, 1);
    });

    it("(admin) Configure group with max leverage of 100x (maximum valid)", async () => {
      const maxLeverage = bigNumberToWrappedI80F48(100);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxLeverage: maxLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);

      const group = await bankrunProgram.account.marginfiGroup.fetch(
        emodeGroup.publicKey
      );
      assertI80F48Approx(group.emodeMaxLeverageCache, 100);
    });

    it("(admin) Configure group with null max leverage (defaults to 20x)", async () => {
      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxLeverage: null,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);

      const group = await bankrunProgram.account.marginfiGroup.fetch(
        emodeGroup.publicKey
      );

      assertI80F48Approx(group.emodeMaxLeverageCache, 20);
    });
  });

  describe("Group Configuration - Invalid Max Leverage", () => {
    it("(admin) Configure group with max leverage < 1 - should fail", async () => {
      const maxLeverage = bigNumberToWrappedI80F48(0.5);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxLeverage: maxLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      const result = await banksClient.tryProcessTransaction(tx);
      // 6075 (BadEmodeConfig)
      assertBankrunTxFailed(result, "0x17bb");
    });

    it("(admin) Configure group with max leverage > 100 - should fail", async () => {
      const maxLeverage = bigNumberToWrappedI80F48(101);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxLeverage: maxLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      const result = await banksClient.tryProcessTransaction(tx);
      // 6075 (BadEmodeConfig)
      assertBankrunTxFailed(result, "0x17bb");
    });

    it("(admin) Configure group with max leverage of 0 - should fail", async () => {
      const maxLeverage = bigNumberToWrappedI80F48(0);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxLeverage: maxLeverage,
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
      // Set group max leverage to 10x for these tests
      const maxLeverage = bigNumberToWrappedI80F48(10);

      const tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxLeverage: maxLeverage,
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

    it("(emode admin) Configure bank emode exactly at leverage limit (10x) - should succeed", async () => {
      // SOL bank has liability weights of 1.0/1.0 (init/maint)
      // To achieve exactly 10x leverage: L = 1/(1-CW/LW) => 10 = 1/(1-CW/1.0) => CW = 0.9
      const tx = new Transaction().add(
        await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
          bank: solBank,
          tag: EMODE_SOL_TAG,
          entries: [
            newEmodeEntry(
              EMODE_LST_TAG,
              EMODE_APPLIES_TO_ISOLATED,
              bigNumberToWrappedI80F48(0.9),
              bigNumberToWrappedI80F48(0.9)
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

    it("(emode admin) Configure bank emode exceeding leverage limit (15x) - should fail", async () => {
      // SOL bank has liability weights of 1.0/1.0 (init/maint)
      // To achieve 15x leverage: L = 1/(1-CW/LW) => 15 = 1/(1-CW/1.0) => CW ≈ 0.9333
      const tx = new Transaction().add(
        await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
          bank: solBank,
          tag: EMODE_SOL_TAG,
          entries: [
            newEmodeEntry(
              EMODE_LST_TAG,
              EMODE_APPLIES_TO_ISOLATED,
              bigNumberToWrappedI80F48(0.9333),
              bigNumberToWrappedI80F48(0.9333)
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

    it("(emode admin) Configure bank emode with very high leverage (25x) - should fail", async () => {
      // SOL bank has liability weights of 1.0/1.0 (init/maint)
      // To achieve 25x leverage: L = 1/(1-CW/LW) => 25 = 1/(1-CW/1.0) => CW = 0.96
      // This exceeds the group's 10x limit
      const tx = new Transaction().add(
        await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
          bank: solBank,
          tag: EMODE_SOL_TAG,
          entries: [
            newEmodeEntry(
              EMODE_LST_TAG,
              EMODE_APPLIES_TO_ISOLATED,
              bigNumberToWrappedI80F48(0.96),
              bigNumberToWrappedI80F48(0.96)
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
      // Set group max leverage to 15x
      const maxLeverage = bigNumberToWrappedI80F48(15);

      let tx = new Transaction().add(
        await groupConfigure(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          emodeMaxLeverage: maxLeverage,
        })
      );

      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);

      // Now configure the bank's emode, which should trigger bank cache update
      // Use 5x leverage which is well within the 15x limit
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

      // Verify bank cache has the updated max leverage
      const bank = await bankrunProgram.account.bank.fetch(solBank);
      assertI80F48Approx(bank.cache.maxEmodeLeverage, 15);
    });


  });
});
