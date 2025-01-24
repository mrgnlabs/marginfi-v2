import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  workspace,
} from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairSol,
  bankKeypairUsdc,
  bankrunContext,
  bankrunProgram,
  bankRunProvider,
  banksClient,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
  users,
  validators,
  verbose,
} from "./rootHooks";
import {
  assertBNApproximately,
  assertI80F48Equal,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import { liquidateIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { bigNumberToWrappedI80F48, getMint, wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { defaultStakedInterestSettings, StakedSettingsEdit } from "./utils/types";
import { editStakedSettings, propagateStakedSettings } from "./utils/group-instructions";
import { deriveStakedSettings } from "./utils/pdas";

describe("Liquidate user (including staked assets)", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  let settingsKey: PublicKey;
  before(async () => {
    [settingsKey] = deriveStakedSettings(
      program.programId,
      marginfiGroup.publicKey
    );
  });

  const confidenceInterval = 0.0212; // 1% confidence * CONF_INTERVAL_MULTIPLE
  const liquidateAmountSol = 0.1;
  const liquidateAmountSol_native = new BN(
    liquidateAmountSol * 10 ** ecosystem.wsolDecimals
  );

  /**
   * Maintenance ratio allowed = 10%
   * Liquidator fee = 2.5%
   * Insurance fee = 2.5%
   * Confidence interval = 2.12% (1% confidence * 2.12 = 2.12%)
   * 
   * 
   * Staked SOL (hereinafter Staked) is worth $305.04680972609873 with conf ~$6.46 (worth $298.573 low, $311.506 high)
   * SOL is worth $150 with conf ~$3.18 (worth $146.82 low, $153.18 high)
   * 
   * User 2 has a validator 0 Staked [0] deposit position and a SOL [1] debt position:
   * ASSETS
   *    [index 0] 1,000,000,000 (1) Staked (worth $305.047)
   * DEBTS
   *    [index 1] 1,122,110,000.0000017 (1.12211) SOL (worth $168.317)
   * Note: $168.317 is ~55% of $305.047, which is more than 10%, so liquidation is allowed
   *
   * Liquidator tries to repay 0.1 Staked (worth $30.5047) of liquidatee's debt, so liquidator's assets
   * increase by this value, while liquidatee's assets decrease by this value. Which also means that:
   * 
   * Liquidator must pay
   *  value of Staked minus liquidator fee (low bias within the confidence interval): .1 * (1 - 0.025) * 298.573 = $29.133
   *  SOL equivalent (high bias): 29.133 / 153.18 ~= 0.1902 (190,188,014 native)
   *
   * Liquidatee receives
   *  value of Staked minus (liquidator fee + insurance) (low bias): .1 * (1 - 0.025 - 0.025) * 298.573 = $27.659
   *  SOL equivalent (high bias): 27.659 / 153.18 ~= 0.1806 (180,565,347 native)
   * 
   * Insurance fund collects the difference
   *  SOL diff 190,188,014  - 180,565,347 = 9,622,667 (the actual number in the test can be different, since the Staked price is approximated)
   */

  it("(user 1) liquidates user 2 with staked SOL against their SOL position - succeeds", async () => {
    const liquidatee = users[2];
    const liquidator = users[1];

    const assetBankKey = validators[0].bank;
    const assetBankBefore = await bankrunProgram.account.bank.fetch(assetBankKey);
    const liabilityBankKey = bankKeypairSol.publicKey;
    const liabilityBankBefore = await bankrunProgram.account.bank.fetch(liabilityBankKey);
    
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT);
    const liquidateeMarginfiAccount = await bankrunProgram.account.marginfiAccount.fetch(liquidateeAccount);

    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT);
    const liquidatorMarginfiAccount = await bankrunProgram.account.marginfiAccount.fetch(liquidatorAccount);

    const liquidateeBalances = liquidateeMarginfiAccount.lendingAccount.balances;
    const liquidatorBalances = liquidatorMarginfiAccount.lendingAccount.balances;
  
    const insuranceVaultBalance = await getTokenBalance(bankRunProvider, liabilityBankBefore.insuranceVault);
    assert.equal(insuranceVaultBalance, 0);

    const sharesStaked = wrappedI80F48toBigNumber(liquidateeBalances[0].assetShares).toNumber();
    const shareValueStaked = wrappedI80F48toBigNumber(assetBankBefore.assetShareValue).toNumber();
    const sharesSol = wrappedI80F48toBigNumber(liquidateeBalances[1].liabilityShares).toNumber();
    const shareValueSol = wrappedI80F48toBigNumber(liabilityBankBefore.liabilityShareValue).toNumber();
  
    const solPool = await bankRunProvider.connection.getAccountInfo(
      validators[0].splSolPool
    );
    const solPoolLamports = solPool.lamports;
    const mintData = await getMint(bankRunProvider.connection, validators[0].splMint);
    const stakedPrice = oracles.wsolPrice * (solPoolLamports) / Number(mintData.supply);

    if (verbose) {
      console.log("BEFORE");
      console.log("liability bank insurance vault before: " + insuranceVaultBalance.toLocaleString());
      console.log("user 0 (liquidatee) Staked asset shares: " + sharesStaked.toString());
      console.log("  value (in Staked native): " + (sharesStaked * shareValueStaked).toLocaleString());
      console.log("  value (in dollars): $" + (sharesStaked * shareValueStaked * stakedPrice / 10 ** (oracles.wsolDecimals)).toLocaleString());
      console.log("user 0 (liquidatee) SOL liability shares: " + sharesSol.toString());
      console.log("  debt (in SOL native): " + (sharesSol * shareValueSol).toLocaleString());
      console.log("  debt (in dollars): $" + (sharesSol * shareValueSol * oracles.wsolPrice / 10 ** (oracles.wsolDecimals)).toLocaleString());
      console.log("user 1 (liquidator) staked asset shares: " + wrappedI80F48toBigNumber(liquidatorBalances[0].assetShares).toString());
      console.log("user 1 (liquidator) USDC liability shares: " + wrappedI80F48toBigNumber(liquidatorBalances[0].liabilityShares).toString());
    }

    const defaultSettings = defaultStakedInterestSettings(
      oracles.wsolOracle.publicKey
    );
    const settings: StakedSettingsEdit = {
      oracle: defaultSettings.oracle,
      assetWeightInit: bigNumberToWrappedI80F48(0.05),
      assetWeightMaint: bigNumberToWrappedI80F48(0.1),
      depositLimit: defaultSettings.depositLimit,
      totalAssetValueInitLimit: defaultSettings.totalAssetValueInitLimit,
      oracleMaxAge: defaultSettings.oracleMaxAge,
      riskTier: defaultSettings.riskTier,
    };
    let editTx = new Transaction().add(
      await editStakedSettings(groupAdmin.mrgnProgram, {
        settingsKey: settingsKey,
        settings: settings,
      }),
      await propagateStakedSettings(program, {
        settings: settingsKey,
        bank: assetBankKey,
        oracle: defaultSettings.oracle,
      })
    );
    editTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    editTx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(editTx);
  
    const stakedLowPrice = stakedPrice * (1 - confidenceInterval); // see top of test
    const wsolHighPrice = oracles.wsolPrice * (1 + confidenceInterval); // see top of test
    const insuranceToBeCollected = (liquidateAmountSol * 0.025 * shareValueStaked * stakedLowPrice / (shareValueSol * wsolHighPrice)) * 10 ** (oracles.wsolDecimals);

    let tx = new Transaction().add(
      await liquidateIx(bankrunProgram, {
        assetBankKey,
        liabilityBankKey,
        liquidatorMarginfiAccount: liquidatorAccount,
        liquidatorMarginfiAccountAuthority: liquidatorMarginfiAccount.authority,
        liquidateeMarginfiAccount: liquidateeAccount,
        remaining: [
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          oracles.wsolOracle.publicKey,
          liabilityBankKey,
          oracles.wsolOracle.publicKey,
          assetBankKey,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          assetBankKey,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          liabilityBankKey,
          oracles.wsolOracle.publicKey,
        ],
        amount: liquidateAmountSol_native,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(liquidator.wallet);
    await banksClient.processTransaction(tx);

    const liquidateeMarginfiAccountAfter = await bankrunProgram.account.marginfiAccount.fetch(liquidateeAccount);
    const liquidatorMarginfiAccountAfter = await bankrunProgram.account.marginfiAccount.fetch(liquidatorAccount);

    const liquidateeBalancesAfter = liquidateeMarginfiAccountAfter.lendingAccount.balances;
    const liquidatorBalancesAfter = liquidatorMarginfiAccountAfter.lendingAccount.balances;

    const sharesStakedAfter = wrappedI80F48toBigNumber(liquidateeBalancesAfter[0].assetShares).toNumber();
    const sharesSolAfter = wrappedI80F48toBigNumber(liquidateeBalancesAfter[1].liabilityShares).toNumber();

    assertI80F48Equal(liquidateeBalancesAfter[0].assetShares, wrappedI80F48toBigNumber(liquidateeBalances[0].assetShares).toNumber() - liquidateAmountSol_native.toNumber());
    assertI80F48Equal(liquidateeBalancesAfter[0].liabilityShares, 0);
    assertI80F48Equal(liquidateeBalancesAfter[1].assetShares, 0);

    assertI80F48Equal(liquidatorBalancesAfter[0].liabilityShares, 0);
    assertI80F48Equal(liquidatorBalancesAfter[1].assetShares, wrappedI80F48toBigNumber(liquidatorBalances[1].assetShares).toNumber() + liquidateAmountSol_native.toNumber());
    assertI80F48Equal(liquidatorBalancesAfter[1].liabilityShares, 0);

    const insuranceVaultBalanceAfter = await getTokenBalance(bankRunProvider, liabilityBankBefore.insuranceVault);
    assert.approximately(insuranceVaultBalanceAfter, insuranceToBeCollected, (insuranceToBeCollected * .1)); // see top of test

    if (verbose) {
      console.log("AFTER");
      console.log("liability bank insurance vault after (SOL): " + insuranceVaultBalanceAfter.toLocaleString());
      console.log("user 0 (liquidatee) Staked asset shares after: " + sharesStakedAfter.toString());
      console.log("  value (in Staked native): " + (sharesStakedAfter * shareValueStaked).toLocaleString());
      console.log("  value (in dollars): $" + (sharesStakedAfter * shareValueStaked * stakedPrice / 10 ** (oracles.wsolDecimals)).toLocaleString());
      console.log("user 0 (liquidatee) SOL liability shares after: " + sharesSolAfter.toString());
      console.log("  debt (in SOL native): " + (sharesSolAfter * shareValueSol).toLocaleString());
      console.log("  debt (in dollars): $" + (sharesSolAfter * shareValueSol * oracles.wsolPrice / 10 ** (oracles.wsolDecimals)).toLocaleString());
      console.log("user 1 (liquidator) SOL asset shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[0].assetShares).toString());
      console.log("user 1 (liquidator) SOL liability shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[0].liabilityShares).toString());
      console.log("user 1 (liquidator) Staked asset shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[1].assetShares).toString());
      console.log("user 1 (liquidator) Staked liability shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[1].liabilityShares).toString());
    }

    let now = Math.floor(Date.now() / 1000);
    assertBNApproximately(liquidatorBalancesAfter[0].lastUpdate, now, 20);
    assertBNApproximately(liquidatorBalancesAfter[1].lastUpdate, now, 20);
    assertBNApproximately(liquidateeBalancesAfter[0].lastUpdate, now, 20);
    assertBNApproximately(liquidateeBalancesAfter[1].lastUpdate, now, 20);
  });
});

// TODO: 0,1 - should fail