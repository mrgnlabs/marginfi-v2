import { Program, workspace } from "@coral-xyz/anchor";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { addBank, groupConfigure } from "./utils/instructions";
import { Marginfi } from "../target/types/marginfi";
import { ecosystem, groupAdmin, marginfiGroup, oracles } from "./rootHooks";
import { assertKeysEqual } from "./utils/genericTests";
import { defaultBankConfig } from "./utils/types";

describe("Lending pool add bank (add bank to group)", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  it("(admin) Add bank - happy path", async () => {
    let config = defaultBankConfig(oracles.usdcOracle.publicKey);
    let bankKey = Keypair.generate();

    await groupAdmin.userMarginProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await addBank(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: ecosystem.usdcMint.publicKey,
          bank: bankKey.publicKey,
          config: config,
        })
      ),
      [bankKey]
    );

    let bank = await program.account.bank.fetch(bankKey.publicKey);
    assertKeysEqual(bank.config.oracleKeys[0], oracles.usdcOracle.publicKey);
    // TODO assert the rest
  });
});
