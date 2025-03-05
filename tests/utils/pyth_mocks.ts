// TODO the Price struct has changed a bit since this copy-pasta was generated some time ago,
// however price and ema price/expo/conf are in the same spot, so if those are all you need, there's
// no need to update (all modern changes are backwards compatible, new versions of Pyth on-chain
// will still deserialize the price data)

// Adapated from PsyLend, Jet labs, etc
import { Program, Wallet, workspace } from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { Oracles, createMockAccount, storeMockAccount } from "./mocks";
import { Mocks } from "../../target/types/mocks";
/** Copied from `@pythnetwork/client": "^2.19.0"`, used as a discriminator */
const Magic = 2712847316;

/**
 * As long as it's large enough, any size is fine.
 */
const PYTH_ACCOUNT_SIZE = 3312;

const mockProgram: Program<Mocks> = workspace.Mocks;

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
 * Creates a Pyth price account
 * @param wallet - pays the TX fee
 * @returns
 */
export const createPriceAccount = async (wallet: Wallet) => {
  return createMockAccount(mockProgram, PYTH_ACCOUNT_SIZE, wallet);
};

/**
 * Creates a Pyth product account
 * @param wallet - pays the TX fee
 * @returns
 */
export const createProductAccount = async (wallet: Wallet) => {
  return createMockAccount(mockProgram, PYTH_ACCOUNT_SIZE, wallet);
};

/**
 * Update a Pyth price account with new data
 * @param account The account to update
 * @param data The new data to place in the account
 * @param wallet - pays tx fee
 */
export const updatePriceAccount = async (
  account: Keypair,
  data: Price,
  wallet: Wallet
) => {
  const buf = Buffer.alloc(512);
  const d = getPythPriceDataWithDefaults(data);
  d.aggregatePriceInfo = getPythPriceInfoWithDefaults(d.aggregatePriceInfo);
  d.twap = getPythEmaWithDefaults(d.twap);

  writePriceBuffer(buf, 0, d);
  await storeMockAccount(mockProgram, wallet, account, 0, buf);
};

/**
 * Update a Pyth product account with new data
 * @param account The account to update
 * @param data The new data to place in the account
 * @param wallet - pays tx fee
 */
export const updateProductAccount = async (
  account: Keypair,
  data: Product,
  wallet: Wallet
) => {
  const buf = Buffer.alloc(512);
  const d = getProductWithDefaults(data);

  writeProductBuffer(buf, 0, d);
  await storeMockAccount(mockProgram, wallet, account, 0, buf);
};

export const getPythPriceDataWithDefaults = ({
  version = 2,
  type = 3, // AccountType::Price
  size = PYTH_ACCOUNT_SIZE,
  priceType = "price",
  exponent = 0,
  currentSlot = BigInt(0),
  validSlot = BigInt(0),
  twap = {},
  productAccountKey = PublicKey.default,
  nextPriceAccountKey = PublicKey.default,
  aggregatePriceUpdaterAccountKey = PublicKey.default,
  aggregatePriceInfo = {},
  priceComponents = [],
}: Price) => {
  return {
    version,
    type,
    size,
    priceType,
    exponent,
    currentSlot,
    validSlot,
    twap,
    productAccountKey,
    nextPriceAccountKey,
    aggregatePriceUpdaterAccountKey,
    aggregatePriceInfo,
    priceComponents,
  };
};

export const getPythPriceInfoWithDefaults = ({
  price = BigInt(0),
  conf = BigInt(0),
  status = 1, // PriceStatus::Trading
  corpAct = 0, // CorpAction::NoCorpAct
  pubSlot = BigInt(Number.MAX_SAFE_INTEGER), // Pubslot has to be newer than current slot.
}: PriceInfo) => {
  return {
    price,
    conf,
    status,
    corpAct,
    pubSlot,
  };
};

export const getPythEmaWithDefaults = ({
  valueComponent = BigInt(0),
  denominator = BigInt(0),
  numerator = BigInt(0),
}: Ema) => {
  return {
    valueComponent,
    denominator,
    numerator,
  };
};

export const writePublicKeyBuffer = (
  buf: Buffer,
  offset: number,
  key: PublicKey
) => {
  buf.write(key.toBuffer().toString("binary"), offset, "binary");
};

export const writePriceInfoBuffer = (
  buf: Buffer,
  offset: number,
  info: PriceInfo
) => {
  buf.writeBigInt64LE(info.price, offset + 0);
  buf.writeBigUInt64LE(info.conf, offset + 8);
  buf.writeUInt32LE(info.status, offset + 16);
  buf.writeUInt32LE(info.corpAct, offset + 20);
  buf.writeBigUInt64LE(info.pubSlot, offset + 24);
};

export const writePriceComponentBuffer = (
  buf: Buffer,
  offset: number,
  component: PriceComponent
) => {
  component.publisher.toBuffer().copy(buf, offset);
  writePriceInfoBuffer(buf, offset + 32, component.agg);
  writePriceInfoBuffer(buf, offset + 64, component.latest);
};

