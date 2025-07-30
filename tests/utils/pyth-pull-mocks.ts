import { Keypair, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { createMockAccount, storeMockAccount } from "./mocks";
import { Mocks } from "../../target/types/mocks";
import { Program, Wallet, workspace } from "@coral-xyz/anchor";
import { printBuffers } from "../rootHooks";

type VerificationLevel =
  | { kind: "Partial"; num_signatures: number }
  | { kind: "Full" };

interface PriceFeedMessage {
  feed_id: PublicKey; // 32 bytes
  price: BN; // i64
  conf: BN; // u64
  exponent: number; // i32
  publish_time: BN; // i64 (timestamp in seconds)
  prev_publish_time: BN; // i64
  ema_price: BN; // i64
  ema_conf: BN; // u64
}

interface PriceUpdateV2 {
  write_authority: PublicKey; // 32 bytes
  verification_level: VerificationLevel; // 2 bytes (1 byte tag + 1 byte value/padding)
  price_message: PriceFeedMessage; // 84 bytes
  posted_slot: BN; // u64 (8 bytes)
}

/**
 * Decodes a base64 string (Borsh-serialized PriceUpdateV2) into the PriceUpdateV2 structure.
 *
 * Note: The first 8 bytes are assumed to be the Anchor account discriminator.
 */
export function decodePriceUpdateV2(base64Data: string): PriceUpdateV2 {
  const buffer = Buffer.from(base64Data, "base64");
  let offset = 0;

  // Skip the 8-byte discriminator. (34 241 35 99 157 126 244 205)

  //   const discrim = buffer.subarray(offset, offset + 8);
  //   for (let i = 0; i < discrim.length; i++) {
  //     console.log(i + " " + discrim[i]);
  //   }
  offset += 8;

  // 1. write_authority (32 bytes)
  const write_authority = new PublicKey(buffer.subarray(offset, offset + 32));
  offset += 32;

  // 2. verification_level (2 bytes)
  const verTag = buffer.readUInt8(offset);
  offset += 1;
  let verification_level: VerificationLevel;
  if (verTag === 0) {
    // "Partial" variant: the next byte is the num_signatures.
    const num_signatures = buffer.readUInt8(offset);
    offset += 1;
    verification_level = { kind: "Partial", num_signatures };
  } else if (verTag === 1) {
    // "Full" variant: the next byte is NOT SKIPPED, it is just one byte shorter!
    // offset += 1;
    verification_level = { kind: "Full" };
  } else {
    throw new Error(`Unknown verification level tag: ${verTag}`);
  }

  // 3. PriceFeedMessage
  // - feed_id: 32 bytes
  const feed_id = new PublicKey(buffer.subarray(offset, offset + 32));
  offset += 32;
  // - price: i64 (8 bytes, little-endian)
  const priceBN = new BN(buffer.subarray(offset, offset + 8), "le");
  offset += 8;
  // - conf: u64 (8 bytes)
  const conf = new BN(buffer.subarray(offset, offset + 8), "le");
  offset += 8;
  // - exponent: i32 (4 bytes)
  const exponent = buffer.readInt32LE(offset);
  offset += 4;
  // - publish_time: i64 (8 bytes)
  const publishTimeBN = new BN(buffer.subarray(offset, offset + 8), "le");
  offset += 8;
  // - prev_publish_time: i64 (8 bytes)
  const prevPublishTimeBN = new BN(buffer.subarray(offset, offset + 8), "le");
  offset += 8;
  // - ema_price: i64 (8 bytes)
  const emaPriceBN = new BN(buffer.subarray(offset, offset + 8), "le");
  offset += 8;
  // - ema_conf: u64 (8 bytes)
  const ema_conf = new BN(buffer.subarray(offset, offset + 8), "le");
  offset += 8;

  const price_message: PriceFeedMessage = {
    feed_id,
    price: priceBN,
    conf,
    exponent,
    publish_time: publishTimeBN,
    prev_publish_time: prevPublishTimeBN,
    ema_price: emaPriceBN,
    ema_conf,
  };

  // 4. posted_slot: u64 (8 bytes)
  const posted_slot = new BN(buffer.subarray(offset, offset + 8), "le");
  offset += 8;

  return {
    write_authority,
    verification_level,
    price_message,
    posted_slot,
  };
}

/**
 * Constructs a PriceUpdateV2 buffer (with discriminator) and writes it to the mock account. Pass `existingAccount` to update an existing feed.
 *
 * Layout:
 *   - 8 bytes: Discriminator (34, 241, 35, 99, 157, 126, 244, 205)
 *   - 32 bytes: write_authority (PublicKey)
 *   - 2 bytes: verification_level (Full => [1, 0])
 *   - 32 bytes: feed_id (PublicKey)
 *   - 8 bytes: price (i64)
 *   - 8 bytes: conf (u64)
 *   - 4 bytes: exponent (i32)
 *   - 8 bytes: publish_time (i64)
 *   - 8 bytes: prev_publish_time (i64)
 *   - 8 bytes: ema_price (i64)
 *   - 8 bytes: ema_conf (u64)
 *   - 8 bytes: posted_slot (u64)
 *
 * Total: 134 bytes.
 */
export async function initOrUpdatePriceUpdateV2(
  wallet: Wallet,
  feed_id: PublicKey,
  price: BN,
  conf: BN,
  ema_price: BN,
  ema_conf: BN,
  slot: BN,
  exponent: number,
  // User after setup to update existing account
  existingAccount?: Keypair,
  // Use to give a deterministic keypair during initial setup instead of a random one
  oracleKeypair?: Keypair
) {
  const space = 134;
  // Compute publish times.
  const now = Math.floor(Date.now() / 1000);
  const publish_time = new BN(now);
  const prev_publish_time = new BN(now - 1);
  // Allocate a 134-byte buffer.
  const buf = Buffer.alloc(space);
  let offset = 0;
  // Write the 8-byte discriminator.
  const discriminator = Buffer.from([34, 241, 35, 99, 157, 126, 244, 205]);
  discriminator.copy(buf, offset);
  offset += 8;
  // Write the write_authority (32 bytes).
  // Don't care about this key, use any value.
  const writeAuthority = PublicKey.unique().toBuffer();
  writeAuthority.copy(buf, offset);
  offset += 32;
  // Write verification_level (2 bytes) - "Full": tag 1 and dummy 0.
  buf.writeUInt8(1, offset); // tag for Full
  offset += 1;

  // PriceFeedMessage:
  // feed_id (32 bytes)
  feed_id.toBuffer().copy(buf, offset);
  offset += 32;
  // price (i64, 8 bytes)
  price.toArrayLike(Buffer, "le", 8).copy(buf, offset);
  offset += 8;
  // conf (u64, 8 bytes)
  conf.toArrayLike(Buffer, "le", 8).copy(buf, offset);
  offset += 8;
  // exponent (i32, 4 bytes)
  buf.writeInt32LE(exponent, offset);
  offset += 4;
  // publish_time (i64, 8 bytes)
  publish_time.toArrayLike(Buffer, "le", 8).copy(buf, offset);
  offset += 8;
  // prev_publish_time (i64, 8 bytes)
  prev_publish_time.toArrayLike(Buffer, "le", 8).copy(buf, offset);
  offset += 8;
  // ema_price (i64, 8 bytes)
  ema_price.toArrayLike(Buffer, "le", 8).copy(buf, offset);
  offset += 8;
  // ema_conf (u64, 8 bytes)
  ema_conf.toArrayLike(Buffer, "le", 8).copy(buf, offset);
  offset += 8;
  // posted_slot (u64, 8 bytes)
  slot.toArrayLike(Buffer, "le", 8).copy(buf, offset);
  offset += 8;

  if (printBuffers) {
    console.log("PriceUpdateV2 Buffer (base64):", buf.toString("base64"));
  }

  // Write the buffer to the mock account
  const mockProgram: Program<Mocks> = workspace.Mocks;
  if (existingAccount) {
    await storeMockAccount(mockProgram, wallet, existingAccount, 0, buf);
    return existingAccount;
  } else {
    let account = await createMockAccount(
      mockProgram,
      space,
      wallet,
      oracleKeypair
    );
    await storeMockAccount(mockProgram, wallet, account, 0, buf);
    return account;
  }
}

/**
 * Price updates expect a `valid` feed, but don't actually read anything from it. If suffices to
 * create an account of the correct size with "PYTH" as the owner. Pass this key as the `feed_id` to
 * create price updates for an asset. This size of this account also doesn't matter, nor does the
 * discrminator, etc. It can literally be any account owned by PYTH.
 * @param wallet
 * @returns
 */
export async function initBlankOracleFeed(wallet: Wallet, keypair?: Keypair) {
  const space = 300;
  const buf = Buffer.alloc(space);

  // Write the buffer to the mock account
  const mockProgram: Program<Mocks> = workspace.Mocks;
  let account = await createMockAccount(mockProgram, space, wallet, keypair);
  await storeMockAccount(mockProgram, wallet, account, 0, buf);

  return account;
}

// Sample usage (run with `ts-node pyth-pull-mocks.ts`)

// const base64Data = "IvEjY51+9M0AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQETiBzPres/ehv0qThWXLPfQ9hD11F3I/i9W1bPF+o7/QAW0L4oAAAAAMOd0AAAAAD3////vBLSZwAAAAC7EtJnAAAAAAAW0L4oAAAAAMOd0AAAAAAAAAAAAAAAAAA=";
// //const base64Data =
// //  "IvEjY51+9M1gMUcENA3t3zcf1CRyFI8kjp0abRpesqw6zYt/1dayQwHvDYtv2izrpB2hXUCV0do5Kg0vjtDGx7wPTPrIwoC1bW2njwUFAAAARTozAQAAAAD4////1wN5ZwAAAADXA3lnAAAAAIRKaggFAAAAk1kTAQAAAAA8qpUSAAAAAAA=";
// const priceUpdate = decodePriceUpdateV2(base64Data);

// // For debugging, output fields in a readable format.
// console.log("Write Authority (hex):", priceUpdate.write_authority.toString());
// console.log("Verification Level:", priceUpdate.verification_level);
// console.log("Price Feed Message:", {
//   feed_id: priceUpdate.price_message.feed_id.toString(),
//   price: priceUpdate.price_message.price.toString(),
//   conf: priceUpdate.price_message.conf.toString(),
//   exponent: priceUpdate.price_message.exponent,
//   publish_time: priceUpdate.price_message.publish_time.toString(),
//   prev_publish_time: priceUpdate.price_message.prev_publish_time.toString(),
//   ema_price: priceUpdate.price_message.ema_price.toString(),
//   ema_conf: priceUpdate.price_message.ema_conf.toString(),
// });
// console.log("Posted Slot:", priceUpdate.posted_slot.toString());
