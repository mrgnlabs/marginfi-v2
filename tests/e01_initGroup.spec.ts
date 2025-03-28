import { BN } from "@coral-xyz/anchor";
import { AccountMeta, Transaction } from "@solana/web3.js";
import {
  addBankWithSeed,
  groupConfigure,
  groupInitialize,
} from "./utils/group-instructions";
import {
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  EMODE_SEED,
  emodeAdmin,
  emodeGroup,
  groupAdmin,
  oracles,
  verbose,
} from "./rootHooks";
import { assertKeyDefault, assertKeysEqual } from "./utils/genericTests";
import {
  ASSET_TAG_DEFAULT,
  ASSET_TAG_SOL,
  defaultBankConfig,
  ORACLE_SETUP_PYTH_LEGACY,
  ORACLE_SETUP_PYTH_PUSH,
} from "./utils/types";
import { assert } from "chai";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { deriveBankWithSeed } from "./utils/pdas";

describe("Init e-mode enabled group", () => {
  const seed = new BN(EMODE_SEED);

  it("(admin) Init group - happy path", async () => {
    let tx = new Transaction();

    tx.add(
      await groupInitialize(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: emodeGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, emodeGroup);
    await banksClient.processTransaction(tx);

    let group = await bankrunProgram.account.marginfiGroup.fetch(
      emodeGroup.publicKey
    );
    assertKeysEqual(group.admin, groupAdmin.wallet.publicKey);
    assertKeyDefault(group.emodeAdmin);
    if (verbose) {
      console.log("*init group: " + emodeGroup.publicKey);
      console.log(" group admin: " + group.admin);
    }
  });

  it("(admin) Set the emode admin - happy path", async () => {
    let tx = new Transaction();

    tx.add(
      await groupConfigure(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: emodeGroup.publicKey,
        newEmodeAdmin: emodeAdmin.wallet.publicKey
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, emodeGroup);
    await banksClient.processTransaction(tx);

    let group = await bankrunProgram.account.marginfiGroup.fetch(
      emodeGroup.publicKey
    );
    assertKeysEqual(group.emodeAdmin, emodeAdmin.wallet.publicKey);
  });


  it("(admin) Add bank (USDC)", async () => {
    let setConfig = defaultBankConfig();
    const [bankKey] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      seed
    );
    const oracle = oracles.usdcOracle.publicKey;
    const oracleMeta: AccountMeta = {
      pubkey: oracle,
      isSigner: false,
      isWritable: false,
    };
    const config_ix = await groupAdmin.mrgnProgram.methods
      .lendingPoolConfigureBankOracle(ORACLE_SETUP_PYTH_LEGACY, oracle)
      .accountsPartial({
        group: emodeGroup.publicKey,
        bank: bankKey,
        admin: groupAdmin.wallet.publicKey,
      })
      .remainingAccounts([oracleMeta])
      .instruction();

    let tx = new Transaction();
    tx.add(
      await addBankWithSeed(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: emodeGroup.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.usdcMint.publicKey,
        bank: bankKey,
        config: setConfig,
        seed: seed,
      }),
      config_ix
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    if (verbose) {
      console.log("*init USDC bank " + bankKey);
    }

    const bank = await bankrunProgram.account.bank.fetch(bankKey);
    assert.equal(bank.config.assetTag, ASSET_TAG_DEFAULT);
  });

  it("(admin) Add bank (SOL)", async () => {
    let setConfig = defaultBankConfig();
    setConfig.assetTag = ASSET_TAG_SOL;
    const [bankKey] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.wsolMint.publicKey,
      seed
    );
    const oracle = oracles.wsolOracle.publicKey;
    const oracleMeta: AccountMeta = {
      pubkey: oracle,
      isSigner: false,
      isWritable: false,
    };
    const config_ix = await groupAdmin.mrgnProgram.methods
      .lendingPoolConfigureBankOracle(ORACLE_SETUP_PYTH_LEGACY, oracle)
      .accountsPartial({
        group: emodeGroup.publicKey,
        bank: bankKey,
        admin: groupAdmin.wallet.publicKey,
      })
      .remainingAccounts([oracleMeta])
      .instruction();

    let tx = new Transaction();
    tx.add(
      await addBankWithSeed(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: emodeGroup.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.wsolMint.publicKey,
        bank: bankKey,
        config: setConfig,
        seed: seed,
      }),
      config_ix
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    if (verbose) {
      console.log("*init SOL bank " + bankKey);
    }

    const bank = await bankrunProgram.account.bank.fetch(bankKey);
    assert.equal(bank.config.assetTag, ASSET_TAG_SOL);
  });

  it("(admin) Add bank (LST)", async () => {
    let setConfig = defaultBankConfig();
    const [bankKey] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      seed
    );
    const oracleMeta: AccountMeta = {
      pubkey: oracles.pythPullLst.publicKey, // NOTE: This is the Price V2 update
      isSigner: false,
      isWritable: false,
    };

    let tx = new Transaction();
    tx.add(
      await addBankWithSeed(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: emodeGroup.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.lstAlphaMint.publicKey,
        bank: bankKey,
        config: setConfig,
        seed: seed,
      }),
      await groupAdmin.mrgnProgram.methods
        // Note: This is the feed id
        .lendingPoolConfigureBankOracle(
          ORACLE_SETUP_PYTH_PUSH,
          oracles.pythPullLstOracleFeed.publicKey
        )
        .accountsPartial({
          group: emodeGroup.publicKey,
          bank: bankKey,
          admin: groupAdmin.wallet.publicKey,
        })
        .remainingAccounts([oracleMeta])
        .instruction()
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    if (verbose) {
      console.log("*init WSOL bank " + bankKey);
    }

    const bank = await bankrunProgram.account.bank.fetch(bankKey);
    assert.equal(bank.config.assetTag, ASSET_TAG_DEFAULT);
  });
});
