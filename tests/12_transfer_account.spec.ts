import { Keypair, Transaction } from "@solana/web3.js";
import { Program, workspace } from "@coral-xyz/anchor";
import { Marginfi } from "../target/types/marginfi";
import { marginfiGroup, users, globalFeeWallet } from "./rootHooks";
import { accountInit, transferAccountAuthorityIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { assert } from "chai";

describe("Transfer account authority", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  it("(user 0) migrate account to new", async () => {
    const oldAcc = Keypair.generate();
    users[0].accounts.set(USER_ACCOUNT, oldAcc.publicKey);
    let tx = new Transaction();
    tx.add(
      await accountInit(program, {
        marginfiGroup: marginfiGroup.publicKey,
        marginfiAccount: oldAcc.publicKey,
        authority: users[0].wallet.publicKey,
        feePayer: users[0].wallet.publicKey,
      })
    );
    await program.provider.sendAndConfirm(tx, [oldAcc]);

    const newAcc = Keypair.generate();
    const newAuthority = Keypair.generate();

    let tx2 = new Transaction();
    tx2.add(
      transferAccountAuthorityIx(program, {
        oldAccount: oldAcc.publicKey,
        newAccount: newAcc.publicKey,
        newAuthority: newAuthority.publicKey,
        marginfiGroup: marginfiGroup.publicKey,
        feePayer: users[0].wallet.publicKey,
        globalFeeWallet: globalFeeWallet,
      })
    );
    await program.provider.sendAndConfirm(tx2, [oldAcc, newAcc]);

    const account = await program.account.marginfiAccount.fetch(newAcc.publicKey);
    assert(account.authority.equals(newAuthority.publicKey));
    assert(account.migratedFrom.equals(oldAcc.publicKey));
  });
});
