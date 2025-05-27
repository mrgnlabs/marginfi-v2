/*
Interest accrues when users interact with a bank (withdraw, deposit, etc). Here we demonstrate what
happens if nobody interacts with a bank vs if it gets frequent updates.

This test must run last in the bankrun suite because it advances a lot of time, generating a lot of
interest, which messes with other tests.

The test demonstrates that interest earned is slightly dependent on compounding schedule. 
*/
import { BN } from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  groupAdmin,
  bankrunContext,
  banksClient,
  bankrunProgram,
  verbose,
  ecosystem,
  oracles,
  users,
} from "./rootHooks";
import { arrueInterest as acrrueInterest } from "./utils/group-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { assert } from "chai";
import {
  borrowIx,
  composeRemainingAccounts,
  depositIx,
} from "./utils/user-instructions";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { genericMultiBankTestSetup } from "./genericSetups";
import { Clock } from "solana-bankrun";

/** Banks in this test use a "random" seed so their key is non-deterministic. */
let startingSeed: number;

const USER_ACCOUNT_THROWAWAY = "throwaway_account2";
const ONE_WEEK_IN_SECONDS = 7 * 24 * 60 * 60;

let banks: PublicKey[] = [];
let throwawayGroup: Keypair;

const depositAmount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
const borrowAmount = new BN(30 * 10 ** ecosystem.lstAlphaDecimals);

