import { Commitment, ConfirmOptions, SendOptions } from "@solana/web3.js";
import BigNumber from "bignumber.js";

export const PDA_BANK_LIQUIDITY_VAULT_AUTH_SEED = Buffer.from(
  "liquidity_vault_auth"
);
export const PDA_BANK_INSURANCE_VAULT_AUTH_SEED = Buffer.from(
  "insurance_vault_auth"
);
export const PDA_BANK_FEE_VAULT_AUTH_SEED = Buffer.from("fee_vault_auth");

export const PDA_BANK_LIQUIDITY_VAULT_SEED = Buffer.from("liquidity_vault");
export const PDA_BANK_INSURANCE_VAULT_SEED = Buffer.from("insurance_vault");
export const PDA_BANK_FEE_VAULT_SEED = Buffer.from("fee_vault");

export const DEFAULT_COMMITMENT: Commitment = "processed";
export const DEFAULT_SEND_OPTS: SendOptions = {
  skipPreflight: false,
  preflightCommitment: DEFAULT_COMMITMENT,
};

export const DEFAULT_CONFIRM_OPTS: ConfirmOptions = {
  commitment: DEFAULT_COMMITMENT,
  ...DEFAULT_SEND_OPTS,
};

export const PYTH_PRICE_CONF_INTERVALS = new BigNumber(4.24);
export const USDC_DECIMALS = 6;