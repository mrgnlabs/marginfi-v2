import {
  Program,
  workspace,
} from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { groupInitialize } from "./utils/group-instructions";
import { Marginfi } from "../target/types/marginfi";
import { groupAdmin, marginfiGroup, verbose } from "./rootHooks";
import { assertKeysEqual } from "./utils/genericTests";

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

    await groupAdmin.userMarginProgram.provider.sendAndConfirm(tx, [
      marginfiGroup,
    ]);

    let group = await program.account.marginfiGroup.fetch(
      marginfiGroup.publicKey
    );
    assertKeysEqual(group.admin, groupAdmin.wallet.publicKey);
    if(verbose){
      console.log("*init group: " + marginfiGroup.publicKey);
      console.log(" group admin: " + group.admin);
    }
  });
});
