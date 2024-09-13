import { PublicKey } from "@solana/web3.js";

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
