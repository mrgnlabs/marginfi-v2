import { Program, workspace } from "@coral-xyz/anchor";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { addBank, groupConfigure } from "./utils/instructions";
import { Marginfi } from "../target/types/marginfi";
import { ecosystem, groupAdmin, marginfiGroup } from "./rootHooks";
import { assertKeysEqual } from "./utils/genericTests";
import { defaultBankConfig } from "./utils/types";

describe("Lending pool add bank (add bank to group)", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  it("(admin) Add bank - happy path", async () => {
    let config = defaultBankConfig(PublicKey.unique());
    let bank = Keypair.generate();

    await groupAdmin.userMarginProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await addBank(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: ecosystem.usdcMint.publicKey,
          bank: bank.publicKey,
          config: config,
        })
      ),
      [bank]
    );

    let group = await program.account.marginfiGroup.fetch(
      marginfiGroup.publicKey
    );
    assertKeysEqual(group.admin, groupAdmin.wallet.publicKey);
  });
});
