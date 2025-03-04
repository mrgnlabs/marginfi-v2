import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairSol,
  bankKeypairUsdc,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
  users,
  verbose,
} from "./rootHooks";
import {
  assertBNApproximately,
  assertI80F48Approx,
  assertI80F48Equal,
  assertKeysEqual,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import { healthPulse, liquidateIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { updatePriceAccount } from "./utils/pyth_mocks";
import {
  bigNumberToWrappedI80F48,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import { configureBank } from "./utils/instructions";
import { defaultBankConfigOptRaw } from "./utils/types";

describe("Health pulse", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  it("(user 1) health pulse - happy path", async () => {
    const user = users[1];
    const acc = user.accounts.get(USER_ACCOUNT);
    await user.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await healthPulse(user.mrgnProgram, {
          marginfiAccount: acc,
          remaining: [
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
          ],
        })
      )
    );

    const accAfter = await program.account.marginfiAccount.fetch(acc);
    const cacheAfter = accAfter.healthCache;
    console.log(
      "assets " + wrappedI80F48toBigNumber(cacheAfter.assetValue).toString()
    );
  });
});
