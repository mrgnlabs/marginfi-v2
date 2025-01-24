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
  assertBankrunTxFailed,
  assertBNApproximately,
  assertI80F48Equal,
  assertKeysEqual,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import { borrowIx, liquidateIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { bigNumberToWrappedI80F48, getMint, wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { defaultBankConfigOptRaw, defaultStakedInterestSettings, StakedSettingsEdit } from "./utils/types";
import { configureBank, editStakedSettings, propagateStakedSettings } from "./utils/group-instructions";
import {BankConfigOptWithAssetTag} from "./utils/types";
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


  const provider = getProvider() as AnchorProvider;

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
   * USDC is worth $1 with conf $0.01 (worth $0.9788 low, $1.0212 high)
   * SOL is worth $150 with conf $1.5 (worth $9.788 low, $10.212 high)
   * 
   * User 0 has a USDC [0] deposit position and a validator 0 Staked [1] deposit position:
   * ASSETS
   *    [index 0] 10,000,000 (10) USDC (worth $10)
   * DEBTS
   *    [index 1] 10,100,000.000000015 (0.0101) SOL (worth $1,515)
   * Note: $1,515 is 15.15% of $10, which is more than 10%, so liquidation is allowed
   *
   * Liquidator tries to repay 1.0 USDC (worth $1) of liquidatee's debt, so liquidator's assets
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

  it("(user 1) liquidates user 2 with staked user 0 who borrowed SOL against their USDC position - succeeds", async () => {
    const liquidatee = users[2];
    const liquidator = users[1];
    console.log("usdcBankPk: " + bankKeypairUsdc.publicKey.toString());
    console.log("solBankPk: " + bankKeypairSol.publicKey.toString());
    console.log("aBankPk: " + bankKeypairA.publicKey.toString());
    console.log("stakeBank: " + validators[0].bank.toString());

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
    console.log("Asset Bank asset tag: " + assetBankBefore.config.assetTag.toString());
    console.log("Liab Bank asset tag: " + liabilityBankBefore.config.assetTag.toString());
    console.log("Liquidatee 0 Bank asset tag: " + liquidateeBalances[0].bankAssetTag.toString());
    console.log("Liquidatee 1 Bank asset tag: " + liquidateeBalances[1].bankAssetTag.toString());
    console.log("bank 0: " + liquidateeBalances[0].bankPk.toString());
    console.log("assets 0: " + wrappedI80F48toBigNumber(liquidateeBalances[0].assetShares).toNumber().toString());
    console.log("liabs 0: " + wrappedI80F48toBigNumber(liquidateeBalances[0].liabilityShares).toNumber().toString());
    console.log("bank 1: " + liquidateeBalances[1].bankPk.toString());
    console.log("assets 1: " + wrappedI80F48toBigNumber(liquidateeBalances[1].assetShares).toNumber().toString());
    console.log("liabs 1: " + wrappedI80F48toBigNumber(liquidateeBalances[1].liabilityShares).toNumber().toString());

    for (let i = 0; i < liquidateeBalances.length; i++) {
      if (!liquidateeBalances[i].active)
        continue;
      console.log("Liquidatee " + i + " Bank asset tag: " + liquidateeBalances[i].bankAssetTag.toString());
    }

    // Check this!
    console.log("bank 2 active: " + liquidateeBalances[2].active.toString());
    console.log("bank 2: " + liquidateeBalances[2].bankPk.toString());
    console.log("assets 2: " + wrappedI80F48toBigNumber(liquidateeBalances[2].assetShares).toNumber().toString());
    console.log("liabs 2: " + wrappedI80F48toBigNumber(liquidateeBalances[2].liabilityShares).toNumber().toString());

    console.log("Liquidator 0 Bank asset tag: " + liquidatorBalances[0].bankAssetTag.toString());
    console.log("Liquidator 1 Bank asset tag: " + liquidatorBalances[1].bankAssetTag.toString());
    console.log("bank 0: " + liquidatorBalances[0].bankPk.toString());
    console.log("assets 0: " + wrappedI80F48toBigNumber(liquidatorBalances[0].assetShares).toNumber().toString());
    console.log("liabs 0: " + wrappedI80F48toBigNumber(liquidatorBalances[0].liabilityShares).toNumber().toString());
    console.log("bank 1: " + liquidatorBalances[1].bankPk.toString());
    console.log("assets 1: " + wrappedI80F48toBigNumber(liquidatorBalances[1].assetShares).toNumber().toString());
    console.log("liabs 1: " + wrappedI80F48toBigNumber(liquidatorBalances[1].liabilityShares).toNumber().toString());
    
    for (let i = 0; i < liquidatorBalances.length; i++) {
      if (!liquidatorBalances[i].active)
        continue;
      console.log("Liquidator " + i + " Bank asset tag: " + liquidatorBalances[i].bankAssetTag.toString());
    }

    const insuranceVaultBalance = await getTokenBalance(bankRunProvider, liabilityBankBefore.insuranceVault);
    assert.equal(insuranceVaultBalance, 0);

    const sharesStaked = wrappedI80F48toBigNumber(liquidateeBalances[0].assetShares).toNumber();
    const shareValueStaked = wrappedI80F48toBigNumber(assetBankBefore.assetShareValue).toNumber();
    const sharesSol = wrappedI80F48toBigNumber(liquidateeBalances[1].liabilityShares).toNumber();
    const shareValueSol = wrappedI80F48toBigNumber(liabilityBankBefore.liabilityShareValue).toNumber();
  
    if (verbose) {
      console.log("BEFORE");
      console.log("liability bank insurance vault before: " + insuranceVaultBalance.toLocaleString());
      console.log("user 0 (liquidatee) USDC asset shares: " + sharesStaked.toString());
      console.log("  value (in USDC native): " + (sharesStaked * shareValueStaked).toLocaleString());
      console.log("  value (in dollars): $" + (sharesStaked * shareValueStaked * oracles.usdcPrice / 10 ** (oracles.usdcDecimals)).toLocaleString());
      console.log("user 0 (liquidatee) SOL liability shares: " + sharesSol.toString());
      console.log("  debt (in SOL native): " + (sharesSol * shareValueSol).toLocaleString());
      console.log("  debt (in dollars): $" + (sharesSol * shareValueSol * oracles.wsolPrice / 10 ** (oracles.wsolDecimals)).toLocaleString());
      console.log("user 1 (liquidator) staked asset shares: " + wrappedI80F48toBigNumber(liquidatorBalances[0].assetShares).toString());
//      console.log("user 1 (liquidator) USDC liability shares: " + wrappedI80F48toBigNumber(liquidatorBalances[0].liabilityShares).toString());
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
  
    const solPool = await bankRunProvider.connection.getAccountInfo(
      validators[0].splSolPool
    );
    const solPoolLamports = solPool.lamports;
    console.log("solPoolLamports: " + solPoolLamports.toString());
    const mintData = await getMint(bankRunProvider.connection, validators[0].splMint);
    console.log("mintSupply: " + mintData.supply.toString());
    const stakedPrice = oracles.wsolPrice * (solPoolLamports) / Number(mintData.supply);
    console.log("stakedPrice: " + stakedPrice.toString());

    const stakedLowPrice = stakedPrice * (1 - confidenceInterval); // see top of test
    const wsolHighPrice = oracles.wsolPrice * (1 + confidenceInterval); // see top of test
    const insuranceToBeCollected = (liquidateAmountSol * 0.025 * shareValueStaked * stakedLowPrice / (shareValueSol * wsolHighPrice)) * 10 ** (oracles.wsolDecimals);

    console.log("Asset Bank Key: " + assetBankKey.toString());
    const assetBankFF = await bankrunProgram.account.bank.fetch(assetBankKey);
    console.log("Asset Bank Tag: " + assetBankFF.config.assetTag.toString());
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
    //await banksClient.processTransaction(tx);
    const result = await banksClient.tryProcessTransaction(tx);
    result.meta.logMessages.forEach((msg) => console.log(msg));

    const liquidateeMarginfiAccountAfter = await bankrunProgram.account.marginfiAccount.fetch(liquidateeAccount);
    const liquidatorMarginfiAccountAfter = await bankrunProgram.account.marginfiAccount.fetch(liquidatorAccount);

    const liquidateeBalancesAfter = liquidateeMarginfiAccountAfter.lendingAccount.balances;
    const liquidatorBalancesAfter = liquidatorMarginfiAccountAfter.lendingAccount.balances;

    const sharesUsdcAfter = wrappedI80F48toBigNumber(liquidateeBalancesAfter[0].assetShares).toNumber();
    const sharesSolAfter = wrappedI80F48toBigNumber(liquidateeBalancesAfter[1].liabilityShares).toNumber();

    assertI80F48Equal(liquidateeBalancesAfter[0].assetShares, wrappedI80F48toBigNumber(liquidateeBalances[0].assetShares).toNumber() - liquidateAmountSol_native.toNumber());
    assertI80F48Equal(liquidateeBalancesAfter[0].liabilityShares, 0);
    assertI80F48Equal(liquidateeBalancesAfter[1].assetShares, 0);

    assertI80F48Equal(liquidatorBalancesAfter[0].liabilityShares, 0);
    assertI80F48Equal(liquidatorBalancesAfter[1].assetShares, wrappedI80F48toBigNumber(liquidatorBalances[1].assetShares).toNumber() + liquidateAmountSol_native.toNumber());
    assertI80F48Equal(liquidatorBalancesAfter[1].liabilityShares, 0);

    console.log("liabilityBankBefore.insuranceVault = " + liabilityBankBefore.insuranceVault.toString());
    const insuranceVaultBalanceAfter = await getTokenBalance(bankRunProvider, liabilityBankBefore.insuranceVault);
    console.log("AFTER");
    assert.approximately(insuranceVaultBalanceAfter, insuranceToBeCollected, (insuranceToBeCollected * .1)); // see top of test

    if (verbose) {
      console.log("AFTER");
      console.log("liability bank insurance vault after (SOL): " + insuranceVaultBalanceAfter.toLocaleString());
      console.log("user 0 (liquidatee) USDC asset shares after: " + sharesUsdcAfter.toString());
      console.log("  value (in USDC native): " + (sharesUsdcAfter * shareValueStaked).toLocaleString());
      console.log("  value (in dollars): $" + (sharesUsdcAfter * shareValueStaked * oracles.usdcPrice / 10 ** (oracles.usdcDecimals)).toLocaleString());
      console.log("user 0 (liquidatee) SOL liability shares after: " + sharesSolAfter.toString());
      console.log("  debt (in SOL native): " + (sharesSolAfter * shareValueSol).toLocaleString());
      console.log("  debt (in dollars): $" + (sharesSolAfter * shareValueSol * oracles.wsolPrice / 10 ** (oracles.wsolDecimals)).toLocaleString());
      console.log("user 1 (liquidator) SOL asset shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[0].assetShares).toString());
      console.log("user 1 (liquidator) SOL liability shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[0].liabilityShares).toString());
      console.log("user 1 (liquidator) USDC asset shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[1].assetShares).toString());
      console.log("user 1 (liquidator) USDC liability shares after: " + wrappedI80F48toBigNumber(liquidatorBalancesAfter[1].liabilityShares).toString());
    }

    let now = Math.floor(Date.now() / 1000);
    assertBNApproximately(liquidatorBalancesAfter[0].lastUpdate, now, 20);
    assertBNApproximately(liquidatorBalancesAfter[1].lastUpdate, now, 20);
    assertBNApproximately(liquidateeBalancesAfter[0].lastUpdate, now, 20);
    assertBNApproximately(liquidateeBalancesAfter[1].lastUpdate, now, 20);
  });
});

// TODO: 0,1 - should fail