describe("Compound interest demonstration", () => {
  it("init group, init banks, and fund banks", async () => {
    const result = await genericMultiBankTestSetup(4, USER_ACCOUNT_THROWAWAY);
    startingSeed = result.startingSeed;
    banks = result.banks;
    throwawayGroup = result.throwawayGroup;
  });

  it("(admin) Seeds liquidity in all banks", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    // Note: This is about the max per TX without using LUTs.
    const depositsPerTx = 5;

    for (let i = 0; i < banks.length; i += depositsPerTx) {
      const chunk = banks.slice(i, i + depositsPerTx);
      const tx = new Transaction();
      let k = 0;
      for (const bank of chunk) {
        tx.add(
          await depositIx(user.mrgnBankrunProgram, {
            marginfiAccount: userAccount,
            bank,
            tokenAccount: user.lstAlphaAccount,
            amount: depositAmount,
            depositUpToLimit: false,
          })
        );
        if (verbose) {
          console.log(
            "seed bank " + k + " with liquidity " + depositAmount.toNumber()
          );
        }
        k++;
      }
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(user.wallet);
      await banksClient.tryProcessTransaction(tx);
    }
  });

  it("(user 0) Borrows from banks 1-3 against bank 0 to generate interest", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    // enough to borrow as much as we want...
    const depositAmt = depositAmount.muln(3);

    const tx = new Transaction();
    tx.add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[0],
        tokenAccount: user.lstAlphaAccount,
        amount: depositAmt,
        depositUpToLimit: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.tryProcessTransaction(tx);

    if (verbose) {
      console.log("deposit bank 0 liquidity " + depositAmt.toNumber());
    }

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
      await banksClient.processTransaction(tx);

      if (verbose) {
        console.log("borrow " + borrowAmount.toNumber() + " from bank " + i);
      }
    }
  });

  let bankValuesInitial: number[] = [];
  it("grab the value per share at the start", async () => {
    bankValuesInitial = await Promise.all(
      [0, 1, 2, 3].map((i) =>
        bankrunProgram.account.bank
          .fetch(banks[i])
          .then((bankAcc) =>
            wrappedI80F48toBigNumber(bankAcc.assetShareValue).toNumber()
          )
      )
    );
  });

  it("One week elapses", async () => {
    let now = Math.floor(Date.now() / 1000);
    const targetUnix = BigInt(now + ONE_WEEK_IN_SECONDS);

    // Construct a new Clock; we only care about the unixTimestamp field here.
    const newClock = new Clock(
      0n, // slot
      0n, // epochStartTimestamp
      0n, // epoch
      0n, // leaderScheduleEpoch
      targetUnix
    );

    bankrunContext.setClock(newClock);
  });

  it("(user 0 - permissionless) Accrues interest on bank 1 ONLY", async () => {
    const user = users[0];

    const tx = new Transaction();
    tx.add(
      await acrrueInterest(user.mrgnBankrunProgram, {
        bank: banks[1],
      }),
      // dummy tx to trick bankrun
      SystemProgram.transfer({
        fromPubkey: users[0].wallet.publicKey,
        toPubkey: bankrunProgram.provider.publicKey,
        lamports: 41,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    let bankValuesOneWeek: { asset: number; liability: number }[] = [];
    bankValuesOneWeek = await Promise.all(
      [0, 1, 2, 3].map(async (i) => {
        const bankAcc = await bankrunProgram.account.bank.fetch(banks[i]);
        return {
          asset: wrappedI80F48toBigNumber(bankAcc.assetShareValue).toNumber(),
          liability: wrappedI80F48toBigNumber(
            bankAcc.liabilityShareValue
          ).toNumber(),
        };
      })
    );

    if (verbose) {
      console.log("Value per share after first week:");
      bankValuesOneWeek.forEach(({ asset, liability }, idx) =>
        console.log(`  Bank ${idx}: asset: ${asset}, liab: ${liability}`)
      );
    }
    assert.notEqual(bankValuesOneWeek[1].asset, bankValuesInitial[1]);
    // No change to the other two banks...
    assert.equal(bankValuesOneWeek[2].asset, bankValuesInitial[2]);
    assert.equal(bankValuesOneWeek[3].asset, bankValuesInitial[3]);
  });

  it("(user 0 - permissionless) Accrues on bank 1, weekly, for 51 more weeks (1 year total)", async () => {
    const user = users[0];

    let prevAsset = bankValuesInitial[1];

    for (let week = 1; week <= 52; week++) {
      const now = Math.floor(Date.now() / 1000);
      const targetUnix = BigInt(now + ONE_WEEK_IN_SECONDS * week);
      const newClock = new Clock(
        0n, // slot
        0n, // epochStartTimestamp
        0n, // epoch
        0n, // leaderScheduleEpoch
        targetUnix
      );
      bankrunContext.setClock(newClock);

      const tx = new Transaction();
      tx.add(
        await acrrueInterest(user.mrgnBankrunProgram, {
          bank: banks[1],
        }),
        // dummy tx to trick bankrun
        SystemProgram.transfer({
          fromPubkey: users[0].wallet.publicKey,
          toPubkey: bankrunProgram.provider.publicKey,
          lamports: 42 + week,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(user.wallet);
      await banksClient.processTransaction(tx);

      let bankValuesNWeek: { asset: number; liability: number }[] = [];
      bankValuesNWeek = await Promise.all(
        [0, 1, 2, 3].map(async (i) => {
          const bankAcc = await bankrunProgram.account.bank.fetch(banks[i]);
          return {
            asset: wrappedI80F48toBigNumber(bankAcc.assetShareValue).toNumber(),
            liability: wrappedI80F48toBigNumber(
              bankAcc.liabilityShareValue
            ).toNumber(),
          };
        })
      );

      const currAsset = bankValuesNWeek[1].asset;
      const weeklyRate = currAsset / prevAsset - 1;
      const annualizedRate = weeklyRate * 52;
      prevAsset = currAsset;

      if (verbose) {
        const b = await bankrunProgram.account.bank.fetch(banks[1]);
        const utilActual =
          ((wrappedI80F48toBigNumber(b.totalLiabilityShares).toNumber() *
            wrappedI80F48toBigNumber(b.liabilityShareValue).toNumber()) /
            wrappedI80F48toBigNumber(b.totalAssetShares).toNumber()) *
          wrappedI80F48toBigNumber(b.assetShareValue).toNumber();
        // print every 4 weeks to avoid spaming
        if (week % 4 == 0) {
          console.log(
            ` week ${week} Bank 1: asset: ${bankValuesNWeek[1].asset}, liab: ${bankValuesNWeek[1].liability} util: ${utilActual}`
          );
          console.log(`   annualized rate: ${annualizedRate}`);
        }
      }
      assert.notEqual(bankValuesNWeek[1].asset, bankValuesInitial[1]);
      // No change to the other two banks...
      assert.equal(bankValuesNWeek[2].asset, bankValuesInitial[2]);
      assert.equal(bankValuesNWeek[3].asset, bankValuesInitial[3]);
    }
  });

  it("(user 0 - permissionless) Accrues interest on bank 2/3 after one year", async () => {
    const user = users[0];

    const tx = new Transaction();
    tx.add(
      await acrrueInterest(user.mrgnBankrunProgram, {
        bank: banks[2],
      })
    );
    tx.add(
      await acrrueInterest(user.mrgnBankrunProgram, {
        bank: banks[3],
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    let bankValuesOneYear: { asset: number; liability: number }[] = [];
    bankValuesOneYear = await Promise.all(
      [0, 1, 2, 3].map(async (i) => {
        const bankAcc = await bankrunProgram.account.bank.fetch(banks[i]);
        return {
          asset: wrappedI80F48toBigNumber(bankAcc.assetShareValue).toNumber(),
          liability: wrappedI80F48toBigNumber(
            bankAcc.liabilityShareValue
          ).toNumber(),
        };
      })
    );

    if (verbose) {
      console.log("Value per share after first accrue (1 year):");
      bankValuesOneYear.forEach(({ asset, liability }, idx) =>
        console.log(`  Bank ${idx}: asset: ${asset}, liab: ${liability}`)
      );
    }
    assert.notEqual(bankValuesOneYear[1].asset, bankValuesInitial[1]);
    assert.notEqual(bankValuesOneYear[2].asset, bankValuesInitial[2]);
    assert.notEqual(bankValuesOneYear[3].asset, bankValuesInitial[3]);
    // They are NOT the same, due to the power of compound interest...
    assert.notEqual(bankValuesOneYear[1].asset, bankValuesOneYear[2].asset);
    assert.notEqual(bankValuesOneYear[1].asset, bankValuesOneYear[3].asset);

    let bank = await bankrunProgram.account.bank.fetch(banks[1]);
    const util =
      wrappedI80F48toBigNumber(bank.totalLiabilityShares).toNumber() /
      wrappedI80F48toBigNumber(bank.totalAssetShares).toNumber();
    const utilActual =
      ((wrappedI80F48toBigNumber(bank.totalLiabilityShares).toNumber() *
        wrappedI80F48toBigNumber(bank.liabilityShareValue).toNumber()) /
        wrappedI80F48toBigNumber(bank.totalAssetShares).toNumber()) *
      wrappedI80F48toBigNumber(bank.assetShareValue).toNumber();
    //aka borrowAmount.toNumber() / depositAmount.toNumber();
    const optimalRate = wrappedI80F48toBigNumber(
      bank.config.interestRateConfig.optimalUtilizationRate
    ).toNumber();
    const platRate = wrappedI80F48toBigNumber(
      bank.config.interestRateConfig.plateauInterestRate
    ).toNumber();

    const baseRate = (util / optimalRate) * platRate;
    const lendingRate = util * baseRate;
    // Note: typically base * (1 + ir) + fixed, but tests have no fees to simplify.
    const borrowRate = baseRate;

    if (verbose) {
      // Gather all rate metrics into an object for pretty printing
      const rateMetrics: Record<string, number> = {
        "Utilization (shares only)": util,
        "Utilization (actual w/ share value)": utilActual,
        "Optimal Utilization Rate": optimalRate,
        "Plateau Interest Rate": platRate,
        "Base Rate": baseRate,
        "Lending Rate": lendingRate,
        "Borrow Rate": borrowRate,
      };

      // Calculate padding for alignment
      const labelWidth = Math.max(
        ...Object.keys(rateMetrics).map((k) => k.length)
      );

      console.log("🏷  Interest Rate Metrics:");
      for (const [label, value] of Object.entries(rateMetrics)) {
        console.log(`  ${label.padEnd(labelWidth)} : ${value}`);
      }
    }

    // Banks 2/3 got simple interest over 1 year...
    assert.approximately(
      bankValuesOneYear[2].asset,
      bankValuesInitial[2] * (1 + lendingRate),
      bankValuesOneYear[2].asset * 0.01 // 1%
    );
    assert.approximately(
      bankValuesOneYear[3].asset,
      bankValuesInitial[3] * (1 + lendingRate),
      bankValuesOneYear[3].asset
    );

    // Bank 1 earned interest compounded weekly...
    const periods = 52;
    const weeklyRate = lendingRate / periods;
    const apy = Math.pow(1 + weeklyRate, periods) - 1;
    if (verbose) {
      console.log("apy expected (1 year, compounding weekly): " + apy);
    }
    // Note that the simple compounding is quite far off: the lending rate changes week-to-week as
    // the actual utilization ratio (which INCLUDES the value/share) updates.
    assert.approximately(
      bankValuesOneYear[1].asset,
      bankValuesInitial[1] * (1 + apy),
      bankValuesOneYear[1].asset * 0.1
    );

    const relativeIncrease =
      ((bankValuesOneYear[1].asset - bankValuesOneYear[2].asset) /
        bankValuesOneYear[2].asset) *
      100;

    if (verbose) {
      console.log(
        `Weekly compounding yields ${relativeIncrease.toFixed(
          4
        )}% more than simple once annual compounding`
      );
    }

    const naiveValue = bankValuesInitial[1] * (1 + apy);
    const actualValue = bankValuesOneYear[1].asset;
    const increasevsNaiveApy = ((actualValue - naiveValue) / naiveValue) * 100;

    if (verbose) {
      console.log(
        `Weekly compounding yields ${increasevsNaiveApy.toFixed(
          4
        )}% more than naive weekly compounding`
      );
    }
  });
});
