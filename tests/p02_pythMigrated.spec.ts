import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { Transaction, PublicKey, Keypair } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  banksClient,
  bankrunContext,
  bankrunProgram,
  groupAdmin,
  marginfiGroup,
  users,
  ecosystem,
  oracles,
  verbose,
} from "./rootHooks";
import { deriveBankWithSeed } from "./utils/pdas";
import {
  accountInit,
  borrowIx,
  composeRemainingAccounts,
  depositIx,
  migratePythArgs,
} from "./utils/user-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { USER_ACCOUNT } from "./utils/mocks";
import { assert } from "chai";
import { addBankWithSeed, groupInitialize } from "./utils/group-instructions";
import {
  defaultBankConfig,
  ORACLE_SETUP_PYTH_PUSH,
  PYTH_PULL_MIGRATED,
} from "./utils/types";
import { dumpBankrunLogs } from "./utils/tools";
import { createMintToInstruction } from "@solana/spl-token";
import { assertBankrunTxFailed, assertKeysEqual } from "./utils/genericTests";
import {
  decodePriceUpdateV2,
  initOrUpdatePriceUpdateV2,
} from "./utils/pyth-pull-mocks";

const seed: number = 789;
const groupBuff = Buffer.from("MARGINFI_GROUP_SEED_1234000000p2");
let throwawayGroup: Keypair;
const USER_ACCOUNT_THROWAWAY = "throwaway_accountp02";
let throwawayBank: PublicKey;
// Note: seed 789, group created with `groupBuff`
let preMigrationBank: PublicKey = new PublicKey(
  "A5qx1NMxfb3zywMuuo276KntUQk2zA3r3q6ZNwVBbMZC"
);

