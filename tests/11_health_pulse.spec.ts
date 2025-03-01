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
import { liquidateIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { updatePriceAccount } from "./utils/pyth_mocks";
import { bigNumberToWrappedI80F48, wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { configureBank } from "./utils/instructions";
import { defaultBankConfigOptRaw } from "./utils/types";

describe("Health pulse", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;


  it("oracle data refreshes", async () => {
    // TODO
  });


});
