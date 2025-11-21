import { Program } from "@coral-xyz/anchor";
import { ExponentCore } from "./fixtures/exponent_core";
import { exponentBankrunProgram, users } from "./rootHooks";
import { exponentTestState } from "./ex01_exponentInit.spec";
import { exponentUserState } from "./ex02_user.spec";
import { depositYtIx } from "./utils/exponent-instructions";
import { assert } from "chai";

const program = exponentBankrunProgram as Program<ExponentCore>;
describe("exponent::deposit", () => {
  it("builds a deposit instruction for user YT", async () => {
    const user = users[0];
    const userYtAta = exponentUserState.userYtAta;
    const escrowYt = exponentTestState.escrowYt;

    const ix = await depositYtIx(program, {
      depositor: user.wallet.publicKey,
      vault: exponentTestState.vault,
      userYieldPosition: exponentUserState.userYieldPosition,
      ytSrc: userYtAta,
      escrowYt,
      syProgram: program.programId,
      addressLookupTable: exponentTestState.lookupTable,
      yieldPosition: exponentTestState.vaultYieldPosition,
      amount: BigInt(1_000),
    });

    assert.equal(ix.programId.toBase58(), program.programId.toBase58());
    const depositorMeta = ix.keys.find((k) => k.pubkey.equals(user.wallet.publicKey));
    assert.isTrue(depositorMeta?.isSigner ?? false, "depositor must sign deposit instruction");
  });
});