describe("Pyth push oracle migration", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  before(async () => {
    // Init throwaway group
    throwawayGroup = Keypair.fromSeed(groupBuff);

    let tx = new Transaction();
    tx.add(
      await groupInitialize(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: throwawayGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, throwawayGroup);
    await banksClient.processTransaction(tx);

    if (verbose) console.log(`*init group: ${throwawayGroup.publicKey}`);

    // Init bank for deposits
    const config = defaultBankConfig();
    const [bankPk] = deriveBankWithSeed(
      bankrunProgram.programId,
      throwawayGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      new BN(seed + 1)
    );
    throwawayBank = bankPk;

    tx = new Transaction();
    tx.add(
      await addBankWithSeed(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: throwawayGroup.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.lstAlphaMint.publicKey,
        config: config,
        seed: new BN(seed + 1),
      }),
      await groupAdmin.mrgnProgram.methods
        .lendingPoolConfigureBankOracle(
          ORACLE_SETUP_PYTH_PUSH,
          oracles.pythPullLst.publicKey
        )
        .accountsPartial({
          group: throwawayGroup.publicKey,
          bank: bankPk,
          admin: groupAdmin.wallet.publicKey,
        })
        .remainingAccounts([
          {
            pubkey: oracles.pythPullLst.publicKey,
            isSigner: false,
            isWritable: false,
          },
        ])
        .instruction()
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
    if (verbose) console.log("init bank: " + throwawayBank);

    // Init user accounts, fund both banks to enable borrows.
    for (let i = 0; i < 2; i++) {
      const u = users[i];
      const kp = Keypair.generate();

      // Init user marginfi account
      let tx = new Transaction();
      tx.add(
        await accountInit(u.mrgnBankrunProgram, {
          marginfiGroup: throwawayGroup.publicKey,
          marginfiAccount: kp.publicKey,
          authority: u.wallet.publicKey,
          feePayer: u.wallet.publicKey,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(u.wallet, kp);
      await banksClient.processTransaction(tx);
      u.accounts.set(USER_ACCOUNT_THROWAWAY, kp.publicKey);
      if (verbose) console.log(`init user ${i} acc: ${kp.publicKey}`);

      // Fund user
      let fundUserTx = new Transaction();
      const provider = getProvider() as AnchorProvider;
      const wallet = provider.wallet as Wallet;
      fundUserTx.add(
        createMintToInstruction(
          ecosystem.lstAlphaMint.publicKey,
          u.lstAlphaAccount,
          wallet.publicKey,
          20 * 10 ** ecosystem.lstAlphaDecimals
        )
      );
      fundUserTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      fundUserTx.sign(wallet.payer);
      await banksClient.processTransaction(fundUserTx);
      if (verbose) console.log(`funded user ${i}`);

      // Deposit assets
      const bankToUse = i === 0 ? throwawayBank : preMigrationBank;
      let depositTx = new Transaction().add(
        await depositIx(u.mrgnBankrunProgram, {
          marginfiAccount: kp.publicKey,
          bank: bankToUse,
          tokenAccount: u.lstAlphaAccount,
          amount: new BN(1 * 10 ** ecosystem.lstAlphaDecimals),
          depositUpToLimit: false,
        })
      );
      depositTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      depositTx.sign(u.wallet);
      await banksClient.processTransaction(depositTx);
      if (verbose) console.log(`deposit to bank ${bankToUse} for user ${i}`);
    }
  });

  it("(user 0) borrows before migration", async () => {
    const user = users[0];
    const userAcc = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const bank = await bankrunProgram.account.bank.fetch(preMigrationBank);
    assertKeysEqual(bank.group, throwawayGroup.publicKey);

    console.log("lst oracle: " + oracles.pythPullLst.publicKey);
    let oracleAcc = await bankrunProgram.provider.connection.getAccountInfo(
      oracles.pythPullLst.publicKey
    );
    const base64Data = oracleAcc.data.toString("base64");
    const priceUpdate = decodePriceUpdateV2(base64Data);
    const feed_id = priceUpdate.price_message.feed_id.toString();
    console.log("feed id: " + feed_id);

    let tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAcc,
        bank: preMigrationBank,
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts([
          [throwawayBank, oracles.pythPullLst.publicKey],
          [preMigrationBank, oracles.pythPullLst.publicKey],
        ]),
        amount: new BN(0.1 * 10 ** ecosystem.lstAlphaDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const acc = await bankrunProgram.account.marginfiAccount.fetch(userAcc);
    assert.equal(acc.lendingAccount.balances[1].active, 1);
  });

  it("(admin) tries to migrate to bad oracle - should fail", async () => {
    let tx = new Transaction().add(
      await migratePythArgs(groupAdmin.mrgnBankrunProgram, {
        bank: preMigrationBank,
        oracle: oracles.wsolOracle.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    // On mainnet, should fail with 6052 WrongOracleAccountKeys. On localnet, fails the later
    // feed_id validation in `check_ai_and_feed_id`, resulting in 6055 (PythPushMismatchedFeedId)
    assertBankrunTxFailed(result, 6055);
  });

  it("(admin) migrates oracle - happy path", async () => {
    const bankBefore = await bankrunProgram.account.bank.fetch(
      preMigrationBank
    );
    assert.equal(bankBefore.config.configFlags, 0);
    let tx = new Transaction().add(
      await migratePythArgs(groupAdmin.mrgnBankrunProgram, {
        bank: preMigrationBank,
        oracle: oracles.pythPullLst.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
    const bankAfter = await bankrunProgram.account.bank.fetch(preMigrationBank);
    assert.equal(bankAfter.config.configFlags, PYTH_PULL_MIGRATED);
  });

  it("(admin) migrates an oracle that's already migrated - nothing happens", async () => {
    const bankBefore = await bankrunProgram.account.bank.fetch(
      preMigrationBank
    );
    assert.equal(bankBefore.config.configFlags, PYTH_PULL_MIGRATED);
    let tx = new Transaction().add(
      await migratePythArgs(groupAdmin.mrgnBankrunProgram, {
        bank: preMigrationBank,
        oracle: oracles.wsolOracle.publicKey, // sneaky sneaky
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
    // Note: no tx failure, we just abort and do nothing.

    const bankAfter = await bankrunProgram.account.bank.fetch(preMigrationBank);
    assert.equal(bankAfter.config.configFlags, PYTH_PULL_MIGRATED);
  });

  it("(admin) migrates a bank created after 0.1.4 - nothing happens", async () => {
    const bankBefore = await bankrunProgram.account.bank.fetch(throwawayBank);
    assert.equal(bankBefore.config.configFlags, PYTH_PULL_MIGRATED);
    let tx = new Transaction().add(
      await migratePythArgs(groupAdmin.mrgnBankrunProgram, {
        bank: throwawayBank,
        oracle: oracles.wsolOracle.publicKey, // sneaky sneaky
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
    // Note: no tx failure, we just abort and do nothing.

    const bankAfter = await bankrunProgram.account.bank.fetch(throwawayBank);
    assert.equal(bankAfter.config.configFlags, PYTH_PULL_MIGRATED);
  });

  it("(user 0) borrows after migration", async () => {
    const user = users[0];
    const userAcc = user.accounts.get(USER_ACCOUNT_THROWAWAY);

    let tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAcc,
        bank: preMigrationBank,
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts([
          [throwawayBank, oracles.pythPullLst.publicKey],
          [preMigrationBank, oracles.pythPullLst.publicKey],
        ]),
        amount: new BN(0.1 * 10 ** ecosystem.lstAlphaDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);
  });
});
