import type { AnchorProvider } from "@coral-xyz/anchor";
import type { RawAccount } from "@solana/spl-token";
import { AccountLayout } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { assert } from "chai";

/**
 * Shorthand for `assert.equal(a.toString(), b.toString())`
 * @param a
 * @param b
 */
export const assertKeysEqual = (a: PublicKey, b: PublicKey) => {
  assert.equal(a.toString(), b.toString());
};

/**
 * Shorthand for `assert.equal(a.toString(), PublicKey.default.toString())`
 * @param a
 */
export const assertKeyDefault = (a: PublicKey) => {
  assert.equal(a.toString(), PublicKey.default.toString());
};

/**
 * Shorthand for `assert.equal(a.toString(), b.toString())`
 * @param a - a BN
 * @param b - a BN or number
 */
export const assertBNEqual = (a: BN, b: BN | number) => {
  if (typeof b === "number") {
    b = new BN(b);
  }
  assert.equal(a.toString(), b.toString());
};

/**
 * Shorthand for `assert.approximately(a, b, tolerance)` for two BNs. Safe from Integer overflow
 * @param a
 * @param b
 * @param tolerance
 */
export const assertBNApproximately = (
  a: BN,
  b: BN | number,
  tolerance: BN | number
) => {
  const aB = BigInt(a.toString());
  const bB = BigInt(b.toString());
  const toleranceB = BigInt(tolerance.toString());
  assert.ok(aB >= bB - toleranceB);
  assert.ok(aB <= bB + toleranceB);
};

/**
 * Returns the balance of a token account, in whatever currency the account is in.
 * @param provider
 * @param account
 * @returns
 */
export const getTokenBalance = async (
  provider: AnchorProvider,
  account: PublicKey
) => {
  const accountInfo = await provider.connection.getAccountInfo(account);
  if (!accountInfo) {
    console.error("Tried to balance of acc that doesn't exist");
    return 0;
  }
  const data: RawAccount = AccountLayout.decode(accountInfo.data);
  if (data === undefined || data.amount === undefined) {
    return 0;
  }
  const amount: BigInt = data.amount;
  return Number(amount);
};

/**
 * Waits until the given time
 * @param time - in seconds (e.g. Date.now()/1000)
 * @param silenceWarning - (optional) set to true to silence the warning if the time is in the past
 */
export const waitUntil = async (
  time: number,
  silenceWarning: boolean = false
) => {
  const now = Date.now() / 1000;
  if (time > now + 500) {
    console.error("Tried to wait a very long time, aborted");
    return;
  }
  if (now > time) {
    if (!silenceWarning) {
      console.error(
        "Tried to wait for a time that's in the past. You probably need to adjust test timings."
      );
      console.error("now: " + now + " and tried waiting until: " + time);
    }
    return new Promise((r) => setTimeout(r, 1)); //waits 1 ms
  }
  const toWait = Math.ceil(time - now) * 1000;
  await new Promise((r) => setTimeout(r, toWait));
};
