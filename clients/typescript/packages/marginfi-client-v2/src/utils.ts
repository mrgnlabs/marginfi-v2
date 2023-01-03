import { AnchorProvider } from "@project-serum/anchor";
import {
  ConfirmOptions,
  Connection,
  Keypair,
  PublicKey,
  Signer,
  Transaction,
  TransactionSignature,
} from "@solana/web3.js";
import {
  PDA_BANK_LIQUIDITY_VAULT_AUTH_SEED,
  PDA_BANK_INSURANCE_VAULT_AUTH_SEED,
  PDA_BANK_FEE_VAULT_AUTH_SEED,
} from "./constants";
import { BankVaultType, Environment } from "./types";

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

/**
 * @internal
 */
export function getEnvFromStr(envString: string = "devnet"): Environment {
  switch (envString.toUpperCase()) {
    case "MAINNET":
      return Environment.MAINNET;
    case "MAINNET-BETA":
      return Environment.MAINNET;
    default:
      return Environment.DEVNET;
  }
}

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
 * Compute bank authority PDA for a specific marginfi group
 */
export async function getBankVaultAuthority(
  marginfiGroupPk: PublicKey,
  programId: PublicKey,
  bankVaultType: BankVaultType = BankVaultType.LiquidityVault
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddressSync(
    [getVaultAuthoritySeed(bankVaultType), marginfiGroupPk.toBuffer()],
    programId
  );
}

function getVaultAuthoritySeed(type: BankVaultType): Buffer {
  switch (type) {
    case BankVaultType.LiquidityVault:
      return PDA_BANK_LIQUIDITY_VAULT_AUTH_SEED;
    case BankVaultType.InsuranceVault:
      return PDA_BANK_INSURANCE_VAULT_AUTH_SEED;
    case BankVaultType.FeeVault:
      return PDA_BANK_FEE_VAULT_AUTH_SEED;
    default:
      throw Error("Unkown bank vault type requested");
  }
}
