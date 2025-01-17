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
  assertBNEqual,
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
import { deriveLiquidityVaultAuthority } from "./utils/pdas";
import { configureBank } from "./utils/instructions";
import { defaultBankConfigOptRaw } from "./utils/types";

describe("Liquidate user", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  const liquidateAmountUsdc = 20;
  const liquidateAmountUsdc_native = new BN(
    liquidateAmountUsdc * 10 ** ecosystem.usdcDecimals
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

  it("(user 1) Liquidate user 0 who borrowed USDC against their token A position - happy path", async () => {
    const liquidatee = users[0];
    const liquidator = users[1];

    const assetBankKey = bankKeypairA.publicKey;
    const assetBank = await program.account.bank.fetch(assetBankKey);
    const liabilityBankKey = bankKeypairUsdc.publicKey;
    const liabilityBank = await program.account.bank.fetch(liabilityBankKey);

    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT);
    const liquidateeMarginfiAccount = await program.account.marginfiAccount.fetch(liquidateeAccount);

    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT);
    const liquidatorMarginfiAccount = await program.account.marginfiAccount.fetch(liquidatorAccount);

    const liquidateeBalances = liquidateeMarginfiAccount.lendingAccount.balances;
    const liquidatorBalances = liquidatorMarginfiAccount.lendingAccount.balances;
    const liabilitySharesBefore = liquidateeBalances[1].liabilityShares;
    assertI80F48Equal(liquidatorBalances[1].assetShares, 0);
    
    const insuranceVaultBalance = await getTokenBalance(provider, liabilityBank.insuranceVault);
    assert.equal(insuranceVaultBalance, 0);

    if (verbose) {
      console.log("BEFORE");
      console.log("liability bank insurance vault before: " + insuranceVaultBalance.toLocaleString());
      console.log("user 0 (liquidatee) Token A asset shares: " + wrappedI80F48toBigNumber(liquidateeBalances[0].assetShares).toString());
      console.log("user 0 (liquidatee) USDC liability shares: " + wrappedI80F48toBigNumber(liabilitySharesBefore).toString());
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

    await liquidator.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await liquidateIx(program, {
          marginfiGroup: marginfiGroup.publicKey,
          assetBankKey,
          liabilityBankKey,
          liquidatorMarginfiAccount: liquidatorAccount,
          liquidatorMarginfiAccountAuthority: liquidatorMarginfiAccount.authority,
          liquidateeMarginfiAccount: liquidateeAccount,
          bankLiquidityVault: liabilityBank.liquidityVault,
          bankLiquidityVaultAuthority: deriveLiquidityVaultAuthority(program.programId, liabilityBankKey)[0],
          bankInsuranceVault: liabilityBank.insuranceVault,
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
          amount: liquidateAmountUsdc_native,
        })
      )
    );

    const liquidateeMarginfiAccountAfter = await program.account.marginfiAccount.fetch(liquidateeAccount);
    const liquidatorMarginfiAccountAfter = await program.account.marginfiAccount.fetch(liquidatorAccount);

    const liquidateeBalancesAfter = liquidateeMarginfiAccountAfter.lendingAccount.balances;
    const liquidatorBalancesAfter = liquidatorMarginfiAccountAfter.lendingAccount.balances;
    assertI80F48Equal(liquidateeBalancesAfter[0].assetShares, wrappedI80F48toBigNumber(liquidateeBalances[0].assetShares).toNumber() - liquidateAmountUsdc_native.toNumber());
    assertI80F48Equal(liquidateeBalancesAfter[0].liabilityShares, 0);
    assertI80F48Equal(liquidatorBalancesAfter[1].assetShares, liquidateAmountUsdc_native);

    const insuranceVaultBalanceAfter = await getTokenBalance(provider, liabilityBank.insuranceVault);
    //assert.equal(insuranceVaultBalanceAfter, 0);

    if (verbose) {
      console.log("AFTER");
      console.log("liability bank insurance vault after: " + insuranceVaultBalanceAfter.toLocaleString());
      console.log("user 0 (liquidatee) Token A asset shares after: " + wrappedI80F48toBigNumber(liquidateeBalancesAfter[0].assetShares).toString());
      console.log("user 0 (liquidatee) USDC liability shares after: " + wrappedI80F48toBigNumber(liquidateeBalancesAfter[1].liabilityShares).toString());
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
