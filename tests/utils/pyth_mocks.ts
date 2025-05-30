// TODO the Price struct has changed a bit since this copy-pasta was generated some time ago,
// however price and ema price/expo/conf are in the same spot, so if those are all you need, there's
// no need to update (all modern changes are backwards compatible, new versions of Pyth on-chain
// will still deserialize the price data)

// Adapated from PsyLend, Jet labs, etc
import { BN, Program, Wallet, workspace } from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { Oracles, createMockAccount, storeMockAccount } from "./mocks";
import { Mocks } from "../../target/types/mocks";
import {
  initBlankOracleFeed,
  initOrUpdatePriceUpdateV2,
} from "./pyth-pull-mocks";
import { ORACLE_CONF_INTERVAL } from "./types";
/** Copied from `@pythnetwork/client": "^2.19.0"`, used as a discriminator */
const Magic = 2712847316;

/**
 * As long as it's large enough, any size is fine.
 */
const PYTH_ACCOUNT_SIZE = 3312;

export interface Price {
  version?: number;
  type?: number;
  size?: number;
  priceType?: string;
  exponent?: number;
  currentSlot?: bigint;
  validSlot?: bigint;
  twap?: Ema;
  productAccountKey?: PublicKey;
  nextPriceAccountKey?: PublicKey;
  aggregatePriceUpdaterAccountKey?: PublicKey;
  aggregatePriceInfo?: PriceInfo;
  priceComponents?: PriceComponent[];
}

export interface PriceInfo {
  price?: bigint;
  conf?: bigint;
  status?: number;
  corpAct?: number;
  pubSlot?: bigint;
}

export interface PriceComponent {
  publisher?: PublicKey;
  agg?: PriceInfo;
  latest?: PriceInfo;
}

export interface Product {
  version?: number;
  atype?: number;
  size?: number;
  priceAccount?: PublicKey;
  attributes?: Record<string, string>;
}

export interface Ema {
  valueComponent?: bigint;
  numerator?: bigint;
  denominator?: bigint;
}

/**
 * Set up mock usdc and wsol oracles
 * @param wallet
 * @param wsolPrice
 * @param wsolDecimals
 * @param usdcPrice
 * @param usdcDecimals
 * @param tokenAPrice:
 * @param tokenADecimals:
 * @param tokenBPrice:
 * @param tokenBDecimals:
 * @param verbose
 * @param skips - set to true to skip sending txes, which makes tests run faster if you don't need
 * those oracles.
 * @returns Price oracles for all currencies
 */
