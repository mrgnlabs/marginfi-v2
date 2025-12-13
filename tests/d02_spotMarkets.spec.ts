import { assert } from "chai";
import { PublicKey, Transaction, Keypair } from "@solana/web3.js";
import {
  ecosystem,
  groupAdmin,
  oracles,
  bankrunContext,
  driftBankrunProgram,
  driftAccounts,
  DRIFT_USDC_SPOT_MARKET,
  DRIFT_TOKENA_SPOT_MARKET,
  DRIFT_TOKENA_PULL_ORACLE,
  DRIFT_TOKENA_PULL_FEED,
  banksClient,
} from "./rootHooks";
import { processBankrunTransaction } from "./utils/tools";
import { assertKeysEqual, assertKeyDefault } from "./utils/genericTests";
import { makeInitializeSpotMarketIx } from "./utils/drift-sdk";
import { deriveSpotMarketPDA } from "./utils/pdas";
import {
  getSpotMarketAccount,
  getDriftStateAccount,
  quoteAssetSpotMarketConfig,
  defaultSpotMarketConfig,
  DriftOracleSourceValues,
  USDC_MARKET_INDEX,
  TOKEN_A_MARKET_INDEX,
} from "./utils/drift-utils";
import {
  DRIFT_ORACLE_RECEIVER_PROGRAM_ID,
} from "./utils/types";
import { createBankrunPythOracleAccount } from "./utils/bankrun-oracles";
import { refreshDriftOracles } from "./utils/drift-utils";

describe("d02: Drift - Initialize Spot Markets", () => {
  before(async () => {});

  it("Initialize USDC spot market (index 0)", async () => {
    const config = quoteAssetSpotMarketConfig();

    const initUsdcMarketIx = await makeInitializeSpotMarketIx(
      driftBankrunProgram,
      {
        admin: groupAdmin.wallet.publicKey,
        spotMarketMint: ecosystem.usdcMint.publicKey,
        oracle: PublicKey.default,
      },
      {
        optimalUtilization: config.optimalUtilization,
        optimalRate: config.optimalRate,
        maxRate: config.maxRate,
        oracleSource: DriftOracleSourceValues.quoteAsset,
        initialAssetWeight: config.initialAssetWeight,
        maintenanceAssetWeight: config.maintenanceAssetWeight,
        initialLiabilityWeight: config.initialLiabilityWeight,
        maintenanceLiabilityWeight: config.maintenanceLiabilityWeight,
        marketIndex: USDC_MARKET_INDEX,
      }
    );

    const tx = new Transaction().add(initUsdcMarketIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [groupAdmin.wallet],
      false,
      true
    );

    const usdcMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );

    assert.ok(usdcMarket);
    assert.equal(usdcMarket.marketIndex, USDC_MARKET_INDEX);
    assertKeysEqual(usdcMarket.mint, ecosystem.usdcMint.publicKey);
    assertKeyDefault(usdcMarket.oracle);
    assert.deepStrictEqual(
      usdcMarket.oracleSource,
      DriftOracleSourceValues.quoteAsset
    );

    const state = await getDriftStateAccount(driftBankrunProgram);
    assert.equal(state.numberOfSpotMarkets, 1);

    const [usdcMarketPDA] = deriveSpotMarketPDA(
      driftBankrunProgram.programId,
      USDC_MARKET_INDEX
    );
    driftAccounts.set(DRIFT_USDC_SPOT_MARKET, usdcMarketPDA);
  });

  it("Initialize Token A spot market (index 1) with Pyth Pull oracle", async () => {
    const config = defaultSpotMarketConfig();

    const driftTokenAPullOracle = Keypair.generate();
    const driftTokenAPullFeed = Keypair.generate();

    await createBankrunPythOracleAccount(
      bankrunContext,
      banksClient,
      driftTokenAPullOracle,
      DRIFT_ORACLE_RECEIVER_PROGRAM_ID
    );

    driftAccounts.set(
      DRIFT_TOKENA_PULL_ORACLE,
      driftTokenAPullOracle.publicKey
    );
    driftAccounts.set(DRIFT_TOKENA_PULL_FEED, driftTokenAPullFeed.publicKey);

    await refreshDriftOracles(
      oracles,
      driftAccounts,
      bankrunContext,
      banksClient
    );

    const initTokenAMarketIx = await makeInitializeSpotMarketIx(
      driftBankrunProgram,
      {
        admin: groupAdmin.wallet.publicKey,
        spotMarketMint: ecosystem.tokenAMint.publicKey,
        oracle: driftTokenAPullOracle.publicKey,
      },
      {
        optimalUtilization: config.optimalUtilization,
        optimalRate: config.optimalRate,
        maxRate: config.maxRate,
        oracleSource: DriftOracleSourceValues.pythPull,
        initialAssetWeight: config.initialAssetWeight,
        maintenanceAssetWeight: config.maintenanceAssetWeight,
        initialLiabilityWeight: config.initialLiabilityWeight,
        maintenanceLiabilityWeight: config.maintenanceLiabilityWeight,
        marketIndex: TOKEN_A_MARKET_INDEX,
      }
    );

    const tx = new Transaction().add(initTokenAMarketIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [groupAdmin.wallet],
      false,
      true
    );

    const tokenAMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );

    assert.ok(tokenAMarket);
    assert.equal(tokenAMarket.marketIndex, TOKEN_A_MARKET_INDEX);
    assertKeysEqual(tokenAMarket.mint, ecosystem.tokenAMint.publicKey);
    assertKeysEqual(tokenAMarket.oracle, driftTokenAPullOracle.publicKey);
    assert.deepStrictEqual(
      tokenAMarket.oracleSource,
      DriftOracleSourceValues.pythPull,
      "Oracle source should be PYTH_PULL"
    );

    assert.equal(tokenAMarket.optimalUtilization, config.optimalUtilization);
    assert.equal(tokenAMarket.optimalBorrowRate, config.optimalRate);
    assert.equal(tokenAMarket.maxBorrowRate, config.maxRate);

    const state = await getDriftStateAccount(driftBankrunProgram);
    assert.equal(state.numberOfSpotMarkets, 2);

    const [tokenAMarketPDA] = deriveSpotMarketPDA(
      driftBankrunProgram.programId,
      TOKEN_A_MARKET_INDEX
    );
    driftAccounts.set(DRIFT_TOKENA_SPOT_MARKET, tokenAMarketPDA);
  });

  it("Verify spot market PDAs are correctly derived", async () => {
    const usdcMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );
    assert.ok(usdcMarket);

    const tokenAMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );
    assert.ok(tokenAMarket);
  });
});
