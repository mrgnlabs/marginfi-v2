import { BN } from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import {
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  EMODE_INIT_RATE_LST_TO_LST,
  EMODE_MAINT_RATE_LST_TO_LST,
  EMODE_INIT_RATE_SOL_TO_LST,
  EMODE_MAINT_RATE_SOL_TO_LST,
  EMODE_SEED,
  emodeAdmin,
  emodeGroup,
  oracles,
  users,
  verbose,
} from "./rootHooks";
import { deriveBankWithSeed } from "./utils/pdas";
import {
  bigNumberToWrappedI80F48,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import { assertBankrunTxFailed } from "./utils/genericTests";
import { USER_ACCOUNT_E } from "./utils/mocks";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import {
  CONF_INTERVAL_MULTIPLE,
  EMODE_APPLIES_TO_ISOLATED,
  EMODE_LST_TAG,
  EMODE_SOL_TAG,
  HEALTH_CACHE_HEALTHY,
  newEmodeEntry,
} from "./utils/types";
import {
  depositIx,
  borrowIx,
  liquidateIx,
  healthPulse,
  repayIx,
} from "./utils/user-instructions";
import { configBankEmode } from "./utils/group-instructions";
import { dumpBankrunLogs } from "./utils/tools";
import { assert } from "chai";
import { bytesToF64 } from "./utils/tools";

const seed = new BN(EMODE_SEED);
let usdcBank: PublicKey;
let stableBank: PublicKey;
let solBank: PublicKey;
let lstABank: PublicKey;
let lstBBank: PublicKey;

/** USDC funding for the liquidator (user 2) */
const liquidator_usdc: number = 10;
/** SOL funding for the liquidator (user 2) */
const liquidator_sol: number = 0.1;

const REDUCED_INIT_SOL_LST_RATE = 0.85;
const REDUCED_MAINT_SOL_LST_RATE = 0.9;

describe("Emode liquidation", () => {
  before(async () => {
    [usdcBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      seed
    );
    [stableBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      seed.addn(1)
    );
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
    [lstBBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      seed.addn(1)
    );
  });

  it("(user 2 aka liquidator) Deposits SOL and USDC to operate as a liquidator", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT_E);

    let tx = new Transaction().add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: usdcBank,
        tokenAccount: user.usdcAccount,
        amount: new BN(liquidator_usdc * 10 ** ecosystem.usdcDecimals),
        depositUpToLimit: false,
      }),
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: solBank,
        tokenAccount: user.wsolAccount,
        amount: new BN(liquidator_sol * 10 ** ecosystem.wsolDecimals),
        depositUpToLimit: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);
  });

  // Note: in this test, as in most real-world conditions, the liquidator is not optimizing their
  // liabilities to take advantage of emode. They have a variety of liabilities that they obtain
  // when liquidating positions, and typically close/offload these quickly. Here we pretend the
  // liquidator got some "stable" in a previous liquidation.
  it("(liquidator) borrows a trivial amount of stable to mock normal operation", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT_E);

    let tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: stableBank,
        tokenAccount: user.usdcAccount,
        remaining: [
          usdcBank,
          oracles.usdcOracle.publicKey,
          solBank,
          oracles.wsolOracle.publicKey,
          stableBank,
          oracles.usdcOracle.publicKey,
        ],
        amount: new BN(0.0001 * 10 ** ecosystem.usdcDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    let userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const cacheAfter = userAcc.healthCache;
    const assetValue = wrappedI80F48toBigNumber(cacheAfter.assetValue);
    const liabValue = wrappedI80F48toBigNumber(cacheAfter.liabilityValue);
    const aValMaint = wrappedI80F48toBigNumber(cacheAfter.assetValueMaint);
    const lValMaint = wrappedI80F48toBigNumber(cacheAfter.liabilityValueMaint);
    if (verbose) {
      console.log("---liquidator health state---");
      console.log("asset value: " + assetValue.toString());
      console.log("liab value: " + liabValue.toString());
      console.log("asset value (maint): " + aValMaint.toString());
      console.log("liab value (maint): " + lValMaint.toString());
      console.log("prices: ");
      for (let i = 0; i < cacheAfter.prices.length; i++) {
        const price = bytesToF64(cacheAfter.prices[i]);
        if (price != 0) {
          console.log(" [" + i + "] " + price);
        }
      }
    }
  });

  // Note: excluding emode, user 0 is unhealthy. Any liquidator that does not yet account for emode
  // will try to do this repeatedly and fail.
  it("(liquidator) Tries to liquidate user 0 with emode in effect - can't liquidate", async () => {
    const liquidatee = users[0];
    const liquidator = users[2];

    const assetBankKey = solBank;
    const liabilityBankKey = lstABank;
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_E);
    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT_E);

    let tx = new Transaction().add(
      await liquidateIx(liquidator.mrgnBankrunProgram, {
        assetBankKey,
        liabilityBankKey,
        liquidatorMarginfiAccount: liquidatorAccount,
        liquidateeMarginfiAccount: liquidateeAccount,
        remaining: [
          oracles.wsolOracle.publicKey, // asset oracle
          oracles.pythPullLst.publicKey, // liab oracle

          // liquidator accounts
          usdcBank,
          oracles.usdcOracle.publicKey,
          solBank,
          oracles.wsolOracle.publicKey,
          stableBank,
          oracles.usdcOracle.publicKey,
          // Note: these accounts would be needed if the LST A position was created on the
          // liquidator due to the liablity repayment
          lstABank,
          oracles.pythPullLst.publicKey,

          // liquidatee accounts
          solBank,
          oracles.wsolOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
        ],
        amount: new BN(0.001 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(liquidator.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    // 6068 (HealthyAccount)
    assertBankrunTxFailed(result, 6068);
  });

  // Note: In production, reducing Emode weights is at least as risky as reducing regular weights,
  // which is done rarely or never because it can trigger user liquidations. In rare instances where
  // this must be done outside for security concerns or assets in freefall, it should be done
  // carefully and slowly!
  it("(emode admin) Reduces LST A emode settings", async () => {
    let tx = new Transaction().add(
      await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
        bank: lstABank,
        tag: EMODE_LST_TAG,
        entries: [
          newEmodeEntry(
            EMODE_SOL_TAG,
            EMODE_APPLIES_TO_ISOLATED,
            // Here SOL borrowing power is drastically reduced
            bigNumberToWrappedI80F48(REDUCED_INIT_SOL_LST_RATE),
            bigNumberToWrappedI80F48(REDUCED_MAINT_SOL_LST_RATE)
          ),
          newEmodeEntry(
            EMODE_LST_TAG,
            EMODE_APPLIES_TO_ISOLATED,
            bigNumberToWrappedI80F48(EMODE_INIT_RATE_LST_TO_LST),
            bigNumberToWrappedI80F48(EMODE_MAINT_RATE_LST_TO_LST)
          ),
        ],
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(emodeAdmin.wallet);
    await banksClient.processTransaction(tx);
  });

  // Note: The health cache shows the price for init (borrowing) purposes, the "actual" price
  // (maint) uses `OraclePriceType::RealTime` and applies the confidence interval discount! So
  // instead of $150, the "actual" price of the collateral for liquidation purposes is $146.82
  // (150 * (1 - 1 * 0.0212))

  // TODO look into above, is this a footgun with assets that have broad confidence bands?

  // * SOL is worth $146.82 (see above for confidence discount)
  // * Liquidator will claim .1 sol worth ~= $14.682 (this is really $15 with conf discount)
  // * We expect to repay: .1 * (1 - 0.025) * 146.82 = $14.31495 (worth of LST)
  // * Liquidatee will receive: .1 * (1 - 0.025- 0.025) * 146.82 = $13.9479 (worth of LST)

  // In terms of what we actually see in the health pulse:
  // * Because liquidator has other borrows, they get no emode benefit on the sol they obtained.
  //   The SOL bank's actual asset weight is 0.5, so Liquidator's asset value increases by
  //   ($15 * 0.5) = $7.5
  // * The liability weight is 100%, so liquidator repays ($14.31495 * 1) = $14.31495
  // * Liquidatee loses the same asset amount, but WITH an emode benefit, so liquidatee sees a
  //   drop of ($14.682 * 0.85) = $12.75 and a reduction of $13.9479 in debt

  // In health terms the liquidator has lost money! In real terms the liquidator has gained $
  // value $15 - $14.31495 = $0.68505
  it("(liquidator) Liquidates user 0 after emode reduced - happy path", async () => {
    const liquidatee = users[0];
    const liquidator = users[2];

    const assetBankKey = solBank;
    const liabilityBankKey = lstABank;
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_E);
    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT_E);

    const liqAccountDataBefore = await processHealthPulse(
      liquidator,
      liquidatorAccount,
      [
        usdcBank,
        oracles.usdcOracle.publicKey,
        solBank,
        oracles.wsolOracle.publicKey,
        stableBank,
        oracles.usdcOracle.publicKey,
        // Note: the LST A liability position doesn't exist yet
      ]
    );
    const liqHealthCacheBefore = liqAccountDataBefore.healthCache;
    if (verbose) {
      logHealthCache("liquidator health state before", liqHealthCacheBefore);
    }

    const leeAccountDataBefore = await processHealthPulse(
      liquidatee,
      liquidateeAccount,
      [
        solBank,
        oracles.wsolOracle.publicKey,
        lstABank,
        oracles.pythPullLst.publicKey,
      ]
    );
    const leeHealthCacheBefore = leeAccountDataBefore.healthCache;
    if (verbose) {
      logHealthCache("liquidatee health state before", leeHealthCacheBefore);
    }

    let tx = new Transaction().add(
      await liquidateIx(liquidator.mrgnBankrunProgram, {
        assetBankKey,
        liabilityBankKey,
        liquidatorMarginfiAccount: liquidatorAccount,
        liquidateeMarginfiAccount: liquidateeAccount,
        remaining: [
          oracles.wsolOracle.publicKey, // asset oracle
          oracles.pythPullLst.publicKey, // liab oracle

          // liquidator accounts
          usdcBank,
          oracles.usdcOracle.publicKey,
          solBank,
          oracles.wsolOracle.publicKey,
          stableBank,
          oracles.usdcOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,

          // liquidatee accounts
          solBank,
          oracles.wsolOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
        ],
        amount: new BN(0.1 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(liquidator.wallet);
    await banksClient.processTransaction(tx);

    const liqAccountData = await processHealthPulse(
      liquidator,
      liquidatorAccount,
      [
        usdcBank,
        oracles.usdcOracle.publicKey,
        solBank,
        oracles.wsolOracle.publicKey,
        stableBank,
        oracles.usdcOracle.publicKey,
        lstABank,
        oracles.pythPullLst.publicKey,
      ]
    );
    const liqHealthCache = liqAccountData.healthCache;
    if (verbose) {
      logHealthCache("liquidator health state after", liqHealthCache);
    }

    const leeAccountData = await processHealthPulse(
      liquidatee,
      liquidateeAccount,
      [
        solBank,
        oracles.wsolOracle.publicKey,
        lstABank,
        oracles.pythPullLst.publicKey,
      ]
    );
    const leeHealthCache = leeAccountData.healthCache;
    if (verbose) {
      logHealthCache("liquidatee health state after", leeHealthCache);
    }

    const liqAvBefore = wrappedI80F48toBigNumber(
      liqHealthCacheBefore.assetValue
    ).toNumber();
    const liqAvAfter = wrappedI80F48toBigNumber(
      liqHealthCache.assetValue
    ).toNumber();
    const liqLvBefore = wrappedI80F48toBigNumber(
      liqHealthCacheBefore.liabilityValue
    ).toNumber();
    const liqLvAfter = wrappedI80F48toBigNumber(
      liqHealthCache.liabilityValue
    ).toNumber();

    const leeAvBefore = wrappedI80F48toBigNumber(
      leeHealthCacheBefore.assetValue
    ).toNumber();
    const leeAvAfter = wrappedI80F48toBigNumber(
      leeHealthCache.assetValue
    ).toNumber();
    const leeLvBefore = wrappedI80F48toBigNumber(
      leeHealthCacheBefore.liabilityValue
    ).toNumber();
    const leeLvAfter = wrappedI80F48toBigNumber(
      leeHealthCache.liabilityValue
    ).toNumber();

    assert.approximately(liqAvAfter - liqAvBefore, 7.5, 0.001);
    assert.approximately(liqLvAfter - liqLvBefore, 14.31495, 0.001);
    assert.approximately(leeAvAfter - leeAvBefore, -12.75, 0.001);
    assert.approximately(leeLvAfter - leeLvBefore, -13.9479, 0.001);
  });

  // This test demonstrates a new footgun that liquidators have to watch out for. On the user's
  // portfolio, due to emode, a position might be more valuable than when the liquidator acquires
  // it, which can make their account unhealthy and cause liquidation to fail.
  //
  // Here the liquidator has $20 in collateral at the end of the previous test and ~$14.31 in
  // liabilties. Repeating the 7.5 and 14.31495 repayment above, the liquidator would end up with 20
  // + 7.5 = $27.5 in assets and 14.31495 + 14.31495 = $28.62 in liabilities, so the liquidator has
  // put themselves in an unhealthy state!
  it("(liquidator) renders their own account unhealthy due to liquidation - should fail", async () => {
    const liquidatee = users[0];
    const liquidator = users[2];

    const assetBankKey = solBank;
    const liabilityBankKey = lstABank;
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_E);
    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT_E);

    let tx = new Transaction().add(
      await liquidateIx(liquidator.mrgnBankrunProgram, {
        assetBankKey,
        liabilityBankKey,
        liquidatorMarginfiAccount: liquidatorAccount,
        liquidateeMarginfiAccount: liquidateeAccount,
        remaining: [
          oracles.wsolOracle.publicKey, // asset oracle
          oracles.pythPullLst.publicKey, // liab oracle

          // liquidator accounts
          usdcBank,
          oracles.usdcOracle.publicKey,
          solBank,
          oracles.wsolOracle.publicKey,
          stableBank,
          oracles.usdcOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,

          // liquidatee accounts
          solBank,
          oracles.wsolOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
        ],
        amount: new BN(0.1 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(liquidator.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    // 6009 (RiskEngineInitRejected.)
    assertBankrunTxFailed(result, 6009);
  });

  it("(liquidator) repays their trivial stable position - now has an emode benefit", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT_E);

    let tx = new Transaction().add(
      await repayIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: stableBank,
        tokenAccount: user.usdcAccount,
        repayAll: true,
        remaining: [
          usdcBank,
          oracles.usdcOracle.publicKey,
          solBank,
          oracles.wsolOracle.publicKey,
          stableBank,
          oracles.usdcOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
        ],
        amount: new BN(0.0001 * 10 ** ecosystem.usdcDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const userAcc = await processHealthPulse(user, userAccount, [
      usdcBank,
      oracles.usdcOracle.publicKey,
      solBank,
      oracles.wsolOracle.publicKey,
      // Note: stable is now closed
      lstABank,
      oracles.pythPullLst.publicKey,
    ]);

    const cacheAfter = userAcc.healthCache;
    const assetValue = wrappedI80F48toBigNumber(cacheAfter.assetValue);
    const liabValue = wrappedI80F48toBigNumber(cacheAfter.liabilityValue);
    const aValMaint = wrappedI80F48toBigNumber(cacheAfter.assetValueMaint);
    const lValMaint = wrappedI80F48toBigNumber(cacheAfter.liabilityValueMaint);
    if (verbose) {
      console.log("---liquidator health state---");
      console.log("asset value: " + assetValue.toString());
      console.log("liab value: " + liabValue.toString());
      console.log("asset value (maint): " + aValMaint.toString());
      console.log("liab value (maint): " + lValMaint.toString());
      console.log("prices: ");
      for (let i = 0; i < cacheAfter.prices.length; i++) {
        const price = bytesToF64(cacheAfter.prices[i]);
        if (price != 0) {
          console.log(" [" + i + "] " + price);
        }
      }
    }
  });

  // Completing the possible footguns, this is a somewhat dangerous state for liquidators, since the
  // liquidator may now not be able to consume any position that breaks its own emode benefit. Since
  // most liquidators quickly repay debts and convert them back into a preferred currency, this is
  // probably not an issue for most liquidators, but that those hold balances for longer should be
  // aware of the possible footgun here.
  it("(liquidator) can now liquidate the position due to its own emode benefit!", async () => {
    const liquidatee = users[0];
    const liquidator = users[2];

    const assetBankKey = solBank;
    const liabilityBankKey = lstABank;
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_E);
    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT_E);

    let tx = new Transaction().add(
      await liquidateIx(liquidator.mrgnBankrunProgram, {
        assetBankKey,
        liabilityBankKey,
        liquidatorMarginfiAccount: liquidatorAccount,
        liquidateeMarginfiAccount: liquidateeAccount,
        remaining: [
          oracles.wsolOracle.publicKey, // asset oracle
          oracles.pythPullLst.publicKey, // liab oracle

          // liquidator accounts
          usdcBank,
          oracles.usdcOracle.publicKey,
          solBank,
          oracles.wsolOracle.publicKey,
          // Note: stable bank is closed
          lstABank,
          oracles.pythPullLst.publicKey,

          // liquidatee accounts
          solBank,
          oracles.wsolOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
        ],
        amount: new BN(0.1 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(liquidator.wallet);
    await banksClient.processTransaction(tx);

    const userAcc = await processHealthPulse(liquidator, liquidatorAccount, [
      usdcBank,
      oracles.usdcOracle.publicKey,
      solBank,
      oracles.wsolOracle.publicKey,
      // Note: stable is now closed
      lstABank,
      oracles.pythPullLst.publicKey,
    ]);

    const cacheAfter = userAcc.healthCache;
    const assetValue = wrappedI80F48toBigNumber(cacheAfter.assetValue);
    const liabValue = wrappedI80F48toBigNumber(cacheAfter.liabilityValue);
    if (verbose) {
      console.log("---liquidator health state---");
      console.log("asset value: " + assetValue.toString());
      console.log("liab value: " + liabValue.toString());
      console.log("prices: ");
      for (let i = 0; i < cacheAfter.prices.length; i++) {
        const price = bytesToF64(cacheAfter.prices[i]);
        if (price != 0) {
          console.log(" [" + i + "] " + price);
        }
      }
    }
  });

  const processHealthPulse = async (
    user: { mrgnBankrunProgram: any; wallet: any },
    userAccount: PublicKey,
    remaining: Array<PublicKey>
  ) => {
    const tx = new Transaction().add(
      await healthPulse(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        remaining,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);
    return bankrunProgram.account.marginfiAccount.fetch(userAccount);
  };

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
});
