import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { LAMPORTS_PER_SOL, SystemProgram, Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairSol,
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  marginfiGroup,
  oracles,
  users,
  validators,
  verbose,
} from "./rootHooks";
import {
  assertBankrunTxFailed,
  assertI80F48Approx,
  assertKeysEqual,
} from "./utils/genericTests";
import { assert } from "chai";
import { borrowIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { cacheSolExchangeRate } from "./utils/group-instructions";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";

describe("Borrow power grows as v0 Staked SOL gains value from appreciation", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  // User 2 has a validator 0 staked depost [0] position - net value = 1 LST token
  // Users 0/1/2 deposited 10 SOL each, so a total of 30 is staked with validator 0
  /** SOL to add to the validator as pretend-earned epoch rewards */
  const appreciation = 30;

  it("(user 2) tries to borrow 1.1 SOL against 1 v0 STAKED - fails, not enough funds", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    let tx = new Transaction().add(
      await borrowIx(program, {
        marginfiGroup: marginfiGroup.publicKey,
        marginfiAccount: userAccount,
        authority: user.wallet.publicKey,
        bank: bankKeypairSol.publicKey,
        tokenAccount: user.wsolAccount,
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
        amount: new BN(1.1 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    // 6010 (Generic risk engine rejection)
    assertBankrunTxFailed(result, "0x177a");

    const userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[1].active, false);
  });

  // Note: there is some natural appreciation here because a few epochs have elapsed...
  // TODO: Show math for expected appreciation due to epochs advancing
  it("(permissionless) v0 cache stake - happy path (natural appreciation)", async () => {
    let tx = new Transaction().add(
      await cacheSolExchangeRate(program, {
        bank: validators[0].bank,
        lstMint: validators[0].splMint,
        solPool: validators[0].splStake,
        stakePool: validators[0].splPool,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(wallet.payer); // provider wallet pays the tx fee
    await banksClient.processTransaction(tx);

    const bank = await bankrunProgram.account.bank.fetch(validators[0].bank);
    if (verbose) {
      console.log(
        "1 [validator 0 LST token] is now worth: " +
          wrappedI80F48toBigNumber(bank.solAppreciationRate).toString() +
          " SOL"
      );
    }
    assertI80F48Approx(bank.solAppreciationRate, 1.033, 0.01);
  });

  it("(attacker) tries to sneak a bad spl pool - should fail", async () => {
    let tx = new Transaction().add(
      await cacheSolExchangeRate(program, {
        bank: validators[0].bank,
        lstMint: validators[0].splMint,
        solPool: wallet.publicKey,
        stakePool: validators[0].splPool,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(wallet.payer); // provider wallet pays the tx fee
    let result = await banksClient.tryProcessTransaction(tx);
    // 6048 (Stake pool validation failed)
    assertBankrunTxFailed(result, "0x17a0");
  });

  // Here we mock epoch rewards by simply minting SOL into the validator's pool without staking
  it("v0 stake grows by " + appreciation + " SOL", async () => {
    let tx = new Transaction();
    tx.add(
      SystemProgram.transfer({
        fromPubkey: wallet.publicKey,
        toPubkey: validators[0].splStake,
        lamports: appreciation * LAMPORTS_PER_SOL,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(wallet.payer);
    await banksClient.processTransaction(tx);
  });

  it("(permissionless) validator 0 cache stake - 1 LST is now worth 2 SOL", async () => {
    let tx = new Transaction().add(
      await cacheSolExchangeRate(program, {
        bank: validators[0].bank,
        lstMint: validators[0].splMint,
        solPool: validators[0].splStake,
        stakePool: validators[0].splPool,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(wallet.payer); // provider wallet pays the tx fee
    await banksClient.processTransaction(tx);

    const bank = await bankrunProgram.account.bank.fetch(validators[0].bank);
    if (verbose) {
      console.log(
        "1 [validator 0 LST token] is now worth: " +
          wrappedI80F48toBigNumber(bank.solAppreciationRate).toString() +
          " SOL"
      );
    }
    assertI80F48Approx(bank.solAppreciationRate, 2.033, 0.01);
  });

  // The account is now worth enough for this borrow to succeed!
  it("(user 2) borrows 1.1 SOL against their STAKED position - succeeds", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    let tx = new Transaction().add(
      await borrowIx(program, {
        marginfiGroup: marginfiGroup.publicKey,
        marginfiAccount: userAccount,
        authority: user.wallet.publicKey,
        bank: bankKeypairSol.publicKey,
        tokenAccount: user.wsolAccount,
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
        amount: new BN(1.1 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[1].active, true);
    assertKeysEqual(balances[1].bankPk, bankKeypairSol.publicKey);
  });
});
