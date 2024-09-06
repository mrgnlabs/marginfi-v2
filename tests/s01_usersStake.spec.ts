import { Program, workspace } from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { groupInitialize } from "./utils/instructions";
import { Marginfi } from "../target/types/marginfi";
import { groupAdmin, marginfiGroup, users } from "./rootHooks";
import { assertKeysEqual } from "./utils/genericTests";
import { StakingCollatizer } from "../target/types/staking_collatizer";

describe("User stakes some native and creates an account", () => {
  const program = workspace.StakingCollatizer as Program<StakingCollatizer>;

  it("(admin) Init user account - happy path", async () => {
    let tx = new Transaction();

    tx.add(
      await program.methods
        .initUser()
        .accounts({
          payer: users[0].wallet.publicKey,
        })
        .instruction()
    );

    await users[0].userCollatizerProgram.provider.sendAndConfirm(tx);
  });
});
