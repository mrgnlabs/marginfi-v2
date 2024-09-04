import { bigNumberToWrappedI80F48, WrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { PublicKey } from "@solana/web3.js";

import BN from "bn.js";

export const I80F48_ZERO = bigNumberToWrappedI80F48(0);
export const I80F48_ONE = bigNumberToWrappedI80F48(1);

export type RiskTier = { collateral: {} } | { isolated: {} };

export type OperationalState =
  | { paused: {} }
  | { operational: {} }
  | { reduceOnly: {} };

export type OracleSetup =
  | { none: {} }
  | { pythLegacy: {} }
  | { switchboardV2: {} }
  | { pythPushOracle: {} };

export type BankConfig = {
  assetWeightInit: WrappedI80F48;
  assetWeightMaint: WrappedI80F48;

  liabilityWeightInit: WrappedI80F48;
  liabilityWeightMain: WrappedI80F48;

  depositLimit: BN;
  interestRateConfig: InterestRateConfig;

  /** Paused = 0, Operational = 1, ReduceOnly = 2 */
  operationalState: OperationalState;

  /** None = 0, PythLegacy = 1, SwitchboardV2 = 2, PythPushOracle =3 */
  oracleSetup: OracleSetup;
  oracleKey: PublicKey;

  borrowLimit: BN;
  /** Collateral = 0, Isolated = 1 */
  riskTier: RiskTier;
  totalAssetValueInitLimit: BN;
  oracleMaxAge: number;
};

/**
 * The default bank config has
 * * all weights are 1
 * * state = operational, risk tier = collateral
 * * uses the given oracle, assumes it's = pythLegacy
 * * 1_000_000_000 deposit/borrow limit
 * * 100_000_000_000 total asset value limit
 * @returns
 */
export const defaultBankConfig = (oracleKey: PublicKey) => {
  let config: BankConfig = {
    assetWeightInit: I80F48_ONE,
    assetWeightMaint: I80F48_ONE,
    liabilityWeightInit: I80F48_ONE,
    liabilityWeightMain: I80F48_ONE,
    depositLimit: new BN(1_000_000_000),
    interestRateConfig: defaultInterestRateConfig(),
    operationalState: {
      operational: undefined,
    },
    oracleSetup: {
      pythLegacy: undefined,
    },
    oracleKey: oracleKey,
    borrowLimit: new BN(1_000_000_000),
    riskTier: {
      collateral: undefined,
    },
    totalAssetValueInitLimit: new BN(100_000_000_000),
    oracleMaxAge: 100,
  };
  return config;
};

export type InterestRateConfig = {
  optimalUtilizationRate: WrappedI80F48;
  plateauInterestRate: WrappedI80F48;
  maxInterestRate: WrappedI80F48;

  insuranceFeeFixedApr: WrappedI80F48;
  insuranceIrFee: WrappedI80F48;
  protocolFixedFeeApr: WrappedI80F48;
  protocolIrFee: WrappedI80F48;
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
 * @returns
 */
export const defaultInterestRateConfig = () => {
  let config: InterestRateConfig = {
    optimalUtilizationRate: bigNumberToWrappedI80F48(0.5),
    plateauInterestRate: bigNumberToWrappedI80F48(0.6),
    maxInterestRate: bigNumberToWrappedI80F48(3),
    insuranceFeeFixedApr: bigNumberToWrappedI80F48(0.01),
    insuranceIrFee: bigNumberToWrappedI80F48(0.02),
    protocolFixedFeeApr: bigNumberToWrappedI80F48(0.03),
    protocolIrFee: bigNumberToWrappedI80F48(0.04),
  };
  return config;
};
