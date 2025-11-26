import { BN } from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import {
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  groupAdmin,
  oracles,
  users,
} from "./rootHooks";
import { genericMultiBankTestSetup } from "./genericSetups";
import {
  borrowIx,
  composeRemainingAccounts,
  depositIx,
} from "./utils/user-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { assert } from "chai";
import { CONF_INTERVAL_MULTIPLE, ORACLE_CONF_INTERVAL } from "./utils/types";
import { refreshPullOraclesBankrun } from "./utils/bankrun-oracles";
import { assertI80F48Approx, assertI80F48Equal } from "./utils/genericTests";

const readCacheFields = (cache: any) => {
  // Support both new and legacy cache field names to avoid IDL drift during the refactor
  const price =
    cache?.lastOraclePrice ??
    cache?.oraclePriceUsed ??
    cache?.oraclePrice; // legacy fallback
  const ts =
    cache?.lastOraclePriceTimestamp ??
    cache?.oraclePriceTimestamp ??
    cache?.oraclePriceTime ??
    0;
  const conf =
    cache?.lastOraclePriceConfidence ??
    cache?.oraclePriceConfidence ??
    cache?.oraclePriceConf ??
    0;

  return { price, ts, conf };
};

/**
 * Quick bankrun test to ensure the bank cache records the last oracle price (and confidence)
 * whenever a price-based instruction updates shares. Uses genericMultiBankTestSetup so the
 * environment can be thrown away.
 */
describe("Bank cache last oracle price", () => {
  const startingSeed = 533;
  // Keypair::fromSeed requires exactly 32 bytes
  const groupBuff = Buffer.from("MARGINFI_GROUP_SEED_LAST_ORACLE0");
  const USER_ACCOUNT_THROWAWAY = "throwaway_last_oracle";

  let banks: PublicKey[] = [];

  const seedAmount = new BN(1_000 * 10 ** ecosystem.lstAlphaDecimals);
  const collateralAmount = new BN(200 * 10 ** ecosystem.lstAlphaDecimals);
  const borrowAmount = new BN(50 * 10 ** ecosystem.lstAlphaDecimals);

  it("init group, banks, seed liquidity, and verify cache starts empty", async () => {
    const result = await genericMultiBankTestSetup(
      2,
      USER_ACCOUNT_THROWAWAY,
      groupBuff,
      startingSeed
    );
    banks = result.banks;

    // Admin seeds both banks with liquidity
    const tx = new Transaction();
    for (const bank of banks) {
      tx.add(
        await depositIx(groupAdmin.mrgnBankrunProgram, {
          marginfiAccount: groupAdmin.accounts.get(USER_ACCOUNT_THROWAWAY),
          bank,
          tokenAccount: groupAdmin.lstAlphaAccount,
          amount: seedAmount,
          depositUpToLimit: false,
        })
      );
    }
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    const bank = await bankrunProgram.account.bank.fetch(banks[1]);
    const cache = bank.cache;
    const { price, ts, conf } = readCacheFields(cache);

    assertI80F48Equal(price, 0);
    assert.equal(Number(ts), 0);
    assertI80F48Equal(conf, 0);
  });

  it("(user 0) borrow stamps last oracle price/confidence", async () => {
    await refreshPullOraclesBankrun(oracles, bankrunContext, banksClient);

    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);

    // Deposit collateral into bank 0
    {
      const tx = new Transaction().add(
        await depositIx(user.mrgnBankrunProgram, {
          marginfiAccount: userAccount,
          bank: banks[0],
          tokenAccount: user.lstAlphaAccount,
          amount: collateralAmount,
          depositUpToLimit: false,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(user.wallet);
      await banksClient.processTransaction(tx);
    }

    // Borrow from bank 1 to trigger a price-based share update
    {
      const remaining = composeRemainingAccounts([
        [banks[0], oracles.pythPullLst.publicKey],
        [banks[1], oracles.pythPullLst.publicKey],
      ]);
      const tx = new Transaction().add(
        ComputeBudgetProgram.setComputeUnitLimit({ units: 300_000 }),
        await borrowIx(user.mrgnBankrunProgram, {
          marginfiAccount: userAccount,
          bank: banks[1],
          tokenAccount: user.lstAlphaAccount,
          remaining,
          amount: borrowAmount,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(user.wallet);
      await banksClient.processTransaction(tx);
    }

    const bankAfter = await bankrunProgram.account.bank.fetch(banks[1]);
    const cache = bankAfter.cache;
    const { price, ts, conf } = readCacheFields(cache);
    const expectedPrice = oracles.lstAlphaPrice;
    const expectedConf =
      expectedPrice * ORACLE_CONF_INTERVAL * CONF_INTERVAL_MULTIPLE;

    assert.isAbove(Number(ts), 0);
    assertI80F48Approx(price, expectedPrice, 0.0001);
    assertI80F48Approx(conf, expectedConf, 0.01);
  });
});
