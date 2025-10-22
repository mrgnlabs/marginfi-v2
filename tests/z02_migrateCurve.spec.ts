import { Transaction } from "@solana/web3.js";
import { Clock } from "solana-bankrun";
import { assert } from "chai";
import BigNumber from "bignumber.js";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import {
  PRE_MIGRATION_BANK_SAMPLE,
  bankrunContext,
  bankrunProgram,
  banksClient,
  users,
} from "./rootHooks";
import { accrueInterest, migrateCurve } from "./utils/group-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { getEpochAndSlot } from "./utils/stake-utils";
import { INTEREST_CURVE_SEVEN_POINT, aprToU32, utilToU32 } from "./utils/types";
import { assertI80F48Equal } from "./utils/genericTests";

const INTERVAL_SECONDS = 6 * 60 * 60; // six hours
const SLOT_DURATION_SECONDS = 2.5; // approximate solana slot duration used in other tests

const toBigNumber = (value: any): BigNumber => wrappedI80F48toBigNumber(value);

const advanceTime = async (seconds: number) => {
  const currentClock = await banksClient.getClock();
  const { epoch, slot } = await getEpochAndSlot(banksClient);
  const slotsToAdvance = Math.round(seconds / SLOT_DURATION_SECONDS);
  const newClock = new Clock(
    BigInt(slot + slotsToAdvance),
    0n,
    BigInt(epoch),
    0n,
    currentClock.unixTimestamp + BigInt(seconds)
  );
  bankrunContext.setClock(newClock);
};

const sendLegacyAccrual = async () => {
  const user = users[0];
  const tx = new Transaction();
  tx.add(await accrueInterest(user.mrgnBankrunProgram, { bank: PRE_MIGRATION_BANK_SAMPLE }));
  tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
  tx.feePayer = user.wallet.publicKey;
  tx.sign(user.wallet);
  await banksClient.processTransaction(tx);
};

const sendMigration = async () => {
  const user = users[0];
  const tx = new Transaction();
  tx.add(await migrateCurve(user.mrgnBankrunProgram, { bank: PRE_MIGRATION_BANK_SAMPLE }));
  tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
  tx.feePayer = user.wallet.publicKey;
  tx.sign(user.wallet);
  await banksClient.processTransaction(tx);
};

describe("Legacy bank curve migration", () => {
  let initialLiabilityShareValue: BigNumber;
  let postWarmupLiabilityShareValue: BigNumber;
  let postLegacyLiabilityShareValue: BigNumber;
  let postMigrationLiabilityShareValue: BigNumber;
  let postNewLiabilityShareValue: BigNumber;

  let optimalBefore: number;
  let plateauBefore: number;
  let maxRateBefore: number;

  it("captures the legacy curve configuration", async () => {
    const bankBefore = await bankrunProgram.account.bank.fetch(PRE_MIGRATION_BANK_SAMPLE);
    const ircBefore = bankBefore.config.interestRateConfig;

    assert.notEqual(ircBefore.curveType, INTEREST_CURVE_SEVEN_POINT);

    optimalBefore = toBigNumber(ircBefore.optimalUtilizationRate).toNumber();
    plateauBefore = toBigNumber(ircBefore.plateauInterestRate).toNumber();
    maxRateBefore = toBigNumber(ircBefore.maxInterestRate).toNumber();

    assert.isAbove(optimalBefore, 0, "expected a non-zero optimal utilization rate before migration");
    assert.isAbove(plateauBefore, 0, "expected a non-zero plateau interest rate before migration");
    assert.isAbove(maxRateBefore, 0, "expected a non-zero max interest rate before migration");

    initialLiabilityShareValue = toBigNumber(bankBefore.liabilityShareValue);
  });

  it("accrues interest using the legacy curve", async () => {
    await sendLegacyAccrual();
    const bankAfterWarmup = await bankrunProgram.account.bank.fetch(
      PRE_MIGRATION_BANK_SAMPLE
    );
    postWarmupLiabilityShareValue = toBigNumber(bankAfterWarmup.liabilityShareValue);
    assert.isTrue(
      postWarmupLiabilityShareValue.gte(initialLiabilityShareValue),
      "liability share value should not decrease after accrual"
    );
  });

  it("advances time and accrues again on the legacy curve", async () => {
    await advanceTime(INTERVAL_SECONDS);
    await sendLegacyAccrual();
    const bankAfterLegacyInterval = await bankrunProgram.account.bank.fetch(
      PRE_MIGRATION_BANK_SAMPLE
    );
    postLegacyLiabilityShareValue = toBigNumber(bankAfterLegacyInterval.liabilityShareValue);
    const legacyDelta = postLegacyLiabilityShareValue.minus(postWarmupLiabilityShareValue);
    assert.isTrue(legacyDelta.gt(0), "expected positive interest accrual before migration");
  });

  it("migrates the curve and validates the configuration", async () => {
    await sendMigration();
    const bankAfterMigration = await bankrunProgram.account.bank.fetch(
      PRE_MIGRATION_BANK_SAMPLE
    );
    postMigrationLiabilityShareValue = toBigNumber(bankAfterMigration.liabilityShareValue);

    const ircAfter = bankAfterMigration.config.interestRateConfig;

    assert.equal(ircAfter.curveType, INTEREST_CURVE_SEVEN_POINT);
    assertI80F48Equal(ircAfter.optimalUtilizationRate, 0);
    assertI80F48Equal(ircAfter.plateauInterestRate, 0);
    assertI80F48Equal(ircAfter.maxInterestRate, 0);
    assert.equal(ircAfter.zeroUtilRate, 0);

    const expectedUtil = utilToU32(optimalBefore);
    const expectedPlateauRate = aprToU32(plateauBefore);
    const expectedHundredRate = aprToU32(maxRateBefore);

    assert.approximately(
      ircAfter.points[0].util,
      expectedUtil,
      5,
      "first kink utilization should match migrated legacy optimal utilization"
    );
    assert.approximately(
      ircAfter.points[0].rate,
      expectedPlateauRate,
      5,
      "first kink rate should match migrated legacy plateau rate"
    );
    assert.approximately(
      ircAfter.hundredUtilRate,
      expectedHundredRate,
      5,
      "hundred percent utilization rate should match migrated legacy max interest rate"
    );
  });

  it("accrues interest with the migrated curve at a similar rate", async () => {
    const legacyDelta = postLegacyLiabilityShareValue.minus(postWarmupLiabilityShareValue);
    await advanceTime(INTERVAL_SECONDS);
    await sendLegacyAccrual();
    const bankAfterNewInterval = await bankrunProgram.account.bank.fetch(
      PRE_MIGRATION_BANK_SAMPLE
    );
    postNewLiabilityShareValue = toBigNumber(bankAfterNewInterval.liabilityShareValue);
    const migratedDelta = postNewLiabilityShareValue.minus(postMigrationLiabilityShareValue);

    assert.isTrue(migratedDelta.gt(0), "expected positive interest accrual after migration");

    const averageDelta = legacyDelta.plus(migratedDelta).dividedBy(2);
    const tolerance = averageDelta.abs().multipliedBy(0.05);
    const deltaDifference = legacyDelta.minus(migratedDelta).abs();

    assert.isTrue(
      deltaDifference.lte(tolerance.plus(new BigNumber("1e-12"))),
      "interest accrued after migration should remain within 5% of the legacy curve"
    );
  });
});
