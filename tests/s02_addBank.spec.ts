import { BN, Program, workspace } from "@coral-xyz/anchor";
import { Keypair, Transaction } from "@solana/web3.js";
import { addBank, groupInitialize } from "./utils/group-instructions";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairSol,
  bankKeypairUsdc,
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
  validators,
  verbose,
} from "./rootHooks";
import {
  assertKeysEqual,
} from "./utils/genericTests";
import {
  ASSET_TAG_DEFAULT,
  ASSET_TAG_SOL,
  ASSET_TAG_STAKED,
  defaultBankConfig,
} from "./utils/types";
import { assert } from "chai";
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

    let group = await bankrunProgram.account.marginfiGroup.fetch(
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

    const bank = await bankrunProgram.account.bank.fetch(bankKey);
    assert.equal(bank.config.assetTag, ASSET_TAG_DEFAULT);
  });

  it("(admin) Add bank (SOL) - is tagged as SOL", async () => {
    let setConfig = defaultBankConfig(oracles.wsolOracle.publicKey);
    setConfig.assetTag = ASSET_TAG_SOL;
    let bankKey = bankKeypairSol.publicKey;

    let tx = new Transaction();
    tx.add(
      await addBank(program, {
        marginfiGroup: marginfiGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.wsolMint.publicKey,
        bank: bankKey,
        config: setConfig,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, bankKeypairSol);
    await banksClient.processTransaction(tx);

    if (verbose) {
      console.log("*init SOL bank " + bankKey);
    }

    const bank = await bankrunProgram.account.bank.fetch(bankKey);
    assert.equal(bank.config.assetTag, ASSET_TAG_SOL);
  });

  it("(admin) Add bank (Staked SOL) - is tagged as Staked", async () => {
    let setConfig = defaultBankConfig(oracles.wsolOracle.publicKey);
    setConfig.assetTag = ASSET_TAG_STAKED;
    // Staked assets aren't designed to be borrowed...
    setConfig.borrowLimit = new BN(0);
    let bankKeypair = Keypair.generate();
    validators[0].bank = bankKeypair.publicKey;

    let tx = new Transaction();
    tx.add(
      await addBank(program, {
        marginfiGroup: marginfiGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: validators[0].splMint,
        bank: validators[0].bank,
        config: setConfig,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, bankKeypair);
    await banksClient.processTransaction(tx);

    if (verbose) {
      console.log("*init LST bank " + validators[0].bank + " (validator 0)");
    }

    const bank = await bankrunProgram.account.bank.fetch(validators[0].bank);
    assert.equal(bank.config.assetTag, ASSET_TAG_STAKED);
  });

  // TODO add an LST-based pool without permission
});
