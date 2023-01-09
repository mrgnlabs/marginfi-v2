import { PublicKey } from "@solana/web3.js";
import BigNumber from "bignumber.js";
import BN from "bn.js";
import { MarginRequirementType } from "./account";
import { WrappedI80F48 } from "./types";
import { nativeToUi, wrappedI80F48toBigNumber } from "./utils";
import { Connection } from "@solana/web3.js";
import { PYTH_PRICE_CONF_INTERVALS, USDC_DECIMALS } from "./constants";
import { PriceData, parsePriceData } from "@pythnetwork/client";

/**
 * Wrapper class around a specific marginfi group.
 */
class Bank {
  public readonly publicKey: PublicKey;

  public readonly label: string;

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

  private priceData: PriceData;

  constructor(
    label: string,
    address: PublicKey,
    rawData: BankData,
    priceData: PriceData
  ) {
    this.label = label;
    this.publicKey = address;

    this.mint = rawData.mint;
    this.mintDecimals = rawData.mintDecimals;
    this.group = rawData.group;

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

    this.priceData = priceData;
  }

  public async reloadPriceData(connection: Connection) {
    const pythPriceAccount = await connection.getAccountInfo(
      this.config.pythOracle
    );
    this.priceData = parsePriceData(pythPriceAccount!.data);
  }

  public getDepositValue(depositShares: BigNumber): BigNumber {
    return depositShares.times(this.depositShareValue);
  }

  public getLiabilityValue(liabilityShares: BigNumber): BigNumber {
    return liabilityShares.times(this.liabilityShareValue);
  }

  public getDepositShares(depositValue: BigNumber): BigNumber {
    return depositValue.div(this.depositShareValue);
  }

  public getLiabilityShares(liabilityValue: BigNumber): BigNumber {
    return liabilityValue.div(this.liabilityShareValue);
  }

  public getDepositUsdValue(
    depositShares: BigNumber,
    marginRequirementType: MarginRequirementType,
    priceBias: PriceBias
  ): BigNumber {
    const depositValue = this.getDepositValue(depositShares);
    const price = this.getPrice(priceBias);
    const weight = this.getDepositWeight(marginRequirementType);

    console.log(
      "value: %s, price: %s, weight: %s",
      depositValue,
      price,
      weight
    );

    return depositValue
      .times(price)
      .div(new BigNumber(10).pow(USDC_DECIMALS))
      .times(weight);
  }

  public getLiabilityUsdValue(
    liabilityShares: BigNumber,
    marginRequirementType: MarginRequirementType,
    priceBias: PriceBias
  ): BigNumber {
    const liabilityValue = this.getLiabilityValue(liabilityShares);
    const price = this.getPrice(priceBias);
    const weight = this.getLiabilityWeight(marginRequirementType);

    console.log(
      "value: %s, price: %s, weight: %s",
      liabilityValue,
      price,
      weight
    );

    return liabilityValue.times(price).times(weight);
  }

  public getPrice(priceBias: PriceBias): BigNumber {
    const basePrice = this.priceData.emaPrice;
    const confidenceRange = this.priceData.emaConfidence;

    const basePriceVal = new BigNumber(basePrice.value);
    const confidenceRangeVal = new BigNumber(confidenceRange.value).times(
      PYTH_PRICE_CONF_INTERVALS
    );

    switch (priceBias) {
      case PriceBias.Lowest:
        return basePriceVal.minus(confidenceRangeVal);
      case PriceBias.Highest:
        return basePriceVal.plus(confidenceRangeVal);
      case PriceBias.Average:
        return basePriceVal;
    }
  }

  // Return deposit weight based on margin requirement types
  public getDepositWeight(
    marginRequirementType: MarginRequirementType
  ): BigNumber {
    switch (marginRequirementType) {
      case MarginRequirementType.Init:
        return this.config.depositWeightInit;
      case MarginRequirementType.Maint:
        return this.config.depositWeightMaint;
      case MarginRequirementType.Equity:
        return new BigNumber(1);
      default:
        throw new Error("Invalid margin requirement type");
    }
  }

  public getLiabilityWeight(
    marginRequirementType: MarginRequirementType
  ): BigNumber {
    switch (marginRequirementType) {
      case MarginRequirementType.Init:
        return this.config.liabilityWeightInit;
      case MarginRequirementType.Maint:
        return this.config.liabilityWeightMaint;
      case MarginRequirementType.Equity:
        return new BigNumber(0);
      default:
        throw new Error("Invalid margin requirement type");
    }
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

  depositShareValue: WrappedI80F48;
  liabilityShareValue: WrappedI80F48;

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

  totalLiabilityShares: WrappedI80F48;
  totalDepositShares: WrappedI80F48;

  lastUpdate: BN;
}

export interface BankConfigData {
  depositWeightInit: WrappedI80F48;
  depositWeightMaint: WrappedI80F48;

  liabilityWeightInit: WrappedI80F48;
  liabilityWeightMaint: WrappedI80F48;

  maxCapacity: BN;

  pythOracle: PublicKey;
  interestRateConfig: InterestRateConfigData;
}

export interface InterestRateConfigData {
  // Curve Params
  optimalUtilizationRate: WrappedI80F48;
  plateauInterestRate: WrappedI80F48;
  maxInterestRate: WrappedI80F48;

  // Fees
  insuranceFeeFixedApr: WrappedI80F48;
  insuranceIrFee: WrappedI80F48;
  protocolFixedFeeApr: WrappedI80F48;
  protocolIrFee: WrappedI80F48;
}

export enum PriceBias {
  Lowest = 0,
  Average = 1,
  Highest = 2,
}

function priceComponentsToBigNumber(price: BigNumber, exp: number): BigNumber {
  return price.times(new BigNumber(10).pow(exp - USDC_DECIMALS));
}
