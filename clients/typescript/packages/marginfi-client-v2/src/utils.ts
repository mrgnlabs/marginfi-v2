import { Address, AnchorProvider, BN } from "@project-serum/anchor";
import {
  ConfirmOptions,
  Connection,
  Keypair,
  PublicKey,
  Signer,
  Transaction,
  TransactionSignature,
} from "@solana/web3.js";
import BigNumber from "bignumber.js";
import {
  PDA_BANK_LIQUIDITY_VAULT_AUTH_SEED,
  PDA_BANK_INSURANCE_VAULT_AUTH_SEED,
  PDA_BANK_FEE_VAULT_AUTH_SEED,
  PDA_BANK_LIQUIDITY_VAULT_SEED,
  PDA_BANK_INSURANCE_VAULT_SEED,
  PDA_BANK_FEE_VAULT_SEED,
} from "./constants";
import { BankVaultType, UiAmount } from "./types";
import { Decimal } from "decimal.js";

/**
 * Load Keypair from the provided file.
 */
export function loadKeypair(keypairPath: string): Keypair {
  const path = require("path");
  if (!keypairPath || keypairPath == "") {
    throw new Error("Keypair is required!");
  }
  if (keypairPath[0] === "~") {
    keypairPath = path.join(require("os").homedir(), keypairPath.slice(1));
  }
  const keyPath = path.normalize(keypairPath);
  const loaded = Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(require("fs").readFileSync(keyPath).toString()))
  );
  return loaded;
}

// /**
//  * @internal
//  */
// export function getEnvFromStr(envString: string = "devnet"): Environment {
//   switch (envString.toUpperCase()) {
//     case "MAINNET":
//       return Environment.MAINNET;
//     case "MAINNET-BETA":
//       return Environment.MAINNET;
//     default:
//       return Environment.DEVNET;
//   }
// }

/**
 * Transaction processing and error-handling helper.
 */
export async function processTransaction(
  provider: AnchorProvider,
  tx: Transaction,
  signers?: Array<Signer>,
  opts?: ConfirmOptions
): Promise<TransactionSignature> {
  const connection = new Connection(
    provider.connection.rpcEndpoint,
    provider.opts
  );
  const {
    context: { slot: minContextSlot },
    value: { blockhash, lastValidBlockHeight },
  } = await connection.getLatestBlockhashAndContext();

  tx.recentBlockhash = blockhash;
  tx.feePayer = provider.wallet.publicKey;
  tx = await provider.wallet.signTransaction(tx);

  if (signers === undefined) {
    signers = [];
  }
  signers
    .filter((s) => s !== undefined)
    .forEach((kp) => {
      tx.partialSign(kp);
    });

  try {
    const signature = await connection.sendRawTransaction(
      tx.serialize(),
      opts || {
        skipPreflight: false,
        preflightCommitment: provider.connection.commitment,
        commitment: provider.connection.commitment,
        minContextSlot,
      }
    );
    await connection.confirmTransaction({
      blockhash,
      lastValidBlockHeight,
      signature,
    });
    return signature;
  } catch (e: any) {
    console.log(e);
    throw e;
  }
}

/**
 * @internal
 */
export function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function wrappedI80F48toBigNumber(
  { value }: { value: BN },
  scaleDecimal: number = 0
): BigNumber {
  let numbers = new Decimal(
    `${value.isNeg() ? "-" : ""}0b${value.abs().toString(2)}p-48`
  ).dividedBy(10 ** scaleDecimal);
  return new BigNumber(numbers.toString());
}

/**
 * Converts a ui representation of a token amount into its native value as `BN`, given the specified mint decimal amount (default to 6 for USDC).
 */
export function toNumber(amount: UiAmount): number {
  let amt: number;
  if (typeof amount === "number") {
    amt = amount;
  } else if (typeof amount === "string") {
    amt = Number(amount);
  } else {
    amt = amount.toNumber();
  }
  return amt;
}

/**
 * Converts a ui representation of a token amount into its native value as `BN`, given the specified mint decimal amount (default to 6 for USDC).
 */
export function toBigNumber(amount: UiAmount | BN): BigNumber {
  let amt: BigNumber;
  if (amount instanceof BigNumber) {
    amt = amount;
  } else {
    amt = new BigNumber(amount.toString());
  }
  return amt;
}

/**
 * Converts a UI representation of a token amount into its native value as `BN`, given the specified mint decimal amount (default to 6 for USDC).
 */
export function uiToNative(amount: UiAmount, decimals: number): BN {
  let amt = toBigNumber(amount);
  return new BN(amt.times(10 ** decimals).toFixed(0, BigNumber.ROUND_FLOOR));
}

/**
 * Converts a native representation of a token amount into its UI value as `number`, given the specified mint decimal amount (default to 6 for USDC).
 */
export function nativeToUi(amount: UiAmount | BN, decimals: number): number {
  let amt = toBigNumber(amount);
  return amt.div(10 ** decimals).toNumber();
}

function getBankVaultSeeds(type: BankVaultType): Buffer {
  switch (type) {
    case BankVaultType.LiquidityVault:
      return PDA_BANK_LIQUIDITY_VAULT_SEED;
    case BankVaultType.InsuranceVault:
      return PDA_BANK_INSURANCE_VAULT_SEED;
    case BankVaultType.FeeVault:
      return PDA_BANK_FEE_VAULT_SEED;
    default:
      throw Error(`Unknown vault type ${type}`);
  }
}

function getBankVaultAuthoritySeeds(type: BankVaultType): Buffer {
  switch (type) {
    case BankVaultType.LiquidityVault:
      return PDA_BANK_LIQUIDITY_VAULT_AUTH_SEED;
    case BankVaultType.InsuranceVault:
      return PDA_BANK_INSURANCE_VAULT_AUTH_SEED;
    case BankVaultType.FeeVault:
      return PDA_BANK_FEE_VAULT_AUTH_SEED;
    default:
      throw Error(`Unknown vault type ${type}`);
  }
}

/**
 * Compute authority PDA for a specific marginfi group bank vault
 */
export function getBankVaultAuthority(
  bankVaultType: BankVaultType,
  bankPk: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [getBankVaultAuthoritySeeds(bankVaultType), bankPk.toBuffer()],
    programId
  );
}

// shorten the checksummed version of the input address to have 4 characters at start and end
export function shortenAddress(pubkey: Address, chars = 4): string {
  const pubkeyStr = pubkey.toString();
  return `${pubkeyStr.slice(0, chars)}...${pubkeyStr.slice(-chars)}`;
}
