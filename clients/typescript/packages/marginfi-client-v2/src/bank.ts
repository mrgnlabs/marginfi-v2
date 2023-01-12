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

  public totalDepositShares: BigNumber;
  public totalLiabilityShares: BigNumber;

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
      rawData.depositShareValue
    );
    this.liabilityShareValue = wrappedI80F48toBigNumber(
      rawData.liabilityShareValue
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
        rawData.config.depositWeightInit
      ),
      depositWeightMaint: wrappedI80F48toBigNumber(
        rawData.config.depositWeightMaint
      ),
      liabilityWeightInit: wrappedI80F48toBigNumber(
        rawData.config.liabilityWeightInit
      ),
      liabilityWeightMaint: wrappedI80F48toBigNumber(
        rawData.config.liabilityWeightMaint
      ),
      maxCapacity: nativeToUi(rawData.config.maxCapacity, this.mintDecimals),
      pythOracle: rawData.config.pythOracle,
      interestRateConfig: {
        insuranceFeeFixedApr: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.insuranceFeeFixedApr
        ),
        maxInterestRate: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.maxInterestRate
        ),
        insuranceIrFee: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.insuranceIrFee
        ),
        optimalUtilizationRate: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.optimalUtilizationRate
        ),
        plateauInterestRate: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.optimalUtilizationRate
        ),
        protocolFixedFeeApr: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.protocolFixedFeeApr
        ),
        protocolIrFee: wrappedI80F48toBigNumber(
          rawData.config.interestRateConfig.protocolIrFee
        ),
      },
    };

    this.totalDepositShares = wrappedI80F48toBigNumber(
      rawData.totalDepositShares
    );
    this.totalLiabilityShares = wrappedI80F48toBigNumber(
      rawData.totalLiabilityShares
    );

    this.priceData = priceData;
  }

  get totalDeposits(): BigNumber {
    return this.getAssetQuantity(this.totalDepositShares);
  }

  get totalLiabilities(): BigNumber {
    return this.getLiabilityQuantity(this.totalLiabilityShares);
  }

  public async reloadPriceData(connection: Connection) {
    const pythPriceAccount = await connection.getAccountInfo(
      this.config.pythOracle
    );
    this.priceData = parsePriceData(pythPriceAccount!.data);
  }

  public getAssetQuantity(depositShares: BigNumber): BigNumber {
    return depositShares.times(this.depositShareValue);
  }

  public getLiabilityQuantity(liabilityShares: BigNumber): BigNumber {
    return liabilityShares.times(this.liabilityShareValue);
  }

  public getAssetShares(depositValue: BigNumber): BigNumber {
    return depositValue.div(this.depositShareValue);
  }

  public getLiabilityShares(liabilityValue: BigNumber): BigNumber {
    return liabilityValue.div(this.liabilityShareValue);
  }

  public getAssetUsdValue(
    depositShares: BigNumber,
    marginRequirementType: MarginRequirementType,
    priceBias: PriceBias
  ): BigNumber {
    return this.getUsdValue(
      this.getAssetQuantity(depositShares),
      priceBias,
      this.getAssetWeight(marginRequirementType)
    );
  }

  public getLiabilityUsdValue(
    liabilityShares: BigNumber,
    marginRequirementType: MarginRequirementType,
    priceBias: PriceBias
  ): BigNumber {
    return this.getUsdValue(
      this.getLiabilityQuantity(liabilityShares),
      priceBias,
      this.getLiabilityWeight(marginRequirementType)
    );
  }

  public getUsdValue(
    quantity: BigNumber,
    priceBias: PriceBias,
    weight?: BigNumber
  ): BigNumber {
    const price = this.getPrice(priceBias);
    return quantity
      .times(price)
      .times(weight ?? 1)
      .dividedBy(10 ** this.mintDecimals);
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
      case PriceBias.None:
        return basePriceVal;
    }
  }

  // Return deposit weight based on margin requirement types
  public getAssetWeight(
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
        return new BigNumber(1);
      default:
        throw new Error("Invalid margin requirement type");
    }
  }

  public getQuantityFromUsdValue(
    usdValue: BigNumber,
    priceBias: PriceBias
  ): BigNumber {
    const price = this.getPrice(priceBias);
    return usdValue.div(price);
  }

  public getInterestRates(): {
    lendingRate: BigNumber;
    borrowingRate: BigNumber;
  } {
    const {
      insuranceFeeFixedApr,
      insuranceIrFee,
      protocolFixedFeeApr,
      protocolIrFee,
    } = this.config.interestRateConfig;

    const rateFee = insuranceFeeFixedApr.plus(protocolFixedFeeApr);
    const fixedFee = insuranceIrFee.plus(protocolIrFee);

    const interestRate = this.interestRateCurve();
    const utilizationRate = this.getUtilizationRate();

    const lendingRate = interestRate.times(utilizationRate);
    const borrowingRate = interestRate
      .times(new BigNumber(1).plus(rateFee))
      .plus(fixedFee);

    return { lendingRate, borrowingRate };
  }

  private interestRateCurve(): BigNumber {
    const { optimalUtilizationRate, plateauInterestRate, maxInterestRate } =
      this.config.interestRateConfig;

    const utilizationRate = this.getUtilizationRate();

    if (utilizationRate.lte(optimalUtilizationRate)) {
      return utilizationRate.times(maxInterestRate).div(optimalUtilizationRate);
    } else {
      return utilizationRate
        .minus(optimalUtilizationRate)
        .div(new BigNumber(1).minus(optimalUtilizationRate))
        .times(maxInterestRate.minus(plateauInterestRate))
        .plus(plateauInterestRate);
    }
  }

  private getUtilizationRate(): BigNumber {
    return this.totalLiabilities.div(this.totalDeposits);
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
  None = 1,
  Highest = 2,
}

function priceComponentsToBigNumber(price: BigNumber, exp: number): BigNumber {
  return price.times(new BigNumber(10).pow(exp - USDC_DECIMALS));
}
