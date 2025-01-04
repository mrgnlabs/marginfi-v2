// !!!IMPORTANT NOTE This test will fail if run with the other localnet tests because those will use
// Pyth Legacy for simplicity. You must have built with a live()! flag (for example devnet) to use
// this test. TThe easiest way to run this is to simply swap the staging key for the localnet one in
// lib.rs, enable this test only in Anchor.toml, and run:
/*
anchor build -p marginfi -- --no-default-features --features staging
anchor test --skip-build
*/
// This test also isn't currently part of the CI pipeline

import { BN, Program, workspace } from "@coral-xyz/anchor";
import { Keypair, Transaction } from "@solana/web3.js";
import {
  addBankPermissionless,
  groupInitialize,
  initStakedSettings,
} from "./utils/group-instructions";
import { Marginfi } from "../target/types/marginfi";
import {
  bankrunContext,
  bankrunProgram,
  banksClient,
  groupAdmin,
  PYTH_ORACLE_FEED_SAMPLE,
  PYTH_ORACLE_SAMPLE,
  validators,
  verbose,
} from "./rootHooks";
import {
  assertBNEqual,
  assertI80F48Approx,
  assertKeysEqual,
} from "./utils/genericTests";
import {
  ASSET_TAG_DEFAULT,
  ASSET_TAG_STAKED,
  defaultStakedInterestSettings,
} from "./utils/types";
import { assert } from "chai";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { deriveBankWithSeed, deriveStakedSettings } from "./utils/pdas";

describe("Add Bank using Pyth Pull Oracles", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const groupKeypair = Keypair.generate();
  const groupKey = groupKeypair.publicKey;

  it("(admin) Init group - happy path", async () => {
    let tx = new Transaction();
    console.log("group " + groupKey);
    console.log("walet: " + groupAdmin.wallet.publicKey);

    tx.add(
      await groupInitialize(program, {
        marginfiGroup: groupKey,
        admin: groupAdmin.wallet.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, groupKeypair);
    await banksClient.processTransaction(tx);

    let group = await bankrunProgram.account.marginfiGroup.fetch(groupKey);
    assertKeysEqual(group.admin, groupAdmin.wallet.publicKey);
    if (verbose) {
      console.log("*init group: " + groupKey);
      console.log(" group admin: " + group.admin);
    }
  });

  it("(admin) Init staked settings for group - opts in to use staked collateral", async () => {
    const settings = defaultStakedInterestSettings(PYTH_ORACLE_SAMPLE);
    let tx = new Transaction();

    tx.add(
      await initStakedSettings(groupAdmin.mrgnProgram, {
        group: groupKey,
        feePayer: groupAdmin.wallet.publicKey,
        settings: settings,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    const [settingsKey] = deriveStakedSettings(program.programId, groupKey);
    if (verbose) {
      console.log("*init staked settings: " + settingsKey);
    }

    let settingsAcc = await bankrunProgram.account.stakedSettings.fetch(
      settingsKey
    );
    assertKeysEqual(settingsAcc.key, settingsKey);
    assertKeysEqual(settingsAcc.oracle, PYTH_ORACLE_SAMPLE);
    assertI80F48Approx(settingsAcc.assetWeightInit, 0.8);
    assertI80F48Approx(settingsAcc.assetWeightMaint, 0.9);
    assertBNEqual(settingsAcc.depositLimit, 1_000_000_000_000);
    assertBNEqual(settingsAcc.totalAssetValueInitLimit, 150_000_000);
    assert.equal(settingsAcc.oracleMaxAge, 60);
    assert.deepEqual(settingsAcc.riskTier, { collateral: {} });
  });

  it("(admin) Add staked sol bank (pyth pull oracle)", async () => {
    const [bankKey] = deriveBankWithSeed(
      program.programId,
      groupKey,
      validators[0].splMint,
      new BN(0)
    );
    validators[0].bank = bankKey;

    let tx = new Transaction();
    tx.add(
      await addBankPermissionless(bankrunProgram, {
        marginfiGroup: groupKey,
        feePayer: groupAdmin.wallet.publicKey,
        pythOracle: PYTH_ORACLE_FEED_SAMPLE,
        stakePool: validators[0].splPool,
        seed: new BN(0),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    // let logs = result.meta.logMessages;
    // for (let i = 0; i < logs.length; i++) {
    //   console.log(logs[i]);
    // }

    if (verbose) {
      console.log("*init Staked Sol bank " + bankKey);
    }

    const bank = await bankrunProgram.account.bank.fetch(bankKey);
    assert.equal(bank.config.assetTag, ASSET_TAG_STAKED);
    assert.deepEqual(bank.config.oracleSetup, {
      stakedWithPythPush: {},
    });
  });
});
