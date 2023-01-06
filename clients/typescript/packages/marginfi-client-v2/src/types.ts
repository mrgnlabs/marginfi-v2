import { AnchorProvider, BN, Program } from "@project-serum/anchor";
import { SignerWalletAdapter } from "@solana/wallet-adapter-base";
import {
  ConfirmOptions,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import BigNumber from "bignumber.js";
import { Marginfi } from "./idl/marginfi-types";

export type MarginfiProgram = Omit<Program<Marginfi>, "provider"> & {
  provider: AnchorProvider;
};
export type MarginfiReadonlyProgram = Program<Marginfi>;

export type UiAmount = BigNumber | number | string;

export type Wallet = Pick<
  SignerWalletAdapter,
  "signAllTransactions" | "signTransaction"
> & {
  publicKey: PublicKey;
};

export interface TransactionOptions extends ConfirmOptions {
  dryRun?: boolean;
}

/**
 * Supported config environments.
 */
export type Environment = "devnet1";

export interface InstructionsWrapper {
  instructions: TransactionInstruction[];
  keys: Keypair[];
}

/**
 * Marginfi bank vault type
 */
export enum BankVaultType {
  LiquidityVault,
  InsuranceVault,
  FeeVault,
}

export interface MarginfiAccountData {
  group: PublicKey;
  authority: PublicKey;
}

export interface MarginfiConfig {
  environment: Environment;
  cluster: string;
  programId: PublicKey;
  groupPk: PublicKey;
  banks: BankAddress[];
}

export interface BankAddress {
  label: string;
  address: PublicKey;
}

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

export interface InstructionsWrapper {
  instructions: TransactionInstruction[];
  keys: Keypair[];
}

// --- On-chain account structs

export enum AccountType {
  MarginfiGroup = "marginfiGroup",
  MarginfiAccount = "marginfiAccount",
}

export interface WrappedI8048F {
  value: BN;
}

export interface MarginfiGroupData {
  admin: PublicKey;
  reservedSpace: BN[];
}

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
