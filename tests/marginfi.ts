import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Marginfi } from "../clients/typescript/packages/marginfi-client-v2/src/idl/marginfi";

describe("marginfi", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Marginfi as Program<Marginfi>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
