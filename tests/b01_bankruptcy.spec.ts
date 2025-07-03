import { BN, Wallet } from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  PublicKey,
  SystemProgram,
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
  verbose,
  globalProgramAdmin,
} from "./rootHooks";
import { accrueInterest, configureBank } from "./utils/group-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { assert } from "chai";
import {
  emptyBankConfigOptRaw,
  HEALTH_CACHE_ENGINE_OK,
  HEALTH_CACHE_HEALTHY,
  HEALTH_CACHE_ORACLE_OK,
} from "./utils/types";
import {
  borrowIx,
  composeRemainingAccounts,
  depositIx,
  healthPulse,
  withdrawIx,
} from "./utils/user-instructions";
import {
  bigNumberToWrappedI80F48,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import { Clock } from "solana-bankrun";
import { genericMultiBankTestSetup } from "./genericSetups";
import { assertBankrunTxFailed } from "./utils/genericTests";
import { bytesToF64, dumpAccBalances, dumpBankrunLogs } from "./utils/tools";
import { initOrUpdatePriceUpdateV2 } from "./utils/pyth-pull-mocks";

const USER_ACCOUNT_THROWAWAY = "throwaway_account1";
const ONE_YEAR_IN_SECONDS = 365 * 24 * 60 * 60;

let banks: PublicKey[] = [];

describe("Bank bankruptcy tests", () => {
  it("init group, init banks, and fund banks", async () => {
    const result = await genericMultiBankTestSetup(2, USER_ACCOUNT_THROWAWAY);
    banks = result.banks;
  });

  it("(admin) Seeds liquidity in both banks", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const amount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);

    const tx = new Transaction();
    for (const bank of banks) {
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
    await banksClient.processTransaction(tx);
  });

  it("(admin) Sets both banks' asset weights to 1", async () => {
    let config = emptyBankConfigOptRaw();
    banks[0];
    config.assetWeightInit = bigNumberToWrappedI80F48(1); // 100%
    config.assetWeightMaint = bigNumberToWrappedI80F48(1); // 100%

    let tx = new Transaction().add(
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: banks[0],
        bankConfigOpt: config,
      }),
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: banks[1],
        bankConfigOpt: config,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
  });

  it("(user 0) Borrows B (bank 1) against A (bank 0)", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const depositAmount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
    const borrowAmount = new BN(90 * 10 ** ecosystem.lstAlphaDecimals);

    const depositTx = new Transaction();
    depositTx.add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[0],
        tokenAccount: user.lstAlphaAccount,
        amount: depositAmount,
        depositUpToLimit: false,
      })
    );
    depositTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    depositTx.sign(user.wallet);
    await banksClient.processTransaction(depositTx);

    const remainingAccounts: PublicKey[][] = [];
    remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
    remainingAccounts.push([banks[1], oracles.pythPullLst.publicKey]);

    const borrowTx = new Transaction();
    borrowTx.add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }),
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[1],
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts(remainingAccounts),
        amount: borrowAmount,
      })
    );
    borrowTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    borrowTx.sign(user.wallet);
    await banksClient.processTransaction(borrowTx);
  });

  it("One year elapses", async () => {
    let now = Math.floor(Date.now() / 1000);
    const targetUnix = BigInt(now + ONE_YEAR_IN_SECONDS);

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

  it("(user 0 - permissionless) Accrues interest on bank 1", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);

    // refresh oracles
    let clock = await banksClient.getClock();
    let refresh = await initOrUpdatePriceUpdateV2(
      new Wallet(globalProgramAdmin.wallet),
      oracles.pythPullLstOracleFeed.publicKey,
      new BN(oracles.lstAlphaPrice * 10 ** oracles.lstAlphaDecimals),
      new BN(oracles.lstAlphaPrice * 10 ** oracles.lstAlphaDecimals * 0.02),
      new BN(oracles.lstAlphaPrice * 10 ** oracles.lstAlphaDecimals),
      new BN(oracles.lstAlphaPrice * 10 ** oracles.lstAlphaDecimals * 0.02),
      new BN(Number(clock.slot)),
      -oracles.lstAlphaDecimals,
      oracles.pythPullLst,
      undefined,
      bankrunContext,
      Number(clock.unixTimestamp)
    );

    const tx = new Transaction();
    tx.add(
      await accrueInterest(user.mrgnBankrunProgram, {
        bank: banks[1],
      }),
      // dummy ix to trick bankrun
      SystemProgram.transfer({
        fromPubkey: users[0].wallet.publicKey,
        toPubkey: bankrunProgram.provider.publicKey,
        lamports: 11,
      }),
      await healthPulse(user.mrgnProgram, {
        marginfiAccount: userAccount,
        remaining: composeRemainingAccounts([
          [banks[0], oracles.pythPullLst.publicKey],
          [banks[1], oracles.pythPullLst.publicKey],
        ]),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    dumpBankrunLogs(result);

    const bankAcc = await bankrunProgram.account.bank.fetch(banks[1]);
    let bankValues = {
      asset: wrappedI80F48toBigNumber(bankAcc.assetShareValue).toNumber(),
      liability: wrappedI80F48toBigNumber(
        bankAcc.liabilityShareValue
      ).toNumber(),
    };

    if (verbose) {
      console.log("Value per share after one year:");
      console.log(
        `  asset: ${bankValues.asset}, liab: ${bankValues.liability}`
      );
    }

    console.log("good bank (0): " + banks[0]);
    console.log("bankrupt bank (1) : " + banks[1]);
    const groupAdminAcc =
      await groupAdmin.mrgnBankrunProgram.account.marginfiAccount.fetch(
        groupAdmin.accounts.get(USER_ACCOUNT_THROWAWAY)
      );
    const user0Acc =
      await groupAdmin.mrgnBankrunProgram.account.marginfiAccount.fetch(
        users[0].accounts.get(USER_ACCOUNT_THROWAWAY)
      );

    const bankValueMap = {};
    for (let pk of banks) {
      const acc = await bankrunProgram.account.bank.fetch(pk);
      bankValueMap[pk.toString()] = {
        asset: wrappedI80F48toBigNumber(acc.assetShareValue).toNumber(),
        liability: wrappedI80F48toBigNumber(acc.liabilityShareValue).toNumber(),
      };
    }

    dumpAccBalances(groupAdminAcc, bankValueMap);
    dumpAccBalances(user0Acc, bankValueMap);
    const cA = user0Acc.healthCache;
    const now = Date.now() / 1000;

    const assetValue = wrappedI80F48toBigNumber(cA.assetValue);
    const liabValue = wrappedI80F48toBigNumber(cA.liabilityValue);
    const aValMaint = wrappedI80F48toBigNumber(cA.assetValueMaint);
    const lValMaint = wrappedI80F48toBigNumber(cA.liabilityValueMaint);
    const aValEquity = wrappedI80F48toBigNumber(cA.assetValueEquity);
    const lValEquity = wrappedI80F48toBigNumber(cA.liabilityValueEquity);
    const flags = cA.flags;
    if (verbose) {
      console.log("---user health state---");
      const isHealthy = (flags & HEALTH_CACHE_HEALTHY) !== 0;
      const engineOk = (flags & HEALTH_CACHE_ENGINE_OK) !== 0;
      const oracleOk = (flags & HEALTH_CACHE_ORACLE_OK) !== 0;
      console.log("healthy: " + isHealthy);
      console.log("engine ok: " + engineOk);
      console.log("oracle ok: " + oracleOk);
      console.log("mrgn error: " + cA.mrgnErr);
      console.log("internal error: " + cA.internalErr);
      console.log("index of err:   " + cA.errIndex);
      console.log("bankrpt error: " + cA.internalBankruptcyErr);
      console.log("asset value: " + assetValue.toString());
      console.log("liab value: " + liabValue.toString());
      console.log("asset value (maint): " + aValMaint.toString());
      console.log("liab value (maint): " + lValMaint.toString());
      console.log("asset value (equity): " + aValEquity.toString());
      console.log("liab value equity): " + lValEquity.toString());
      console.log("prices: ");
      for (let i = 0; i < cA.prices.length; i++) {
        const price = bytesToF64(cA.prices[i]);
        if (price != 0) {
          console.log(" [" + i + "] " + price);
        }
      }
    }

    assert.isAbove(bankValues.liability, bankValues.asset);
  });

  const logHealthCache = (header: string, healthCache: any) => {
    const av = wrappedI80F48toBigNumber(healthCache.assetValue);
    const lv = wrappedI80F48toBigNumber(healthCache.liabilityValue);
    const aValMaint = wrappedI80F48toBigNumber(healthCache.assetValueMaint);
    const lValMaint = wrappedI80F48toBigNumber(healthCache.liabilityValueMaint);
    console.log(`---${header}---`);
    if (healthCache.flags & HEALTH_CACHE_HEALTHY) {
      console.log("**HEALTHY**");
    } else {
      console.log("**UNHEALTHY OR INVALID**");
    }
    console.log("asset value: " + av.toString());
    console.log("liab value: " + lv.toString());
    console.log("asset value (maint): " + aValMaint.toString());
    console.log("liab value (maint): " + lValMaint.toString());
    console.log("prices: ");
    healthCache.prices.forEach((priceWrapped: any, i: number) => {
      const price = bytesToF64(priceWrapped);
      if (price !== 0) {
        console.log(` [${i}] ${price}`);
      }
    });
    console.log("");
  };

  it("(admin) Tries to withdraw some B (bank 1 - now bankrupt) - should fail", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const amount = new BN(0.01 * 10 ** ecosystem.lstAlphaDecimals);

    const remainingAccounts: PublicKey[][] = [];
    remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
    remainingAccounts.push([banks[1], oracles.pythPullLst.publicKey]);

    const tx = new Transaction();
    tx.add(
      await withdrawIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[1],
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts(remainingAccounts),
        amount,
        withdrawAll: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    // IllegalUtilizationRatio
    assertBankrunTxFailed(result, "0x178a");
  });

  it("(user 0) Tries to borrow a little more B (bank 1 - now bankrupt) against A (bank 0) - should fail", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const borrowAmount = new BN(0.01 * 10 ** ecosystem.lstAlphaDecimals);

    const remainingAccounts: PublicKey[][] = [];
    remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
    remainingAccounts.push([banks[1], oracles.pythPullLst.publicKey]);

    const borrowTx = new Transaction();
    borrowTx.add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }),
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[1],
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts(remainingAccounts),
        amount: borrowAmount,
      })
    );
    borrowTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    borrowTx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(borrowTx);
    // IllegalUtilizationRatio
    assertBankrunTxFailed(result, "0x178a");
  });
});
