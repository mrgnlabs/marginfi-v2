import {
  AnchorProvider,
  getProvider,
  Program,
  setProvider,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { assert } from "chai";
import { Keypair, SystemProgram, Transaction } from "@solana/web3.js";
import { groupInitialize } from "./utils/instructions";
import { Marginfi } from "../target/types/marginfi";
import { groupAdmin } from "./rootHooks";
import { assertKeysEqual } from "./utils/genericTests";

describe("marginfi e2e test", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  const marginfiGroup = Keypair.generate();

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
  });
});
