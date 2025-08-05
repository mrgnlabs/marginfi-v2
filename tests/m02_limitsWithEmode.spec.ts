import { BN } from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  Keypair,
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import {
  groupAdmin,
  bankrunContext,
  banksClient,
  bankrunProgram,
  ecosystem,
  oracles,
  users,
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
} from "./utils/user-instructions";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { dumpAccBalances } from "./utils/tools";
import { genericMultiBankTestSetup } from "./genericSetups";

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

  // TODO try to liquidate using the new start/end liquidation approach. Withdraw/repay within the
  // same tx to validate that w thidraw/repay can fit even with a 16 position account.

  // TODO try these with switchboard oracles.
});
