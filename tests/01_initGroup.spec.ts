import { Program, workspace } from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { groupInitialize } from "./utils/instructions";
import { Marginfi } from "../target/types/marginfi";
import {
  globalFeeWallet,
  groupAdmin,
  marginfiGroup,
  PROGRAM_FEE_FIXED,
  PROGRAM_FEE_RATE,
} from "./rootHooks";
import { assertI80F48Approx, assertKeysEqual } from "./utils/genericTests";

describe("Init group", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  it("(admin) Init group - happy path", async () => {
    let tx = new Transaction();

    tx.add(
      await groupInitialize(program, {
        marginfiGroup: marginfiGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
      })
    );

    await groupAdmin.mrgnProgram.provider.sendAndConfirm(tx, [
      marginfiGroup,
    ]);

    let group = await program.account.marginfiGroup.fetch(
      marginfiGroup.publicKey
    );
    assertKeysEqual(group.admin, groupAdmin.wallet.publicKey);

    const feeCache = group.feeStateCache;
    const tolerance = 0.00001;
    assertI80F48Approx(feeCache.programFeeFixed, PROGRAM_FEE_FIXED, tolerance);
    assertI80F48Approx(feeCache.programFeeRate, PROGRAM_FEE_RATE, tolerance);
    assertKeysEqual(feeCache.globalFeeWallet, globalFeeWallet);
  });
});
