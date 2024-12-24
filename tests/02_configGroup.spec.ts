import {
  AnchorProvider,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { Keypair, Transaction } from "@solana/web3.js";
import { groupConfigure } from "./utils/group-instructions";
import { Marginfi } from "../target/types/marginfi";
import { groupAdmin, marginfiGroup } from "./rootHooks";
import { assertKeysEqual } from "./utils/genericTests";

describe("Config group", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  it("(admin) Config group - no change", async () => {
    await groupAdmin.mrgnProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await groupConfigure(program, {
          newAdmin: null,
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
        })
      )
    );

    let group = await program.account.marginfiGroup.fetch(
      marginfiGroup.publicKey
    );
    assertKeysEqual(group.admin, groupAdmin.wallet.publicKey);
  });

  it("(admin) Config group - set new admin", async () => {
    let newAdmin = Keypair.generate();
    await groupAdmin.mrgnProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await groupConfigure(program, {
          newAdmin: newAdmin.publicKey,
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
        })
      )
    );

    let group = await program.account.marginfiGroup.fetch(
      marginfiGroup.publicKey
    );
    assertKeysEqual(group.admin, newAdmin.publicKey);

    // Restore original
    await groupAdmin.mrgnProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await groupConfigure(program, {
          newAdmin: groupAdmin.wallet.publicKey,
          marginfiGroup: marginfiGroup.publicKey,
          admin: newAdmin.publicKey,
        })
      ),
      [newAdmin]
    );

    group = await program.account.marginfiGroup.fetch(
      marginfiGroup.publicKey
    );
    assertKeysEqual(group.admin, groupAdmin.wallet.publicKey);
  });
});
