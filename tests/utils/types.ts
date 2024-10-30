import {
  BankConfigOpt,
  BankConfigOptRaw,
  InterestRateConfig,
  InterestRateConfigRaw,
  OperationalState,
  OracleSetupRaw,
  RiskTier,
  RiskTierRaw,
} from "@mrgnlabs/marginfi-client-v2";
import { bigNumberToWrappedI80F48, WrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { PublicKey } from "@solana/web3.js";
import BigNumber from "bignumber.js";

import BN from "bn.js";

export const I80F48_ZERO = bigNumberToWrappedI80F48(0);
export const I80F48_ONE = bigNumberToWrappedI80F48(1);
/** Equivalent in value to u64::MAX in Rust */
export const u64MAX_BN = new BN("18446744073709551615");
export const SINGLE_POOL_PROGRAM_ID = new PublicKey(
  "SVSPxpvHdN29nkVg9rPapPNDddN5DipNLRUFhyjFThE"
);

export const EMISSIONS_FLAG_NONE = 0;
export const EMISSIONS_FLAG_BORROW_ACTIVE = 1;
export const EMISSIONS_FLAG_LENDING_ACTIVE = 2;

export const ASSET_TAG_DEFAULT = 0;
export const ASSET_TAG_SOL = 1;
export const ASSET_TAG_STAKED = 2;

/**
 * The default bank config has
 * * all weights are 1
 * * state = operational, risk tier = collateral
 * * uses the given oracle, assumes it's = pythLegacy
 * * 100_000_000_000 deposit/borrow limit
 * * 1_000_000_000_000 total asset value limit
 * * asset tag default (`ASSET_TAG_DEFAULT`)
 * @returns
 */
export const defaultBankConfig = (oracleKey: PublicKey) => {
  let config: BankConfig = {
    assetWeightInit: I80F48_ONE,
    assetWeightMaint: I80F48_ONE,
    liabilityWeightInit: I80F48_ONE,
    liabilityWeightMain: I80F48_ONE,
    depositLimit: new BN(100_000_000_000),
    interestRateConfig: defaultInterestRateConfigRaw(),
    operationalState: {
      operational: undefined,
    },
    oracleSetup: {
      pythLegacy: undefined,
    },
    oracleKey: oracleKey,
    borrowLimit: new BN(100_000_000_000),
    riskTier: {
      collateral: undefined,
    },
    assetTag: ASSET_TAG_DEFAULT,
    totalAssetValueInitLimit: new BN(1_000_000_000_000),
    oracleMaxAge: 100,
  };
  return config;
};

/**
 * The same parameters as `defaultBankConfig`, and no change to oracle
 * @returns
 */
export const defaultBankConfigOpt = () => {
  let bankConfigOpt: BankConfigOpt = {
    assetWeightInit: new BigNumber(1),
    assetWeightMaint: new BigNumber(1),
    liabilityWeightInit: new BigNumber(1),
    liabilityWeightMaint: new BigNumber(1),
    depositLimit: new BigNumber(1_000_000_000),
    borrowLimit: new BigNumber(1_000_000_000),
    riskTier: RiskTier.Collateral,
    totalAssetValueInitLimit: new BigNumber(100_000_000_000),
    interestRateConfig: defaultInterestRateConfig(),
    operationalState: OperationalState.Operational,
    oracle: null,
    oracleMaxAge: 100,
    permissionlessBadDebtSettlement: null,
  };

  return bankConfigOpt;
};

/**
 * The same parameters as `defaultBankConfig`, and no change to oracle
 * @returns
 */
export const defaultBankConfigOptRaw = () => {
  let bankConfigOpt: BankConfigOptWithAssetTag = {
    assetWeightInit: I80F48_ONE,
    assetWeightMaint: I80F48_ONE,
    liabilityWeightInit: I80F48_ONE,
    liabilityWeightMaint: I80F48_ONE,
    depositLimit: new BN(1_000_000_000),
    borrowLimit: new BN(1_000_000_000),
    riskTier: {
      collateral: undefined,
    },
    assetTag: ASSET_TAG_DEFAULT,
    totalAssetValueInitLimit: new BN(100_000_000_000),
    interestRateConfig: defaultInterestRateConfigRaw(),
    operationalState: {
      operational: undefined,
    },
    oracle: null,
    oracleMaxAge: 100,
    permissionlessBadDebtSettlement: null,
  };

  return bankConfigOpt;
};

/**
 * The default interest config has
 * * optimalUtilizationRate = .5
 * * plateauInterestRate = .6
 * * maxInterestRate = 3
 * * insuranceFeeFixedApr = .01
 * * insuranceIrFee = .02
 * * protocolFixedFeeApr = .03
 * * protocolIrFee = .04
 * * originationFee = .01
 * @returns
 */
export const defaultInterestRateConfigRaw = () => {
  let config: InterestRateConfigRawWithOrigination = {
    optimalUtilizationRate: bigNumberToWrappedI80F48(0.5),
    plateauInterestRate: bigNumberToWrappedI80F48(0.6),
    maxInterestRate: bigNumberToWrappedI80F48(3),
    insuranceFeeFixedApr: bigNumberToWrappedI80F48(0.01),
    insuranceIrFee: bigNumberToWrappedI80F48(0.02),
    protocolFixedFeeApr: bigNumberToWrappedI80F48(0.03),
    protocolIrFee: bigNumberToWrappedI80F48(0.04),
    protocolOriginationFee: bigNumberToWrappedI80F48(0.01),
  };
  return config;
};

/**
 * The same parameters as `defaultInterestRateConfigRaw`
 * @returns
 */
export const defaultInterestRateConfig = () => {
  let config: InterestRateConfigWithOrigination = {
    optimalUtilizationRate: new BigNumber(0.5),
    plateauInterestRate: new BigNumber(0.6),
    maxInterestRate: new BigNumber(3),
    insuranceFeeFixedApr: new BigNumber(0),
    insuranceIrFee: new BigNumber(0),
    protocolFixedFeeApr: new BigNumber(0),
    protocolIrFee: new BigNumber(0),
    protocolOriginationFee: new BigNumber(0.1),
  };
  return config;
};

export const defaultStakedInterestSettings = (oracle: PublicKey) => {
  let settings: StakedSettingsConfig = {
    oracle: oracle,
    assetWeightInit: bigNumberToWrappedI80F48(0.8),
    assetWeightMaint: bigNumberToWrappedI80F48(0.9),
    depositLimit: new BN(1_000_000_000_000), // 1000 SOL
    totalAssetValueInitLimit: new BN(150_000_000),
    oracleMaxAge: 10,
    riskTier: {
      collateral: undefined,
    },
  };
  return settings;
};

// TODO remove when package updates
export type BankConfigOptWithAssetTag = BankConfigOptRaw & {
  assetTag: number | null;
};

// TODO remove when package updates
export type InterestRateConfigRawWithOrigination = InterestRateConfigRaw & {
  protocolOriginationFee: WrappedI80F48;
};

// TODO remove when package updates
export type InterestRateConfigWithOrigination = InterestRateConfig & {
  protocolOriginationFee: BigNumber;
};

// TODO remove when package updates
type OperationalStateRaw =
  | { paused: {} }
  | { operational: {} }
  | { reduceOnly: {} };

// TODO remove when package updates
export type BankConfig = {
  assetWeightInit: WrappedI80F48;
  assetWeightMaint: WrappedI80F48;

  liabilityWeightInit: WrappedI80F48;
  liabilityWeightMain: WrappedI80F48;

  depositLimit: BN;
  interestRateConfig: InterestRateConfigRaw;

  /** Paused = 0, Operational = 1, ReduceOnly = 2 */
  operationalState: OperationalStateRaw;

  /** None = 0, PythLegacy = 1, SwitchboardV2 = 2, PythPushOracle =3 */
  oracleSetup: OracleSetupRaw;
  oracleKey: PublicKey;

  borrowLimit: BN;
  /** Collateral = 0, Isolated = 1 */
  riskTier: RiskTierRaw;
  assetTag: number;
  totalAssetValueInitLimit: BN;
  oracleMaxAge: number;
};

// TODO remove when package updates
export type StakedSettingsConfig = {
  oracle: PublicKey;

  assetWeightInit: WrappedI80F48;
  assetWeightMaint: WrappedI80F48;

  depositLimit: BN;
  totalAssetValueInitLimit: BN;

  oracleMaxAge: number;
  /** Collateral = 0, Isolated = 1 */
  riskTier: RiskTierRaw;
};

export interface StakedSettingsEdit {
  oracle: PublicKey | null;

  assetWeightInit: WrappedI80F48 | null;
  assetWeightMaint: WrappedI80F48 | null;

  depositLimit: BN | null;
  totalAssetValueInitLimit: BN | null;

  oracleMaxAge: number | null;
  riskTier: { collateral: {} } | { isolated: {} } | null;
}
