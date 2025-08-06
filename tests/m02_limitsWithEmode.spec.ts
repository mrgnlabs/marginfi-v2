import { BN } from "@coral-xyz/anchor";
import {
  AddressLookupTableAccount,
  AddressLookupTableProgram,
  ComputeBudgetProgram,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import {
  groupAdmin,
  bankrunContext,
  banksClient,
  bankrunProgram,
  ecosystem,
  oracles,
  users,
  globalFeeWallet,
  verbose,
} from "./rootHooks";
import {
  configBankEmode,
  configureBank,
  groupConfigure,
} from "./utils/group-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { assert } from "chai";
import { defaultBankConfigOptRaw, newEmodeEntry } from "./utils/types";
import {
  borrowIx,
  composeRemainingAccounts,
  depositIx,
  liquidateIx,
  initLiquidationRecordIx,
  startLiquidationIx,
  endLiquidationIx,
  withdrawIx,
  repayIx,
} from "./utils/user-instructions";
import { deriveGlobalFeeState, deriveLiquidationRecord } from "./utils/pdas";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { dumpAccBalances } from "./utils/tools";
import { genericMultiBankTestSetup } from "./genericSetups";
import { getEpochAndSlot } from "./utils/stake-utils";
import { Clock } from "solana-bankrun";

/** Banks in this test use a "random" seed so their key is non-deterministic. */
let startingSeed: number;

/** This is the program-enforced maximum enforced number of balances per account. */
const MAX_BALANCES = 16;
const USER_ACCOUNT_THROWAWAY = "throwaway_account3";

let banks: PublicKey[] = [];
let throwawayGroup: Keypair;

describe("Limits on number of accounts, with emode in effect", () => {
  it("init group, init banks, and fund banks", async () => {
    const result = await genericMultiBankTestSetup(
      MAX_BALANCES,
      USER_ACCOUNT_THROWAWAY
    );
    startingSeed = result.startingSeed;
    banks = result.banks;
    throwawayGroup = result.throwawayGroup;
  });

  it("(admin) set the group admin as the emode admin too", async () => {
    const tx = new Transaction();
    tx.add(
      await groupConfigure(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: throwawayGroup.publicKey,
        newAdmin: groupAdmin.wallet.publicKey,
        newEmodeAdmin: groupAdmin.wallet.publicKey,
        isArena: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
  });

  it("(emode admin) Configures bank emodes - happy path", async () => {
    for (let bankIndex = 0; bankIndex < banks.length; bankIndex++) {
      const bank = banks[bankIndex];

      // pick 10 unique, random tags from 0..MAX_BALANCES-1 (excluding the last bank)
      const entryTags = [...Array(MAX_BALANCES - 1).keys()] // [0,1,2,…,14]
        .sort(() => Math.random() - 0.5) // shuffle
        .slice(0, 10); // take first 10

      // build the 10 entries for this bank with random tags and values
      const entries = entryTags.map((entryTag) =>
        newEmodeEntry(
          entryTag,
          1, // applies to isolated doesn't matter here
          bigNumberToWrappedI80F48(Math.random() * 0.3 + 0.6), // random 0.6–0.9
          bigNumberToWrappedI80F48(Math.random() * 0.1 + 0.9) // random 0.9–1.0
        )
      );

      // construct & send the tx for this bank
      const tx = new Transaction();
      tx.add(
        await configBankEmode(groupAdmin.mrgnBankrunProgram, {
          bank,
          tag: bankIndex, // bank’s own tag = its index
          entries,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet);
      await banksClient.processTransaction(tx);
    }
  });

  it("(admin) Seeds liquidity in all banks - validates 16 deposits is possible", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const amount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
    // Note: This is about the max per TX without using LUTs.
    const depositsPerTx = 5;

    for (let i = 0; i < banks.length; i += depositsPerTx) {
      const chunk = banks.slice(i, i + depositsPerTx);
      const tx = new Transaction();
      for (const bank of chunk) {
        tx.add(
          await depositIx(user.mrgnBankrunProgram, {
            marginfiAccount: userAccount,
            bank,
            tokenAccount: user.lstAlphaAccount,
            amount,
            depositUpToLimit: false,
          })
        );
      }
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(user.wallet);
      await banksClient.tryProcessTransaction(tx);
    }
  });

  it("(user 0) Borrows 15 positions against 1 - validates max borrows possible", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const depositAmount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
    const borrowAmount = new BN(1 * 10 ** ecosystem.lstAlphaDecimals);
    let oomAt = MAX_BALANCES;

    const tx = new Transaction();
    tx.add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[0],
        tokenAccount: user.lstAlphaAccount,
        amount: depositAmount,
        depositUpToLimit: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.tryProcessTransaction(tx);

    for (let i = 1; i < banks.length; i += 1) {
      const remainingAccounts: PublicKey[][] = [];
      remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
      for (let k = 1; k <= i; k++) {
        remainingAccounts.push([banks[k], oracles.pythPullLst.publicKey]);
      }

      const tx = new Transaction();
      tx.add(
        ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
        ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }),
        await borrowIx(user.mrgnBankrunProgram, {
          marginfiAccount: userAccount,
          bank: banks[i],
          tokenAccount: user.lstAlphaAccount,
          remaining: composeRemainingAccounts(remainingAccounts),
          amount: borrowAmount,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(user.wallet);
      let result = await banksClient.tryProcessTransaction(tx);
      console.log("***********" + i + " ***********");
      //dumpBankrunLogs(result);

      // Throws if the error is not OOM.
      if (result.result) {
        const logs = result.meta.logMessages;
        const isOOM = logs.some((msg) =>
          msg.toLowerCase().includes("memory allocation failed, out of memory")
        );

        if (isOOM) {
          oomAt = i + 1;
          console.warn(`⚠️ \t OOM during borrow on bank ${i}: \n`, logs);
          console.log("MAXIMUM ACCOUNTS BEFORE MEMORY FAILURE: " + oomAt);
          assert.ok(false);
        } else {
          // anything other than OOM should blow up the test
          throw new Error(
            `Unexpected borrowIx failure on bank ${banks[i].toBase58()}: ` +
              logs.join("\n")
          );
        }
      }
    }
    console.log("No memory failures detected on " + MAX_BALANCES + " accounts");
  });

  it("(admin) vastly increase last bank liability ratio to make user 0 unhealthy", async () => {
    let config = defaultBankConfigOptRaw();
    config.liabilityWeightInit = bigNumberToWrappedI80F48(210); // 21000%
    config.liabilityWeightMaint = bigNumberToWrappedI80F48(200); // 20000%

    let tx = new Transaction().add(
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: banks[MAX_BALANCES - 1],
        bankConfigOpt: config,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
  });

  it("(user 1) Liquidates user 0", async () => {
    const liquidatee = users[0];
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_THROWAWAY);
    const liquidator = users[1];
    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT_THROWAWAY);
    const depositAmount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
    const liquidateAmount = new BN(0.01 * 10 ** ecosystem.lstAlphaDecimals);

    const remainingAccounts: PublicKey[][] = [];
    for (let i = 0; i < MAX_BALANCES; i++) {
      remainingAccounts.push([banks[i], oracles.pythPullLst.publicKey]);
      // console.log("bank: " + banks[i]);
    }

    // Deposit some funds to operate as a liquidator...
    let tx = new Transaction();
    tx.add(
      await depositIx(liquidator.mrgnBankrunProgram, {
        marginfiAccount: liquidatorAccount,
        bank: banks[0],
        tokenAccount: liquidator.lstAlphaAccount,
        amount: depositAmount,
        depositUpToLimit: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(liquidator.wallet);
    await banksClient.tryProcessTransaction(tx);

    const liquidateeAcc = await bankrunProgram.account.marginfiAccount.fetch(
      liquidateeAccount
    );
    dumpAccBalances(liquidateeAcc);
    const liquidatorAcc = await bankrunProgram.account.marginfiAccount.fetch(
      liquidatorAccount
    );
    dumpAccBalances(liquidatorAcc);

    tx = new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
      await liquidateIx(liquidator.mrgnBankrunProgram, {
        assetBankKey: banks[0],
        liabilityBankKey: banks[MAX_BALANCES - 1],
        liquidatorMarginfiAccount: liquidatorAccount,
        liquidateeMarginfiAccount: liquidateeAccount,
        remaining: [
          oracles.pythPullLst.publicKey, // asset oracle
          oracles.pythPullLst.publicKey, // liab oracle

          ...composeRemainingAccounts([
            // liquidator accounts
            [banks[0], oracles.pythPullLst.publicKey],
            [banks[MAX_BALANCES - 1], oracles.pythPullLst.publicKey],
          ]),

          ...composeRemainingAccounts(
            // liquidatee accounts
            remainingAccounts
          ),
        ],
        amount: liquidateAmount,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(liquidator.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    // dumpBankrunLogs(result);

    // Throws if the error is not OOM.
    if (result.result) {
      const logs = result.meta.logMessages;
      const isOOM = logs.some((msg) =>
        msg.toLowerCase().includes("memory allocation failed, out of memory")
      );

      if (isOOM) {
        console.warn(`⚠️ \t OOM during liquidate: \n`, logs);
        assert.ok(false);
      } else {
        // anything other than OOM should blow up the test
        throw new Error(`Unexpected liquidate failure}: ` + logs.join("\n"));
      }
    }
  });

  it("(user 1) Liquidates user 0 with start/end", async () => {
    const liquidatee = users[0];
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_THROWAWAY);
    const liquidator = users[1];
    const remainingAccounts: PublicKey[][] = [];
    for (let i = 0; i < MAX_BALANCES; i++) {
      remainingAccounts.push([banks[i], oracles.pythPullLst.publicKey]);
    }

    const recentSlot = Number(await banksClient.getSlot());
    const [createLutIx, lutAddress] =
      AddressLookupTableProgram.createLookupTable({
        authority: liquidator.wallet.publicKey,
        payer: liquidator.wallet.publicKey,
        recentSlot: recentSlot - 1,
      });
    let createLutTx = new Transaction().add(createLutIx);
    createLutTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    createLutTx.sign(liquidator.wallet);
    await banksClient.processTransaction(createLutTx);

    let extendLutTx1 = new Transaction().add(
      AddressLookupTableProgram.extendLookupTable({
        authority: liquidator.wallet.publicKey,
        payer: liquidator.wallet.publicKey,
        lookupTable: lutAddress,
        addresses: remainingAccounts.flat().slice(0, 20),
      })
    );
    extendLutTx1.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    extendLutTx1.sign(liquidator.wallet);
    await banksClient.processTransaction(extendLutTx1);

    let extendLutTx2 = new Transaction().add(
      AddressLookupTableProgram.extendLookupTable({
        authority: liquidator.wallet.publicKey,
        payer: liquidator.wallet.publicKey,
        lookupTable: lutAddress,
        addresses: remainingAccounts.flat().slice(20),
      })
    );
    extendLutTx2.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    extendLutTx2.sign(liquidator.wallet);
    await banksClient.processTransaction(extendLutTx2);

    // We must advance the bankrun slot to allow the lut to activate
    const ONE_MINUTE = 60;
    const slotsToAdvance = ONE_MINUTE * 0.4;
    let { epoch: _, slot } = await getEpochAndSlot(banksClient);
    bankrunContext.warpToSlot(BigInt(slot + slotsToAdvance));

    if (verbose) {
      console.log("LUT key: " + lutAddress.toString());
    }

    const [liqRecordKey] = deriveLiquidationRecord(
      bankrunProgram.programId,
      liquidateeAccount
    );

    let tx = new Transaction();
    tx.add(
      await initLiquidationRecordIx(liquidator.mrgnBankrunProgram, {
        marginfiAccount: liquidateeAccount,
        feePayer: liquidator.wallet.publicKey,
        // liquidationRecord: liqRecord,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(liquidator.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
      await startLiquidationIx(liquidator.mrgnBankrunProgram, {
        marginfiAccount: liquidateeAccount,
        // liquidationRecord: liqRecord,
        liquidationReceiver: liquidator.wallet.publicKey,
        remaining: composeRemainingAccounts(remainingAccounts),
      }),
      await withdrawIx(liquidator.mrgnBankrunProgram, {
        marginfiAccount: liquidateeAccount,
        bank: banks[0],
        tokenAccount: liquidator.lstAlphaAccount,
        remaining: composeRemainingAccounts(remainingAccounts),
        amount: new BN(1),
      }),
      await repayIx(liquidator.mrgnBankrunProgram, {
        marginfiAccount: liquidateeAccount,
        bank: banks[MAX_BALANCES - 1],
        tokenAccount: liquidator.lstAlphaAccount,
        remaining: composeRemainingAccounts(remainingAccounts),
        amount: new BN(1),
      }),
      await endLiquidationIx(liquidator.mrgnBankrunProgram, {
        marginfiAccount: liquidateeAccount,
        remaining: composeRemainingAccounts(remainingAccounts),
      })
    );
    const blockhash = await getBankrunBlockhash(bankrunContext);
    const lutRaw = await banksClient.getAccount(lutAddress);
    const lutState = AddressLookupTableAccount.deserialize(lutRaw.data);
    const lutAccount = new AddressLookupTableAccount({
      key: lutAddress,
      state: lutState,
    });
    const messageV0 = new TransactionMessage({
      payerKey: liquidator.wallet.publicKey,
      recentBlockhash: blockhash,
      instructions: [...tx.instructions],
    }).compileToV0Message([lutAccount]);
    const versionedTx = new VersionedTransaction(messageV0);
    versionedTx.sign([liquidator.wallet]);
    await banksClient.processTransaction(versionedTx);

    const record = await bankrunProgram.account.liquidationRecord.fetch(
      liqRecordKey
    );

    // TODO assert various properties of the liquidation record...
  });

  // TODO try these with switchboard oracles.
});