export const setupPythOracles = async (
  wallet: Wallet,
  wsolPrice: number,
  wsolDecimals: number,
  usdcPrice: number,
  usdcDecimals: number,
  tokenAPrice: number,
  tokenADecimals: number,
  tokenBPrice: number,
  tokenBDecimals: number,
  lstAlphaPrice: number,
  lstAlphaDecimals: number,
  verbose: boolean,
  skips?: {
    wsol: boolean;
    usdc: boolean;
    a: boolean;
    b: boolean;
    wsolPyth: boolean;
  }
) => {
  let wsolPythPullOracle = Keypair.generate();
  let wsolPythPullOracleFeed = Keypair.generate();
  let wsolNativePrice = wsolPrice * 10 ** wsolDecimals;
  let wsolConfidence = wsolNativePrice * ORACLE_CONF_INTERVAL;
  if (skips && skips.wsol) {
    // do nothing
  } else {
    wsolPythPullOracleFeed = await initBlankOracleFeed(wallet);
    wsolPythPullOracle = await initOrUpdatePriceUpdateV2(
      wallet,
      wsolPythPullOracleFeed.publicKey,
      new BN(wsolNativePrice),
      new BN(wsolConfidence),
      new BN(wsolNativePrice),
      new BN(wsolConfidence),
      new BN(0),
      -wsolDecimals
    );
  }

  let usdcPythPullOracle = Keypair.generate();
  let usdcPythPullOracleFeed = Keypair.generate();
  let usdcNativePrice = usdcPrice * 10 ** usdcDecimals;
  let usdcConfidence = usdcNativePrice * ORACLE_CONF_INTERVAL;
  if (skips && skips.usdc) {
    // do nothing
  } else {
    usdcPythPullOracleFeed = await initBlankOracleFeed(wallet);
    usdcPythPullOracle = await initOrUpdatePriceUpdateV2(
      wallet,
      usdcPythPullOracleFeed.publicKey,
      new BN(usdcNativePrice),
      new BN(usdcConfidence),
      new BN(usdcNativePrice),
      new BN(usdcConfidence),
      new BN(0),
      -usdcDecimals
    );
  }

  let fakeUsdcPythPullOracle = Keypair.generate();
  let fakeUsdcPythPullOracleFeed = Keypair.generate();
  let fakeUsdcNativePrice = usdcPrice * 10 ** usdcDecimals;
  let fakeUsdcConfidence = fakeUsdcNativePrice * ORACLE_CONF_INTERVAL;
  if (skips && skips.usdc) {
    // do nothing
  } else {
    fakeUsdcPythPullOracleFeed = await initBlankOracleFeed(wallet);
    fakeUsdcPythPullOracle = await initOrUpdatePriceUpdateV2(
      wallet,
      fakeUsdcPythPullOracleFeed.publicKey,
      new BN(fakeUsdcNativePrice),
      new BN(fakeUsdcConfidence),
      new BN(fakeUsdcNativePrice),
      new BN(fakeUsdcConfidence),
      new BN(0),
      -usdcDecimals
    );
  }

  let tokenAPythPullOracle = Keypair.generate();
  let tokenAPythPullOracleFeed = Keypair.generate();
  let tokenANativePrice = tokenAPrice * 10 ** tokenADecimals;
  let tokenAConfidence = tokenANativePrice * ORACLE_CONF_INTERVAL;
  if (skips && skips.a) {
    // do nothing
  } else {
    tokenAPythPullOracleFeed = await initBlankOracleFeed(wallet);
    tokenAPythPullOracle = await initOrUpdatePriceUpdateV2(
      wallet,
      tokenAPythPullOracleFeed.publicKey,
      new BN(tokenANativePrice),
      new BN(tokenAConfidence),
      new BN(tokenANativePrice),
      new BN(tokenAConfidence),
      new BN(0),
      -tokenADecimals
    );
  }

  let tokenBPythPullOracle = Keypair.generate();
  let tokenBPythPullOracleFeed = Keypair.generate();
  let tokenBNativePrice = tokenBPrice * 10 ** tokenBDecimals;
  let tokenBConfidence = tokenBNativePrice * ORACLE_CONF_INTERVAL;
  if (skips && skips.b) {
    // do nothing
  } else {
    tokenBPythPullOracleFeed = await initBlankOracleFeed(wallet);
    tokenBPythPullOracle = await initOrUpdatePriceUpdateV2(
      wallet,
      tokenBPythPullOracleFeed.publicKey,
      new BN(tokenBNativePrice),
      new BN(tokenBConfidence),
      new BN(tokenBNativePrice),
      new BN(tokenBConfidence),
      new BN(0),
      -tokenBDecimals
    );
  }

  let lstPythPullOracle = Keypair.generate();
  let lstPythPullOracleFeed = Keypair.generate();
  let priceAlpha = lstAlphaPrice * 10 ** lstAlphaDecimals;
  let confAlpha = priceAlpha * ORACLE_CONF_INTERVAL;
  if (skips && skips.wsolPyth) {
    // do nothing
  } else {
    lstPythPullOracleFeed = await initBlankOracleFeed(wallet);
    lstPythPullOracle = await initOrUpdatePriceUpdateV2(
      wallet,
      lstPythPullOracleFeed.publicKey,
      new BN(priceAlpha),
      new BN(confAlpha),
      new BN(priceAlpha),
      new BN(confAlpha),
      new BN(0),
      -lstAlphaDecimals
    );
  }

  if (verbose) {
    console.log("Mock Pyth Pull price oracles:");
    console.log("wsol:    \t" + wsolPythPullOracle.publicKey);
    console.log("usdc:    \t" + usdcPythPullOracle.publicKey);
    console.log("token a: \t" + tokenAPythPullOracle.publicKey);
    console.log("token b: \t" + tokenBPythPullOracle.publicKey);
    console.log("lst:     \t" + lstPythPullOracle.publicKey);
    console.log(
      "Price of 1 wsol.......$" +
        wsolPrice +
        "\t  one token in native decimals: " +
        (1 * 10 ** wsolDecimals).toLocaleString()
    );
    console.log(
      "Price of 1 usdc.......$" +
        usdcPrice +
        "\t  one token in native decimals: " +
        (1 * 10 ** usdcDecimals).toLocaleString()
    );
    console.log(
      "Price of 1 token A....$" +
        tokenAPrice +
        "\t  one token in native decimals: " +
        (1 * 10 ** tokenADecimals).toLocaleString()
    );
    console.log(
      "Price of 1 token B....$" +
        tokenBPrice +
        "\t  one token in native decimals: " +
        (1 * 10 ** tokenBDecimals).toLocaleString()
    );
    console.log(
      "Price of 1 LST alpha..$" +
        lstAlphaPrice +
        "\t  one token in native decimals: " +
        (1 * 10 ** lstAlphaDecimals).toLocaleString()
    );
    console.log("");
  }
  let oracles: Oracles = {
    wsolOracle: wsolPythPullOracle,
    wsolOracleFeed: wsolPythPullOracleFeed,
    wsolDecimals: wsolDecimals,
    usdcOracle: usdcPythPullOracle,
    usdcOracleFeed: usdcPythPullOracleFeed,
    usdcDecimals: usdcDecimals,
    tokenAOracle: tokenAPythPullOracle,
    tokenAOracleFeed: tokenAPythPullOracleFeed,
    tokenADecimals: tokenADecimals,
    tokenBOracle: tokenBPythPullOracle,
    tokenBOracleFeed: tokenBPythPullOracleFeed,
    tokenBDecimals: tokenBDecimals,
    wsolPrice: wsolPrice,
    usdcPrice: usdcPrice,
    tokenAPrice: tokenAPrice,
    tokenBPrice: tokenBPrice,
    lstAlphaPrice: lstAlphaPrice,
    lstAlphaDecimals: lstAlphaDecimals,
    fakeUsdc: fakeUsdcPythPullOracle.publicKey,
    fakeUsdcFeed: fakeUsdcPythPullOracleFeed.publicKey,
    pythPullLst: lstPythPullOracle,
    pythPullLstOracleFeed: lstPythPullOracleFeed,
  };
  return oracles;
};
