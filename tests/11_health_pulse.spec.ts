import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairSol,
  bankKeypairUsdc,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
  users,
  verbose,
} from "./rootHooks";
import {
  assertBNApproximately,
  assertBNEqual,
  assertI80F48Approx,
  assertI80F48Equal,
  assertKeysEqual,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import { healthPulse, liquidateIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { updatePriceAccount } from "./utils/pyth_mocks";
import {
  bigNumberToWrappedI80F48,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import {
  defaultBankConfigOptRaw,
  HEALTH_CACHE_ENGINE_OK,
  HEALTH_CACHE_HEALTHY,
  HEALTH_CACHE_NONE,
  HEALTH_CACHE_ORACLE_OK,
} from "./utils/types";
import { configureBank } from "./utils/group-instructions";

describe("Health pulse", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  it("(user 1) health pulse with bad oracle - cache notes the missing price", async () => {
    const user = users[1];
    const acc = user.accounts.get(USER_ACCOUNT);
    await user.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await healthPulse(user.mrgnProgram, {
          marginfiAccount: acc,
          remaining: [
            bankKeypairUsdc.publicKey,
            oracles.fakeUsdc, // sneaky sneaky
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
          ],
        })
      )
    );

    const accAfter = await program.account.marginfiAccount.fetch(acc);
    const cacheAfter = accAfter.healthCache;
    const now = Date.now() / 1000;
    const assetValue = wrappedI80F48toBigNumber(cacheAfter.assetValue);
    const liabValue = wrappedI80F48toBigNumber(cacheAfter.liabilityValue);

    const flags = cacheAfter.flags.toNumber();
    const oracleOk = (flags & HEALTH_CACHE_ORACLE_OK) !== 0;
    if (verbose) {
      console.log("---user health state---");
      console.log("asset value: " + assetValue.toString());
      console.log("liab value: " + liabValue.toString());
      console.log("oracle ok: " + oracleOk);
      console.log("prices: ");
      for (let i = 0; i < cacheAfter.prices.length; i++) {
        const price = wrappedI80F48toBigNumber(cacheAfter.prices[i]).toNumber();
        if (price != 0) {
          console.log(" [" + i + "] " + price);
        }
      }
    }

    assert.approximately(cacheAfter.timestamp.toNumber(), now, 3);
    // Note: still healthy, and the engine has technically resolved, but the oracle flag is not set!
    // This is not a valid entry for risk purposes but you might use this if you are trying to
    // determine what the price would be if the oracle was in a particular state.
    assertBNEqual(
      cacheAfter.flags,
      HEALTH_CACHE_HEALTHY + HEALTH_CACHE_ENGINE_OK
    );
    // The fake usdc price is set to zero due to the bad oracle
    assertI80F48Equal(cacheAfter.prices[0], 0);
    // User 1 has a trivial amount of token A as well, but we note here it is almost worth zero.
    assert.isAtMost(assetValue.toNumber(), 1);
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
    const cacheAfter = accAfter.healthCache;
    const now = Date.now() / 1000;

    const assetValue = wrappedI80F48toBigNumber(cacheAfter.assetValue);
    const liabValue = wrappedI80F48toBigNumber(cacheAfter.liabilityValue);
    const flags = cacheAfter.flags.toNumber();
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
      console.log("prices: ");
      for (let i = 0; i < cacheAfter.prices.length; i++) {
        const price = wrappedI80F48toBigNumber(cacheAfter.prices[i]).toNumber();
        if (price != 0) {
          console.log(" [" + i + "] " + price);
        }
      }
    }

    assert.approximately(cacheAfter.timestamp.toNumber(), now, 3);
    assertBNEqual(
      cacheAfter.flags,
      HEALTH_CACHE_HEALTHY + HEALTH_CACHE_ENGINE_OK + HEALTH_CACHE_ORACLE_OK
    );
    assertI80F48Approx(cacheAfter.prices[0], oracles.usdcPrice);
    assertI80F48Approx(cacheAfter.prices[1], oracles.tokenAPrice);
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
      const flags = cacheAfter.flags.toNumber();
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
        const price = wrappedI80F48toBigNumber(cacheAfter.prices[i]).toNumber();
        if (price != 0) {
          console.log(" [" + i + "] " + price);
        }
      }
    }

    assert.approximately(cacheAfter.timestamp.toNumber(), now, 3);
    // Note: cache is unhealthy (no HEALTH_CACHE_HEALTHY flag set) but price info is still
    // populated, and the risk engine and oracle report no failures.
    assertBNEqual(
      cacheAfter.flags,
      HEALTH_CACHE_ENGINE_OK + HEALTH_CACHE_ORACLE_OK
    );
    assertI80F48Approx(cacheAfter.prices[0], oracles.tokenAPrice);
    assertI80F48Approx(cacheAfter.prices[1], oracles.usdcPrice);
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
      const flags = cacheAfter.flags.toNumber();
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
        const price = wrappedI80F48toBigNumber(cacheAfter.prices[i]).toNumber();
        if (price != 0) {
          console.log(" [" + i + "] " + price);
        }
      }
    }

    assert.approximately(cacheAfter.timestamp.toNumber(), now, 3);
    assertBNEqual(
      cacheAfter.flags,
      HEALTH_CACHE_HEALTHY + HEALTH_CACHE_ENGINE_OK + HEALTH_CACHE_ORACLE_OK
    );
    assertI80F48Approx(cacheAfter.prices[0], oracles.tokenAPrice);
    assertI80F48Approx(cacheAfter.prices[1], oracles.usdcPrice);
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
