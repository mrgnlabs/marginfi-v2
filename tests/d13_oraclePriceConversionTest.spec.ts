import { BN } from "@coral-xyz/anchor";
import {
  PublicKey,
  Transaction,
  Keypair,
  SystemProgram,
} from "@solana/web3.js";
import {
  ecosystem,
  driftAccounts,
  driftGroup,
  DRIFT_USDC_BANK,
  DRIFT_TOKENA_BANK,
  DRIFT_USDC_SPOT_MARKET,
  DRIFT_TOKENA_SPOT_MARKET,
  DRIFT_TOKENA_PULL_ORACLE,
  users,
  bankrunContext,
  bankrunProgram,
  driftBankrunProgram,
  oracles,
  banksClient,
  globalProgramAdmin,
} from "./rootHooks";
import { processBankrunTransaction } from "./utils/tools";
import { assertBNApproximately } from "./utils/genericTests";
import { makeDriftDepositIx } from "./utils/drift-instructions";
import {
  DRIFT_SCALED_BALANCE_DECIMALS,
  DRIFT_SPOT_CUMULATIVE_INTEREST_PRECISION,
  getSpotMarketAccount,
  makePulseHealthIx,
  USDC_MARKET_INDEX,
  TOKEN_A_MARKET_INDEX,
} from "./utils/drift-utils";
import { refreshPullOraclesBankrun } from "./utils/bankrun-oracles";
import { makeUpdateSpotMarketCumulativeInterestIx } from "./utils/drift-sdk";
import { accountInit } from "./utils/user-instructions";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { Clock } from "solana-bankrun";
import assert from "assert";

