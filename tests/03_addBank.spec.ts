import { Program, workspace } from "@coral-xyz/anchor";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { addBank, groupConfigure } from "./utils/instructions";
import { Marginfi } from "../target/types/marginfi";
import { ecosystem, groupAdmin, marginfiGroup, oracles } from "./rootHooks";
import { assertKeysEqual } from "./utils/genericTests";
import { defaultBankConfig } from "./utils/types";
import { decodeBank } from "./utils/parsers";

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

    console.log("actual: " + oracles.usdcOracle.publicKey);
    console.log("buff " + oracles.usdcOracle.publicKey.toBytes());

    let bankAcc = await program.provider.connection.getAccountInfo(bankKey.publicKey);
    let bankData = bankAcc.data.subarray(8);
    console.log("bytes: " + bankData.length);

    for (let i = 0; i < bankData.length; i++) {
      console.log(i + " " + bankData[i]);
    }

    let bankLoaded = decodeBank(bankData);
    for (let i = 0; i < bankLoaded.config.oracleKeys.length; i++) {
      console.log("oracle " + i + " " + bankLoaded.config.oracleKeys[i]);
    }

    let bank = await program.account.bank.fetch(bankKey.publicKey);
    for (let i = 0; i < bank.config.oracleKeys.length; i++) {
      console.log("oracle " + i + " " + bank.config.oracleKeys[i]);
    }
    assertKeysEqual(bank.config.oracleKeys[0], oracles.usdcOracle.publicKey);
  });
});
