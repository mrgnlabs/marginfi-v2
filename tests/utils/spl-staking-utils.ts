import {
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { Connection, PublicKey, TransactionInstruction } from "@solana/web3.js";

export enum SinglePoolAccountType {
  Uninitialized = 0,
  Pool = 1,
}

export type SinglePool = {
  accountType: SinglePoolAccountType;
  voteAccountAddress: PublicKey;
};

const decodeSinglePoolAccountType = (buffer: Buffer, offset: number) => {
  const accountType = buffer.readUInt8(offset);
  if (accountType === 0) {
    return SinglePoolAccountType.Uninitialized;
  } else if (accountType === 1) {
    return SinglePoolAccountType.Pool;
  } else {
    throw new Error("Unknown SinglePoolAccountType");
  }
};

/**
 * Decode an spl single pool from buffer.
 *
 * Get the data buffer with `const data = (await provider.connection.getAccountInfo(poolKey)).data;`
 * and note that there is no discriminator (i.e. pass data directly without additional slicing)
 */
export const decodeSinglePool = (buffer: Buffer) => {
  let offset = 0;

  const accountType = decodeSinglePoolAccountType(buffer, offset);
  offset += 1;

  const voteAccountAddress = new PublicKey(
    buffer.subarray(offset, offset + 32)
  );
  offset += 32;

  return {
    accountType,
    voteAccountAddress,
  };
};

// See `https://www.npmjs.com/package/@solana/spl-single-pool` transactions.ts for the original

export const depositToSinglePoolIxes = async (
  connection: Connection,
  userWallet: PublicKey,
  splMint: PublicKey,
  verbose: boolean = false
) => {
  const ixes: TransactionInstruction[] = [];
  const lstAta = getAssociatedTokenAddressSync(splMint, userWallet);
  try {
    await connection.getAccountInfo(lstAta);
    if (verbose) {
      console.log("Existing LST ata at: " + lstAta);
    }
  } catch (err) {
    if (verbose) {
      console.log("Did not find token account, creating: " + lstAta);
    }
    ixes.push(
      createAssociatedTokenAccountInstruction(
        userWallet,
        lstAta,
        userWallet,
        splMint
      )
    );
  }

  return ixes;
};