describe("d13: Oracle Price Conversion and Interest Tracking", () => {
  let user0Account: PublicKey;
  let user1Account: PublicKey;
  let driftUsdcBank: PublicKey;
  let driftTokenABank: PublicKey;

  before(async () => {
    driftUsdcBank = driftAccounts.get(DRIFT_USDC_BANK);
    driftTokenABank = driftAccounts.get(DRIFT_TOKENA_BANK);
  });

  it("Creates temporary-seed marginfi accounts for both users", async () => {
    for (let i = 0; i < 2; i++) {
      const tempSeed = Buffer.alloc(32);
      tempSeed.write(`oracle_test_user_${i}_`.padEnd(32, "0"));
      const accountKeypair = Keypair.fromSeed(tempSeed);

      const initAccountIx = await accountInit(users[i].mrgnBankrunProgram, {
        marginfiGroup: driftGroup.publicKey,
        marginfiAccount: accountKeypair.publicKey,
        authority: users[i].wallet.publicKey,
        feePayer: users[i].wallet.publicKey,
      });

      const tx = new Transaction().add(initAccountIx);
      await processBankrunTransaction(bankrunContext, tx, [
        users[i].wallet,
        accountKeypair,
      ]);

      users[i].accounts.set("d13_temp_account", accountKeypair.publicKey);

      if (i === 0) user0Account = accountKeypair.publicKey;
      else user1Account = accountKeypair.publicKey;
    }
  });

  let initialAssetShares0: BN;
  let initialAssetShares1: BN;
  let initialUsdcCumulativeInterest: BN;
  let initialTokenACumulativeInterest: BN;
  let finalUsdcCumulativeInterest: BN;
  let finalTokenACumulativeInterest: BN;

  it("Deposits initial amounts into Drift banks", async () => {
    const usdcDepositAmount = new BN(1 * 10 ** ecosystem.usdcDecimals);

    const depositIx0 = await makeDriftDepositIx(
      users[0].mrgnBankrunProgram,
      {
        marginfiAccount: user0Account,
        bank: driftUsdcBank,
        signerTokenAccount: users[0].usdcAccount,
      },
      usdcDepositAmount,
      USDC_MARKET_INDEX
    );

    await processBankrunTransaction(
      bankrunContext,
      new Transaction().add(depositIx0),
      [users[0].wallet]
    );

    const tokenADepositAmount = new BN(0.1 * 10 ** ecosystem.tokenADecimals);

    const depositIx1 = await makeDriftDepositIx(
      users[1].mrgnBankrunProgram,
      {
        marginfiAccount: user1Account,
        bank: driftTokenABank,
        signerTokenAccount: users[1].tokenAAccount,
        driftOracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE),
      },
      tokenADepositAmount,
      TOKEN_A_MARKET_INDEX
    );

    await processBankrunTransaction(
      bankrunContext,
      new Transaction().add(depositIx1),
      [users[1].wallet]
    );

    const marginfiAccount0 = await bankrunProgram.account.marginfiAccount.fetch(
      user0Account
    );
    const balance0 = marginfiAccount0.lendingAccount.balances.find(
      (b) => b.active === 1 && b.bankPk.equals(driftUsdcBank)
    );
    initialAssetShares0 = new BN(
      wrappedI80F48toBigNumber(balance0.assetShares).toString()
    );

    const marginfiAccount1 = await bankrunProgram.account.marginfiAccount.fetch(
      user1Account
    );
    const balance1 = marginfiAccount1.lendingAccount.balances.find(
      (b) => b.active === 1 && b.bankPk.equals(driftTokenABank)
    );
    initialAssetShares1 = new BN(
      wrappedI80F48toBigNumber(balance1.assetShares).toString()
    );

    const usdcSpotMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );
    const tokenASpotMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );

    initialUsdcCumulativeInterest = new BN(
      usdcSpotMarket.cumulativeDepositInterest.toString()
    );
    initialTokenACumulativeInterest = new BN(
      tokenASpotMarket.cumulativeDepositInterest.toString()
    );
  });

  it("Advances time by 30 days and tracks interest accrual", async () => {
    const currentClock = await banksClient.getClock();
    const currentSlot = Number(currentClock.slot);
    const currentTimestamp = Number(currentClock.unixTimestamp);

    const newSlot = currentSlot + 1;
    const newTimestamp = currentTimestamp + 30 * 86400;

    const newClock = new Clock(
      BigInt(newSlot),
      0n,
      currentClock.epoch,
      0n,
      BigInt(newTimestamp)
    );

    bankrunContext.setClock(newClock);

    const dummyTx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: globalProgramAdmin.wallet.publicKey,
        toPubkey: globalProgramAdmin.wallet.publicKey,
        lamports: 1,
      })
    );
    await processBankrunTransaction(bankrunContext, dummyTx, [
      globalProgramAdmin.wallet,
    ]);

    await refreshPullOraclesBankrun(oracles, bankrunContext, banksClient);

    const updateUsdcIx = await makeUpdateSpotMarketCumulativeInterestIx(
      driftBankrunProgram,
      {},
      USDC_MARKET_INDEX
    );

    const updateTokenAIx = await makeUpdateSpotMarketCumulativeInterestIx(
      driftBankrunProgram,
      { oracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE) },
      TOKEN_A_MARKET_INDEX
    );

    const updateTx = new Transaction().add(updateUsdcIx, updateTokenAIx);
    await processBankrunTransaction(bankrunContext, updateTx, [
      globalProgramAdmin.wallet,
    ]);

    const usdcSpotMarketAfter = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );
    const tokenASpotMarketAfter = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );

    finalUsdcCumulativeInterest = new BN(
      usdcSpotMarketAfter.cumulativeDepositInterest.toString()
    );
    finalTokenACumulativeInterest = new BN(
      tokenASpotMarketAfter.cumulativeDepositInterest.toString()
    );

    assert.ok(finalUsdcCumulativeInterest.gt(initialUsdcCumulativeInterest));
    assert.ok(
      finalTokenACumulativeInterest.gt(initialTokenACumulativeInterest)
    );

    const marginfiAccount0After =
      await bankrunProgram.account.marginfiAccount.fetch(user0Account);
    const balance0After = marginfiAccount0After.lendingAccount.balances.find(
      (b) => b.active === 1 && b.bankPk.equals(driftUsdcBank)
    );
    const assetShares0After = new BN(
      wrappedI80F48toBigNumber(balance0After.assetShares).toString()
    );

    const marginfiAccount1After =
      await bankrunProgram.account.marginfiAccount.fetch(user1Account);
    const balance1After = marginfiAccount1After.lendingAccount.balances.find(
      (b) => b.active === 1 && b.bankPk.equals(driftTokenABank)
    );
    const assetShares1After = new BN(
      wrappedI80F48toBigNumber(balance1After.assetShares).toString()
    );

    assert.ok(assetShares0After.eq(initialAssetShares0));
    assert.ok(assetShares1After.eq(initialAssetShares1));
  });

  it("Validates USDC oracle price conversion matches health check valuation", async () => {
    const bank = await bankrunProgram.account.bank.fetch(driftUsdcBank);
    const assetWeightMaint = wrappedI80F48toBigNumber(
      bank.config.assetWeightMaint
    );

    const usdcOraclePrice = new BN(
      oracles.usdcPrice * 10 ** ecosystem.usdcDecimals
    );
    const { expectedValue: expectedUsdcValue } = calculateExpectedValue(
      initialAssetShares0,
      usdcOraclePrice,
      finalUsdcCumulativeInterest,
      new BN(assetWeightMaint.toString())
    );

    const pulseHealthIx = await makePulseHealthIx(
      users[0].mrgnBankrunProgram,
      user0Account,
      [
        { pubkey: driftUsdcBank, isSigner: false, isWritable: false },
        {
          pubkey: oracles.usdcOracle.publicKey,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: driftAccounts.get(DRIFT_USDC_SPOT_MARKET),
          isSigner: false,
          isWritable: false,
        },
      ]
    );

    await processBankrunTransaction(
      bankrunContext,
      new Transaction().add(pulseHealthIx),
      [users[0].wallet]
    );

    const accAfter = await bankrunProgram.account.marginfiAccount.fetch(
      user0Account
    );
    const healthCache = accAfter.healthCache;

    const assetValue = wrappedI80F48toBigNumber(healthCache.assetValue);
    const actualUsdcValue = new BN(
      Math.floor(assetValue.toNumber() * 10 ** ecosystem.usdcDecimals)
    );
    assertBNApproximately(actualUsdcValue, expectedUsdcValue, new BN(3));
  });

  it("Validates Token A oracle price conversion matches health check valuation", async () => {
    const bank = await bankrunProgram.account.bank.fetch(driftTokenABank);
    const assetWeightMaint = wrappedI80F48toBigNumber(
      bank.config.assetWeightMaint
    );

    const tokenAOraclePrice = new BN(
      oracles.tokenAPrice * 10 ** ecosystem.usdcDecimals
    );
    const { expectedValue: expectedTokenAValue } = calculateExpectedValue(
      initialAssetShares1,
      tokenAOraclePrice,
      finalTokenACumulativeInterest,
      new BN(assetWeightMaint.toString())
    );

    const pulseHealthIx = await makePulseHealthIx(
      users[1].mrgnBankrunProgram,
      user1Account,
      [
        { pubkey: driftTokenABank, isSigner: false, isWritable: false },
        {
          pubkey: oracles.tokenAOracle.publicKey,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET),
          isSigner: false,
          isWritable: false,
        },
      ]
    );

    await processBankrunTransaction(
      bankrunContext,
      new Transaction().add(pulseHealthIx),
      [users[1].wallet]
    );

    const accAfter = await bankrunProgram.account.marginfiAccount.fetch(
      user1Account
    );
    const healthCache = accAfter.healthCache;

    const assetValue = wrappedI80F48toBigNumber(healthCache.assetValue);
    const actualTokenAValue = new BN(
      Math.floor(assetValue.toNumber() * 10 ** ecosystem.usdcDecimals)
    );
    assertBNApproximately(actualTokenAValue, expectedTokenAValue, new BN(1));
  });
});

function calculateExpectedValue(
  assetShares: BN,
  oraclePrice: BN,
  cumulativeInterest: BN,
  assetWeightMaint: BN
): { expectedValue: BN; weightedValue: BN } {
  // Apply Drift's cumulative interest adjustment
  const adjustedOraclePrice = oraclePrice
    .mul(cumulativeInterest)
    .div(DRIFT_SPOT_CUMULATIVE_INTEREST_PRECISION);

  // Apply confidence interval for lower price (PriceBias::Low) to adjusted price
  const confidenceInterval = adjustedOraclePrice
    .mul(new BN(212))
    .div(new BN(10000)); // 0.01 * 2.12 = 0.0212
  const lowerPrice = adjustedOraclePrice.sub(confidenceInterval);

  // Calculate expected value: scaled_balance * lower_price / 10^decimals
  const expectedValue = assetShares
    .mul(lowerPrice)
    .div(new BN(10).pow(new BN(DRIFT_SCALED_BALANCE_DECIMALS)));

  // Apply asset weight for maintenance requirement
  const weightedValue = expectedValue.mul(assetWeightMaint).div(new BN(1));

  return { expectedValue, weightedValue };
}
