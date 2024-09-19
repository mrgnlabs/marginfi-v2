import { BN, Program, workspace } from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import { addBank, groupInitialize } from "./utils/group-instructions";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairUsdc,
  bankrunContext,
  banksClient,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
  printBuffers,
  users,
  verbose,
} from "./rootHooks";
import {
  assertBNEqual,
  assertI80F48Approx,
  assertI80F48Equal,
  assertKeyDefault,
  assertKeysEqual,
} from "./utils/genericTests";
import { ASSET_TAG_DEFAULT, defaultBankConfig } from "./utils/types";
import {
  deriveLiquidityVaultAuthority,
  deriveLiquidityVault,
  deriveInsuranceVaultAuthority,
  deriveInsuranceVault,
  deriveFeeVaultAuthority,
  deriveFeeVault,
} from "./utils/pdas";
import { assert } from "chai";
import { printBufferGroups } from "./utils/tools";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";

describe("Init group and add banks with asset category flags", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  it("(admin) Init group - happy path", async () => {
    let tx = new Transaction();

    tx.add(
      await groupInitialize(program, {
        marginfiGroup: marginfiGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, marginfiGroup);
    await banksClient.processTransaction(tx);

    let group = await program.account.marginfiGroup.fetch(
      marginfiGroup.publicKey
    );
    assertKeysEqual(group.admin, groupAdmin.wallet.publicKey);
    if (verbose) {
      console.log("*init group: " + marginfiGroup.publicKey);
      console.log(" group admin: " + group.admin);
    }
  });

  it("(admin) Add bank (USDC) - is neither SOL nor staked LST", async () => {
    let setConfig = defaultBankConfig(oracles.usdcOracle.publicKey);
    let bankKey = bankKeypairUsdc.publicKey;
    const now = Date.now() / 1000;

    let tx = new Transaction();

    tx.add(
      await addBank(program, {
        marginfiGroup: marginfiGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.usdcMint.publicKey,
        bank: bankKey,
        config: setConfig,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, bankKeypairUsdc);
    await banksClient.processTransaction(tx);

    if (verbose) {
      console.log("*init USDC bank " + bankKey);
    }

    let bankData = (
      await program.provider.connection.getAccountInfo(bankKey)
    ).data.subarray(8);
    if (printBuffers) {
      printBufferGroups(bankData, 16, 896);
    }

    const bank = await program.account.bank.fetch(bankKey);
    assert.equal(bank.config.assetTag, ASSET_TAG_DEFAULT);
  });

  //   it("(admin) Add bank (token A) - happy path", async () => {
  //     let config = defaultBankConfig(oracles.tokenAOracle.publicKey);
  //     let bankKey = bankKeypairA.publicKey;

  //     await groupAdmin.userMarginProgram!.provider.sendAndConfirm!(
  //       new Transaction().add(
  //         await addBank(program, {
  //           marginfiGroup: marginfiGroup.publicKey,
  //           admin: groupAdmin.wallet.publicKey,
  //           feePayer: groupAdmin.wallet.publicKey,
  //           bankMint: ecosystem.tokenAMint.publicKey,
  //           bank: bankKey,
  //           config: config,
  //         })
  //       ),
  //       [bankKeypairA]
  //     );

  //     if (verbose) {
  //       console.log("*init token A bank " + bankKey);
  //     }
  //   });
});
