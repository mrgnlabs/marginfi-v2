import { PublicKey } from "@solana/web3.js";
import BigNumber from "bignumber.js";
import BN from "bn.js";
import { WrappedI8048F } from "./types";
import { nativeToUi, wrappedI80F48toBigNumber } from "./utils";

/**
 * Wrapper class around a specific marginfi group.
 */
class Bank {
  public readonly publicKey: PublicKey;

  public group: PublicKey;
  public mint: PublicKey;
  public mintDecimals: number;

  public depositShareValue: BigNumber;
  public liabilityShareValue: BigNumber;

  public liquidityVault: PublicKey;
  public liquidityVaultBump: number;
  public liquidityVaultAuthorityBump: number;

  public insuranceVault: PublicKey;
  public insuranceVaultBump: number;
  public insuranceVaultAuthorityBump: number;

  public feeVault: PublicKey;
  public feeVaultBump: number;
  public feeVaultAuthorityBump: number;

  public config: BankConfig;

  constructor(address: PublicKey, rawData: BankData) {
    this.publicKey = address;

    this.mint = rawData.mint;
    this.mintDecimals = rawData.mintDecimals;
    this.group = rawData.group;
    console.log(rawData.depositShareValue);

    this.depositShareValue = wrappedI80F48toBigNumber(
      rawData.depositShareValue,
      0
    );
    this.liabilityShareValue = wrappedI80F48toBigNumber(
      rawData.liabilityShareValue,
      0
    );

    this.liquidityVault = rawData.liquidityVault;
    this.liquidityVaultBump = rawData.liquidityVaultBump;
    this.liquidityVaultAuthorityBump = rawData.liquidityVaultAuthorityBump;

    this.insuranceVault = rawData.insuranceVault;
    this.insuranceVaultBump = rawData.insuranceVaultBump;
    this.insuranceVaultAuthorityBump = rawData.insuranceVaultAuthorityBump;

    this.feeVault = rawData.feeVault;
    this.feeVaultBump = rawData.feeVaultBump;
    this.feeVaultAuthorityBump = rawData.feeVaultAuthorityBump;

    this.config = {
      depositWeightInit: wrappedI80F48toBigNumber(
        rawData.config.depositWeightInit,
        0
      ),
      depositWeightMaint: wrappedI80F48toBigNumber(
        rawData.config.depositWeightMaint,
        0
      ),
      liabilityWeightInit: wrappedI80F48toBigNumber(
        rawData.config.liabilityWeightInit,
        0
      ),
      liabilityWeightMaint: wrappedI80F48toBigNumber(
        rawData.config.liabilityWeightMaint,
        0
      ),
      maxCapacity: nativeToUi(rawData.config.maxCapacity, this.mintDecimals),
      pythOracle: rawData.config.pythOracle,
      interestRateConfig: {
        insuranceFeeFixedApr: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.insuranceFeeFixedApr,
          0
        ),
        maxInterestRate: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.maxInterestRate,
          0
        ),
        insuranceIrFee: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.insuranceIrFee,
          0
        ),
        optimalUtilizationRate: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.optimalUtilizationRate,
          0
        ),
        plateauInterestRate: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.optimalUtilizationRate,
          0
        ),
        protocolFixedFeeApr: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.protocolFixedFeeApr,
          0
        ),
        protocolIrFee: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.protocolIrFee,
          0
        ),
      },
    };
  }
}

export default Bank;

// Client types

export interface BankConfig {
  depositWeightInit: BigNumber;
  depositWeightMaint: BigNumber;

  liabilityWeightInit: BigNumber;
  liabilityWeightMaint: BigNumber;

  maxCapacity: number;

  pythOracle: PublicKey;
  interestRateConfig: InterestRateConfig;
}

export interface InterestRateConfig {
  // Curve Params
  optimalUtilizationRate: BigNumber;
  plateauInterestRate: BigNumber;
  maxInterestRate: BigNumber;

  // Fees
  insuranceFeeFixedApr: BigNumber;
  insuranceIrFee: BigNumber;
  protocolFixedFeeApr: BigNumber;
  protocolIrFee: BigNumber;
}

// On-chain types

export interface BankData {
  mint: PublicKey;
  mintDecimals: number;

  group: PublicKey;

  depositShareValue: WrappedI8048F;
  liabilityShareValue: WrappedI8048F;

  liquidityVault: PublicKey;
  liquidityVaultBump: number;
  liquidityVaultAuthorityBump: number;

  insuranceVault: PublicKey;
  insuranceVaultBump: number;
  insuranceVaultAuthorityBump: number;

  feeVault: PublicKey;
  feeVaultBump: number;
  feeVaultAuthorityBump: number;

  config: BankConfigData;

  totalLiabilityShares: WrappedI8048F;
  totalDepositShares: WrappedI8048F;

  lastUpdate: BN;
}

export interface BankConfigData {
  depositWeightInit: WrappedI8048F;
  depositWeightMaint: WrappedI8048F;

  liabilityWeightInit: WrappedI8048F;
  liabilityWeightMaint: WrappedI8048F;

  maxCapacity: BN;

  pythOracle: PublicKey;
  interestRateConfig: InterestRateConfigData;
}

export interface InterestRateConfigData {
  // Curve Params
  optimalUtilizationRate: WrappedI8048F;
  plateauInterestRate: WrappedI8048F;
  maxInterestRate: WrappedI8048F;

  // Fees
  insuranceFeeFixedApr: WrappedI8048F;
  insuranceIrFee: WrappedI8048F;
  protocolFixedFeeApr: WrappedI8048F;
  protocolIrFee: WrappedI8048F;
}
