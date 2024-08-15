import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

export const MAX_ORACLE_KEYS = 5;
export const decodeBank = (buff) => {
    const keySize = 32;
    const u8Size = 1;
    const I80F48Size = 16;
    const u64Size = 8;
    const i64Size = 8;
    const bankConfigSize = 544;

    let b = 0;
    const mint = new PublicKey(buff.subarray(b, b + keySize));
    b += keySize;
    const mintDecimals = Number(buff[b]);
    b += u8Size;
    const group = new PublicKey(buff.subarray(b, b + keySize));
    b += keySize;
    const assetShareValue = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;
    const liabilityShareValue = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;
    const liquidityVault = new PublicKey(buff.subarray(b, b + keySize));
    b += keySize;
    const liquidityVaultBump = Number(buff[b]);
    b += u8Size;
    const liquidityVaultAuthorityBump = Number(buff[b]);
    b += u8Size;
    const insuranceVault = new PublicKey(buff.subarray(b, b + keySize));
    b += keySize;
    const insuranceVaultBump = Number(buff[b]);
    b += u8Size;
    const insuranceVaultAuthorityBump = Number(buff[b]);
    b += u8Size;
    const collectedInsuranceFeesOutstanding = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;
    const feeVault = new PublicKey(buff.subarray(b, b + keySize));
    b += keySize;
    const feeVaultBump = Number(buff[b]);
    b += u8Size;
    const feeVaultAuthorityBump = Number(buff[b]);
    b += u8Size;
    const collectedGroupFeesOutstanding = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;
    const totalLiabilityShares = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;
    const totalAssetShares = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;
    const lastUpdate = new BN(buff.subarray(b, b + i64Size), 'le');
    b += i64Size;
    const config = decodeBankConfig(buff.subarray(b, b + bankConfigSize));
    b += bankConfigSize;
    const flags = new BN(buff.subarray(b, b + u64Size), 'le');
    b += u64Size;
    const emissionsRate = new BN(buff.subarray(b, b + u64Size), 'le');
    b += u64Size;
    const emissionsRemaining = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;
    const emissionsMint = new PublicKey(buff.subarray(b, b + keySize));
    b += keySize;

    // Skipping the padding fields
    b += 1024 + 1024; // padding sizes

    let bank = {
        mint,
        mintDecimals,
        group,
        assetShareValue,
        liabilityShareValue,
        liquidityVault,
        liquidityVaultBump,
        liquidityVaultAuthorityBump,
        insuranceVault,
        insuranceVaultBump,
        insuranceVaultAuthorityBump,
        collectedInsuranceFeesOutstanding,
        feeVault,
        feeVaultBump,
        feeVaultAuthorityBump,
        collectedGroupFeesOutstanding,
        totalLiabilityShares,
        totalAssetShares,
        lastUpdate,
        config,
        flags,
        emissionsRate,
        emissionsMint,
        emissionsRemaining,
    };

    return bank;
};

const decodeWrappedI80F48 = (buff) => {
    // WrappedI80F48 is 16 bytes, using BN for BigInt handling for now
    return new BN(buff, 'le');
};

const decodeBankConfig = (buff) => {
    const I80F48Size = 16;
    const u64Size = 8;
    const u16Size = 2;
    const interestRateConfigSize = 240; // Updated size for InterestRateConfig

    let b = 0;
    const assetWeightInit = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const assetWeightMaint = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const liabilityWeightInit = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const liabilityWeightMaint = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const depositLimit = new BN(buff.subarray(b, b + u64Size), 'le');
    b += u64Size;

    const interestRateConfig = decodeInterestRateConfig(buff.subarray(b, b + interestRateConfigSize));
    b += interestRateConfigSize;

    const operationalState = buff[b];
    b += 1;

    const oracleSetup = buff[b];
    b += 1;

    const oracleKeys = [];
    console.log("now at byte: " + b);
    for (let i = 0; i < MAX_ORACLE_KEYS; i++) {
        oracleKeys.push(new PublicKey(buff.subarray(b, b + 32)));
        b += 32;
    }

    const borrowLimit = new BN(buff.subarray(b, b + u64Size), 'le');
    b += u64Size;

    const riskTier = new BN(buff.subarray(b, b + u64Size), 'le');
    b += u64Size;

    const totalAssetValueInitLimit = new BN(buff.subarray(b, b + u64Size), 'le');
    b += u64Size;

    const oracleMaxAge = Number(buff.slice(b, b + u16Size));
    b += u16Size;

    const _padding = buff.subarray(b, b + 64);
    b += 64;

    return {
        assetWeightInit,
        assetWeightMaint,
        liabilityWeightInit,
        liabilityWeightMaint,
        depositLimit,
        interestRateConfig,
        operationalState,
        oracleSetup,
        oracleKeys,
        borrowLimit,
        riskTier,
        totalAssetValueInitLimit,
        oracleMaxAge,
        _padding,
    };
};

const decodeInterestRateConfig = (buff) => {
    const I80F48Size = 16;

    let b = 0;
    const optimalUtilizationRate = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const plateauInterestRate = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const maxInterestRate = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const insuranceFeeFixedApr = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const insuranceIrFee = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const protocolFixedFeeApr = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const protocolIrFee = decodeWrappedI80F48(buff.subarray(b, b + I80F48Size));
    b += I80F48Size;

    const _padding = buff.subarray(b, b + 128);

    return {
        optimalUtilizationRate,
        plateauInterestRate,
        maxInterestRate,
        insuranceFeeFixedApr,
        insuranceIrFee,
        protocolFixedFeeApr,
        protocolIrFee,
        _padding,
    };
};