export const writePriceBuffer = (buf: Buffer, offset: number, data: Price) => {
  buf.writeUInt32LE(Magic, offset + 0); //magic
  buf.writeUInt32LE(data.version, offset + 4); //ver
  buf.writeUInt32LE(data.type, offset + 8); //type
  buf.writeUInt32LE(data.size, offset + 12); //size
  buf.writeUInt32LE(1, offset + 16); //price type
  buf.writeInt32LE(data.exponent, offset + 20); //exp
  buf.writeUInt32LE(data.priceComponents.length, offset + 24); //price comps
  buf.writeBigUInt64LE(data.currentSlot, offset + 32); //curr slot
  buf.writeBigUInt64LE(data.validSlot, offset + 40); //valid slot
  buf.writeBigInt64LE(data.twap.valueComponent, offset + 48); //ema
  buf.writeBigInt64LE(data.twap.numerator, offset + 56); //ema
  buf.writeBigInt64LE(data.twap.denominator, offset + 64); //ema
  writePublicKeyBuffer(buf, offset + 112, data.productAccountKey);
  writePublicKeyBuffer(buf, offset + 144, data.nextPriceAccountKey);
  writePublicKeyBuffer(buf, offset + 176, data.aggregatePriceUpdaterAccountKey);

  writePriceInfoBuffer(buf, 208, data.aggregatePriceInfo);

  let pos = offset + 240;
  for (const component of data.priceComponents) {
    writePriceComponentBuffer(buf, pos, component);
    pos += 96;
  }
};

export const getProductWithDefaults = ({
  version = 2,
  atype = 2,
  size = 0,
  priceAccount = PublicKey.default,
  attributes = {},
}: Product) => {
  return {
    version,
    atype,
    size,
    priceAccount,
    attributes,
  };
};

export const writeProductBuffer = (
  buf: Buffer,
  offset: number,
  product: Product
) => {
  let accountSize = product.size;

  if (!accountSize) {
    accountSize = 48;

    for (const key in product.attributes) {
      accountSize += 1 + key.length;
      accountSize += 1 + product.attributes[key].length;
    }
  }

  buf.writeUInt32LE(Magic, offset + 0);
  buf.writeUInt32LE(product.version, offset + 4);
  buf.writeUInt32LE(product.atype, offset + 8);
  buf.writeUInt32LE(accountSize, offset + 12);

  writePublicKeyBuffer(buf, offset + 16, product.priceAccount);

  let pos = offset + 48;

  for (const key in product.attributes) {
    buf.writeUInt8(key.length, pos);
    buf.write(key, pos + 1);

    pos += 1 + key.length;

    const value = product.attributes[key];
    buf.writeUInt8(value.length, pos);
    buf.write(value, pos + 1);
  }
};

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
  verbose: boolean,
  skips?: {
    wsol: boolean;
    usdc: boolean;
    a: boolean;
    b: boolean;
  }
) => {
  let wsolPythOracle = await createPriceAccount(wallet);
  let price = BigInt(wsolPrice * 10 ** wsolDecimals);
  if (skips && skips.wsol) {
    // do nothing
  } else {
    await updatePriceAccount(
      wsolPythOracle,
      {
        exponent: -wsolDecimals,
        aggregatePriceInfo: {
          price: price,
          conf: price / BigInt(100), // 1% of the price
        },
        twap: {
          valueComponent: price,
        },
      },
      wallet
    );
  }

  let usdcPythOracle = await createPriceAccount(wallet);
  price = BigInt(usdcPrice * 10 ** usdcDecimals);
  if (skips && skips.usdc) {
    // do nothing
  } else {
    await updatePriceAccount(
      usdcPythOracle,
      {
        exponent: -usdcDecimals,
        aggregatePriceInfo: {
          price: price,
          conf: price / BigInt(100), // 1% of the price
        },
        twap: {
          valueComponent: price,
        },
      },
      wallet
    );
  }

  let fakeUsdcPythOracle = await createPriceAccount(wallet);
  price = BigInt(usdcPrice * 10 ** usdcDecimals);
  if (skips && skips.usdc) {
    // do nothing
  } else {
    await updatePriceAccount(
      fakeUsdcPythOracle,
      {
        exponent: -usdcDecimals,
        aggregatePriceInfo: {
          price: price,
          conf: price / BigInt(100), // 1% of the price
        },
        twap: {
          valueComponent: price,
        },
      },
      wallet
    );
  }

  let tokenAPythOracle = await createPriceAccount(wallet);
  price = BigInt(tokenAPrice * 10 ** tokenADecimals);
  if (skips && skips.a) {
    // do nothing
  } else {
    await updatePriceAccount(
      tokenAPythOracle,
      {
        exponent: -tokenADecimals,
        aggregatePriceInfo: {
          price: price,
          conf: price / BigInt(100), // 1% of the price
        },
        twap: {
          valueComponent: price,
        },
      },
      wallet
    );
  }

  let tokenBPythOracle = await createPriceAccount(wallet);
  price = BigInt(tokenBPrice * 10 ** tokenBDecimals);
  if (skips && skips.b) {
    // do nothing
  } else {
    await updatePriceAccount(
      tokenBPythOracle,
      {
        exponent: -tokenBDecimals,
        aggregatePriceInfo: {
          price: price,
          conf: price / BigInt(100), // 1% of the price
        },
        twap: {
          valueComponent: price,
        },
      },
      wallet
    );
  }

  if (verbose) {
    console.log("Mock Pyth price oracles:");
    console.log("wsol price:    \t" + wsolPythOracle.publicKey);
    console.log("usdc price:    \t" + usdcPythOracle.publicKey);
    console.log("token a price: \t" + tokenAPythOracle.publicKey);
    console.log("token b price: \t" + tokenBPythOracle.publicKey);
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
    console.log("");
  }
  let oracles: Oracles = {
    wsolOracle: wsolPythOracle,
    wsolDecimals: wsolDecimals,
    usdcOracle: usdcPythOracle,
    usdcDecimals: usdcDecimals,
    tokenAOracle: tokenAPythOracle,
    tokenADecimals: tokenADecimals,
    tokenBOracle: tokenBPythOracle,
    tokenBDecimals: tokenBDecimals,
    wsolPrice: wsolPrice,
    usdcPrice: usdcPrice,
    tokenAPrice: tokenAPrice,
    tokenBPrice: tokenBPrice,
    fakeUsdc: fakeUsdcPythOracle.publicKey,
  };
  return oracles;
};
