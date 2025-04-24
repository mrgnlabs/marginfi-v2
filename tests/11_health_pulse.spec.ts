import {
  AnchorProvider,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairUsdc,
  groupAdmin,
  oracles,
  users,
  verbose,
} from "./rootHooks";
import { assert } from "chai";
import { healthPulse } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import {
  defaultBankConfigOptRaw,
  HEALTH_CACHE_ENGINE_OK,
  HEALTH_CACHE_HEALTHY,
  HEALTH_CACHE_ORACLE_OK,
  HEALTH_CACHE_PROGRAM_VERSION_0_1_3,
} from "./utils/types";
import { configureBank } from "./utils/group-instructions";
import { bytesToF64 } from "./utils/tools";

describe("Health pulse", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;
  /** Tolerance for float inaccuracy */
  const t = 0.00001;

  it("(user 1) health pulse with bad oracle - cache notes the missing price", async () => {
    const user = users[1];
    const acc = user.accounts.get(USER_ACCOUNT);
    await user.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await healthPulse(user.mrgnProgram, {
          marginfiAccount: acc,
          remaining: [
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
            bankKeypairA.publicKey,
            oracles.fakeUsdc, // sneaky sneaky
          ],
        })
      )
    );

    const accAfter = await program.account.marginfiAccount.fetch(acc);
    const cacheAfter = accAfter.healthCache;
    const now = Date.now() / 1000;
    const assetValue = wrappedI80F48toBigNumber(cacheAfter.assetValue);
    const liabValue = wrappedI80F48toBigNumber(cacheAfter.liabilityValue);

    const flags = cacheAfter.flags;
    const oracleOk = (flags & HEALTH_CACHE_ORACLE_OK) !== 0;
    if (verbose) {
      console.log("---user health state---");
      console.log("asset value: " + assetValue.toString());
      console.log("liab value: " + liabValue.toString());
      console.log("oracle ok: " + oracleOk);
      console.log("internal error: " + cacheAfter.internalErr);
      console.log("index of err:   " + cacheAfter.errIndex);
      console.log("prices: ");
      for (let i = 0; i < cacheAfter.prices.length; i++) {
        const price = bytesToF64(cacheAfter.prices[i]);
        if (price != 0) {
          console.log(" [" + i + "] " + price);
        }
      }
    }

    assert.approximately(cacheAfter.timestamp.toNumber(), now, 3);
    // Note: still healthy, and the engine has technically resolved, but the oracle flag is not set!
    // This is not a valid entry for risk purposes but you might use this if you are trying to
    // determine what the price would be if the oracle was in a particular state.
    assert.equal(
      cacheAfter.flags,
      HEALTH_CACHE_HEALTHY + HEALTH_CACHE_ENGINE_OK
    );
    // no error, the risk engine didn't reject this even with the bad oracle because there are no
    // liabilities, so any asset balance is valid!
    assert.equal(cacheAfter.mrgnErr, 0);
    assert.equal(cacheAfter.internalErr, 6052); // (WrongOracleAccountKeys)
    assert.equal(cacheAfter.errIndex, 1);
    // The fake usdc price is set to zero due to the bad oracle
    assert.approximately(bytesToF64(cacheAfter.prices[1]), 0, t);
  });

  it("(user 1) health pulse - happy path", async () => {
    const user = users[1];
    const acc = user.accounts.get(USER_ACCOUNT);
    await user.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await healthPulse(user.mrgnProgram, {
          marginfiAccount: acc,
          remaining: [
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
          ],
        })
      )
    );

    const accAfter = await program.account.marginfiAccount.fetch(acc);
    const cA = accAfter.healthCache;
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

    assert.approximately(cA.timestamp.toNumber(), now, 3);
    assert.equal(
      cA.flags,
      HEALTH_CACHE_HEALTHY + HEALTH_CACHE_ENGINE_OK + HEALTH_CACHE_ORACLE_OK
    );
    assert.equal(cA.programVersion, HEALTH_CACHE_PROGRAM_VERSION_0_1_3);
    // Note: Technically this is wrong, our test suite doesn't have any ema confidence for legacy
    // pyth oracles so the price uses the actual value. You can note that for an oracle with a valid
    // ema conf like LST the value here will be price - conf.
    assert.approximately(bytesToF64(cA.prices[0]), oracles.usdcPrice, t);
    assert.approximately(bytesToF64(cA.prices[1]), oracles.tokenAPrice, t);
  });

  it("(user 0) health pulse in unhealthy state - happy path", async () => {
    const user = users[0];
    const acc = user.accounts.get(USER_ACCOUNT);
    await user.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await healthPulse(user.mrgnProgram, {
          marginfiAccount: acc,
          remaining: [
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
          ],
        })
      )
    );

    const accAfter = await program.account.marginfiAccount.fetch(acc);
    const cacheAfter = accAfter.healthCache;
    const now = Date.now() / 1000;

    const assetValue = wrappedI80F48toBigNumber(cacheAfter.assetValue);
    const liabValue = wrappedI80F48toBigNumber(cacheAfter.liabilityValue);
    let expectedValue: number;
    let expectedDebt: number;
    if (verbose) {
      console.log("---user balances state---");
      const bals = accAfter.lendingAccount.balances;
      for (let i = 0; i < bals.length; i++) {
        const b = bals[i];
        const bankKey = b.bankPk;
        const shares = wrappedI80F48toBigNumber(b.assetShares).toNumber();
        const debt = wrappedI80F48toBigNumber(b.liabilityShares).toNumber();
        if (shares != 0) {
          const bankAcc = await program.account.bank.fetch(bankKey);
          const config = bankAcc.config;
          expectedValue =
            shares *
            wrappedI80F48toBigNumber(bankAcc.assetShareValue).toNumber() *
            wrappedI80F48toBigNumber(config.assetWeightInit).toNumber();
          console.log(
            " [" + i + "] (asset): " + shares + " of " + bankKey.toString()
          );
          // Note: Multiply this by the asset price, e.g. for Token A this is $10
          console.log("  exp value/price: " + expectedValue);
        }
        if (debt != 0) {
          const bankAcc = await program.account.bank.fetch(bankKey);
          const config = bankAcc.config;
          expectedDebt =
            debt *
            wrappedI80F48toBigNumber(bankAcc.liabilityShareValue).toNumber() *
            wrappedI80F48toBigNumber(config.liabilityWeightInit).toNumber();
          console.log(
            " [" + i + "] (debt):  " + debt + " in " + bankKey.toString()
          );
          // Note: Multiply this by the asset price, e.g. for Token A this is $1
          console.log("  exp value/price: " + expectedDebt);
        }
      }

      console.log("---user health state---");
      const flags = cacheAfter.flags;
      const isHealthy = (flags & HEALTH_CACHE_HEALTHY) !== 0;
      const engineOk = (flags & HEALTH_CACHE_ENGINE_OK) !== 0;
      const oracleOk = (flags & HEALTH_CACHE_ORACLE_OK) !== 0;
      console.log("healthy: " + isHealthy);
      console.log("engine ok: " + engineOk);
      console.log("oracle ok: " + oracleOk);
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

    assert.approximately(cacheAfter.timestamp.toNumber(), now, 3);
    // Note: cache is unhealthy (no HEALTH_CACHE_HEALTHY flag set) but price info is still
    // populated, and the risk engine and oracle report no failures.
    assert.equal(
      cacheAfter.flags,
      HEALTH_CACHE_ENGINE_OK + HEALTH_CACHE_ORACLE_OK
    );
    assert.equal(cacheAfter.mrgnErr, 6009); // RiskEngineInitRejected
    assert.approximately(
      bytesToF64(cacheAfter.prices[0]),
      oracles.tokenAPrice,
      t
    );
    assert.approximately(
      bytesToF64(cacheAfter.prices[1]),
      oracles.usdcPrice,
      t
    );
    assert.approximately(
      (expectedValue * oracles.tokenAPrice) / 10 ** oracles.tokenADecimals,
      assetValue.toNumber(),
      0.01
    );
    assert.approximately(
      (expectedDebt * oracles.usdcPrice) / 10 ** oracles.usdcDecimals,
      liabValue.toNumber(),
      0.01
    );
  });

  it("(admin) restore the default config to Token A bank", async () => {
    let config = defaultBankConfigOptRaw();
    await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await configureBank(groupAdmin.mrgnProgram, {
          bank: bankKeypairA.publicKey,
          bankConfigOpt: config,
        })
      )
    );
  });

  it("(user 0) health pulse in now-healthy state - happy path", async () => {
    const user = users[0];
    const acc = user.accounts.get(USER_ACCOUNT);
    await user.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await healthPulse(user.mrgnProgram, {
          marginfiAccount: acc,
          remaining: [
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
          ],
        })
      )
    );

    const accAfter = await program.account.marginfiAccount.fetch(acc);
    const cacheAfter = accAfter.healthCache;
    const now = Date.now() / 1000;

    const assetValue = wrappedI80F48toBigNumber(cacheAfter.assetValue);
    const liabValue = wrappedI80F48toBigNumber(cacheAfter.liabilityValue);
    let expectedValue: number;
    let expectedDebt: number;
    if (verbose) {
      console.log("---user balances state---");
      const bals = accAfter.lendingAccount.balances;
      for (let i = 0; i < bals.length; i++) {
        const b = bals[i];
        const bankKey = b.bankPk;
        const shares = wrappedI80F48toBigNumber(b.assetShares).toNumber();
        const debt = wrappedI80F48toBigNumber(b.liabilityShares).toNumber();
        if (shares != 0) {
          const bankAcc = await program.account.bank.fetch(bankKey);
          const config = bankAcc.config;
          expectedValue =
            shares *
            wrappedI80F48toBigNumber(bankAcc.assetShareValue).toNumber() *
            wrappedI80F48toBigNumber(config.assetWeightInit).toNumber();
          console.log(
            " [" + i + "] (asset): " + shares + " of " + bankKey.toString()
          );
          // Note: Multiply this by the asset price, e.g. for Token A this is $10
          console.log("  exp value/price: " + expectedValue);
        }
        if (debt != 0) {
          const bankAcc = await program.account.bank.fetch(bankKey);
          const config = bankAcc.config;
          expectedDebt =
            debt *
            wrappedI80F48toBigNumber(bankAcc.liabilityShareValue).toNumber() *
            wrappedI80F48toBigNumber(config.liabilityWeightInit).toNumber();
          console.log(
            " [" + i + "] (debt):  " + debt + " in " + bankKey.toString()
          );
          // Note: Multiply this by the asset price, e.g. for Token A this is $1
          console.log("  exp value/price: " + expectedDebt);
        }
      }

      console.log("---user health state---");
      const flags = cacheAfter.flags;
      const isHealthy = (flags & HEALTH_CACHE_HEALTHY) !== 0;
      const engineOk = (flags & HEALTH_CACHE_ENGINE_OK) !== 0;
      const oracleOk = (flags & HEALTH_CACHE_ORACLE_OK) !== 0;
      console.log("healthy: " + isHealthy);
      console.log("engine ok: " + engineOk);
      console.log("oracle ok: " + oracleOk);
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

    assert.approximately(cacheAfter.timestamp.toNumber(), now, 3);
    assert.equal(
      cacheAfter.flags,
      HEALTH_CACHE_HEALTHY + HEALTH_CACHE_ENGINE_OK + HEALTH_CACHE_ORACLE_OK
    );
    assert.approximately(
      bytesToF64(cacheAfter.prices[0]),
      oracles.tokenAPrice,
      t
    );
    assert.approximately(
      bytesToF64(cacheAfter.prices[1]),
      oracles.usdcPrice,
      t
    );
    assert.approximately(
      (expectedValue * oracles.tokenAPrice) / 10 ** oracles.tokenADecimals,
      assetValue.toNumber(),
      0.01
    );
    assert.approximately(
      (expectedDebt * oracles.usdcPrice) / 10 ** oracles.usdcDecimals,
      liabValue.toNumber(),
      0.01
    );
  });
});
