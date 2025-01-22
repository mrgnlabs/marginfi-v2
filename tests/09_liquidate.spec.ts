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
  assertI80F48Approx,
  assertI80F48Equal,
  assertKeysEqual,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import { liquidateIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { updatePriceAccount } from "./utils/pyth_mocks";
import { bigNumberToWrappedI80F48, wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { configureBank } from "./utils/instructions";
import { defaultBankConfigOptRaw } from "./utils/types";

describe("Liquidate user", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  const confidenceInterval = 0.0212; // see CONF_INTERVAL_MULTIPLE
  const liquidateAmountA = .2;
  const liquidateAmountA_native = new BN(
    liquidateAmountA * 10 ** ecosystem.tokenADecimals
  );

  it("Oracle data refreshes", async () => {
    const usdcPrice = BigInt(oracles.usdcPrice * 10 ** oracles.usdcDecimals);
    await updatePriceAccount(
      oracles.usdcOracle,
      {
        exponent: -oracles.usdcDecimals,
        aggregatePriceInfo: {
          price: usdcPrice,
          conf: usdcPrice / BigInt(100), // 1% of the price
        },
        twap: {
          // aka ema
          valueComponent: usdcPrice,
        },
      },
      wallet
    );

    const tokenAPrice = BigInt(
      oracles.tokenAPrice * 10 ** oracles.tokenADecimals
    );
    await updatePriceAccount(
      oracles.tokenAOracle,
      {
        exponent: -oracles.tokenADecimals,
        aggregatePriceInfo: {
          price: tokenAPrice,
          conf: tokenAPrice / BigInt(100), // 1% of the price
        },
        twap: {
          // aka ema
          valueComponent: tokenAPrice,
        },
      },
      wallet
    );
  });

  /**
   * Maintenance ratio allowed = 10%
   * Liquidator fee = 2.5%
   * Insurance fee = 2.5%
   * Confidence interval = 2.12%
   * 
   * Token A is worth $10 with conf $0.1 (worth $9.788 low, $10.212 high)
   * USDC is worth $1 with conf $0.01 (worth $0.9788 low, $1.0212 high)
   * 
   * User has:
   * ASSETS
   *    [index 0] 200,000,000 (2) Token A (worth $20)
   * DEBTS
   *    [index 1] 5,050,000 (5.05) USDC (worth $5.05)
   * Note: $5.05 is 25.25% of $20, which is more than 10%, so liquidation is allowed
   *
   * Liquidator tries to repay .2 token A (worth $2) of liquidatee's debt, so liquidator's assets
   * increase by this value, while liquidatee's assets decrease by this value. Which also means that:
   * 
   * Liquidator must pay
   *  value of A minus liquidator fee (low bias within the confidence interval): .2 * (1 - 0.025) * 9.788 = $1.90866
   *  USDC equivalent (high bias): 1.90866 / 1.0212 = $1.869036 (1,869,036 native)
   *
   * Liquidatee receives
   *  value of A minus (liquidator fee + insurance) (low bias): .2 * (1 - 0.025 - 0.025) * 9.788 = $1.8608
   *  USDC equivalent (high bias): 1.8608 / 1.0212 = $1.822457 (1,822,457 native)
   * 
   * Insurance fund collects the difference
   *  USDC diff 1,869,036  - 1,822,457 = 46,579
   */

  it("(user 1) Liquidate user 0 who borrowed USDC against their token A position - happy path", async () => {
    const liquidatee = users[0];
    const liquidator = users[1];

    const assetBankKey = bankKeypairA.publicKey;
    const assetBankBefore = await program.account.bank.fetch(assetBankKey);
    const liabilityBankKey = bankKeypairUsdc.publicKey;
    const liabilityBankBefore = await program.account.bank.fetch(liabilityBankKey);

    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT);
    const liquidateeMarginfiAccount = await program.account.marginfiAccount.fetch(liquidateeAccount);

    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT);
    const liquidatorMarginfiAccount = await program.account.marginfiAccount.fetch(liquidatorAccount);

    const liquidateeBalances = liquidateeMarginfiAccount.lendingAccount.balances;
    const liquidatorBalances = liquidatorMarginfiAccount.lendingAccount.balances;
    const liabilitySharesBefore = liquidateeBalances[1].liabilityShares;
    assertI80F48Equal(liquidatorBalances[1].assetShares, 0);
    
    const insuranceVaultBalance = await getTokenBalance(provider, liabilityBankBefore.insuranceVault);
    assert.equal(insuranceVaultBalance, 0);

    const sharesA = wrappedI80F48toBigNumber(liquidateeBalances[0].assetShares).toNumber();
    const shareValueA = wrappedI80F48toBigNumber(assetBankBefore.assetShareValue).toNumber();
    const sharesUsdc = wrappedI80F48toBigNumber(liabilitySharesBefore).toNumber();
    const shareValueUsdc = wrappedI80F48toBigNumber(liabilityBankBefore.liabilityShareValue).toNumber();
    if (verbose) {
      console.log("BEFORE");
      console.log("liability bank insurance vault before: " + insuranceVaultBalance.toLocaleString());
      console.log("user 0 (liquidatee) Token A asset shares: " + sharesA.toString());
      console.log("  value (in Token A native): " + (sharesA * shareValueA).toLocaleString());
      console.log("  value (in dollars): $" + (sharesA * shareValueA * oracles.tokenAPrice / 10 ** (oracles.tokenADecimals)).toLocaleString());
      console.log("user 0 (liquidatee) USDC liability shares: " + sharesUsdc.toString());
      console.log("  debt (in USDC native): " + (sharesUsdc * shareValueUsdc).toLocaleString());
      console.log("  debt (in dollars): $" + (sharesUsdc * shareValueUsdc * oracles.usdcPrice / 10 ** (oracles.usdcDecimals)).toLocaleString());
      console.log("user 1 (liquidator) USDC asset shares: " + wrappedI80F48toBigNumber(liquidatorBalances[0].assetShares).toString());
      console.log("user 1 (liquidator) USDC liability shares: " + wrappedI80F48toBigNumber(liquidatorBalances[0].liabilityShares).toString());
    }

    let config = defaultBankConfigOptRaw();
    config.assetWeightInit = bigNumberToWrappedI80F48(0.05);
    config.assetWeightMaint = bigNumberToWrappedI80F48(0.1);
    await groupAdmin.mrgnProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await configureBank(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
          bank: assetBankKey,
          bankConfigOpt: config,
        })
      )
    );

    const tokenALowPrice = oracles.tokenAPrice * (1 - confidenceInterval); // see top of test
    const usdcHighPrice = oracles.usdcPrice * (1 + confidenceInterval); // see top of test
    const insuranceToBeCollected = (liquidateAmountA * 0.025 * shareValueA * tokenALowPrice / (shareValueUsdc * usdcHighPrice)) * 10 ** (oracles.usdcDecimals);

    await liquidator.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await liquidateIx(program, {
          assetBankKey,
          liabilityBankKey,
          liquidatorMarginfiAccount: liquidatorAccount,
          liquidatorMarginfiAccountAuthority: liquidatorMarginfiAccount.authority,
          liquidateeMarginfiAccount: liquidateeAccount,
          remaining: [
            oracles.tokenAOracle.publicKey,
            oracles.usdcOracle.publicKey,
            liabilityBankKey,
            oracles.usdcOracle.publicKey,
            assetBankKey,
            oracles.tokenAOracle.publicKey,
            assetBankKey,
            oracles.tokenAOracle.publicKey,
            liabilityBankKey,
            oracles.usdcOracle.publicKey,
          ],
          amount: liquidateAmountA_native,
        })
      )
    );

    const liquidateeMarginfiAccountAfter = await program.account.marginfiAccount.fetch(liquidateeAccount);
    const liquidatorMarginfiAccountAfter = await program.account.marginfiAccount.fetch(liquidatorAccount);

    const liquidateeBalancesAfter = liquidateeMarginfiAccountAfter.lendingAccount.balances;
    const liquidatorBalancesAfter = liquidatorMarginfiAccountAfter.lendingAccount.balances;

    const sharesAAfter = wrappedI80F48toBigNumber(liquidateeBalancesAfter[0].assetShares).toNumber();
    const sharesUsdcAfter = wrappedI80F48toBigNumber(liquidateeBalancesAfter[1].liabilityShares).toNumber();

    assertI80F48Equal(liquidateeBalancesAfter[0].assetShares, wrappedI80F48toBigNumber(liquidateeBalances[0].assetShares).toNumber() - liquidateAmountA_native.toNumber());
    assertI80F48Equal(liquidateeBalancesAfter[0].liabilityShares, 0);
    assertI80F48Equal(liquidateeBalancesAfter[1].assetShares, 0);

    assertI80F48Equal(liquidatorBalancesAfter[0].liabilityShares, 0);
    assertI80F48Equal(liquidatorBalancesAfter[1].assetShares, liquidateAmountA_native);
    assertI80F48Equal(liquidatorBalancesAfter[1].liabilityShares, 0);

    const insuranceVaultBalanceAfter = await getTokenBalance(provider, liabilityBankBefore.insuranceVault);

    assert.approximately(insuranceVaultBalanceAfter, insuranceToBeCollected, (insuranceToBeCollected * .1)); // see top of test

    if (verbose) {
      console.log("AFTER");
      console.log("liability bank insurance vault after (usdc): " + insuranceVaultBalanceAfter.toLocaleString());
      console.log("user 0 (liquidatee) Token A asset shares after: " + sharesAAfter.toString());
      console.log("  value (in Token A native): " + (sharesAAfter * shareValueA).toLocaleString());
      console.log("  value (in dollars): $" + (sharesAAfter * shareValueA * oracles.tokenAPrice / 10 ** (oracles.tokenADecimals)).toLocaleString());
      console.log("user 0 (liquidatee) USDC liability shares after: " + sharesUsdcAfter.toString());
      console.log("  debt (in USDC native): " + (sharesUsdcAfter * shareValueUsdc).toLocaleString());
      console.log("  debt (in dollars): $" + (sharesUsdcAfter * shareValueUsdc * oracles.usdcPrice / 10 ** (oracles.usdcDecimals)).toLocaleString());
      console.log("user 1 (liquidator) USDC asset shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[0].assetShares).toString());
      console.log("user 1 (liquidator) USDC liability shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[0].liabilityShares).toString());
      console.log("user 1 (liquidator) Token A asset shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[1].assetShares).toString());
      console.log("user 1 (liquidator) Token A liability shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[1].liabilityShares).toString());
    }

    assert.equal(liquidatorBalancesAfter[1].active, true);
    assertKeysEqual(liquidatorBalancesAfter[1].bankPk, assetBankKey);

    let now = Math.floor(Date.now() / 1000);
    assertBNApproximately(liquidatorBalancesAfter[0].lastUpdate, now, 2);
    assertBNApproximately(liquidatorBalancesAfter[1].lastUpdate, now, 2);
    assertBNApproximately(liquidateeBalancesAfter[0].lastUpdate, now, 2);
    assertBNApproximately(liquidateeBalancesAfter[1].lastUpdate, now, 2);
  });
});
