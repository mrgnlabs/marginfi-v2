import { workspace, Program } from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import BN from "bn.js";
import { Marginfi } from "../target/types/marginfi";
import {
  marginfiGroup,
  validators,
  groupAdmin,
  oracles,
  bankrunContext,
  banksClient,
  bankrunProgram,
  users,
  ecosystem,
  bankKeypairSol,
  bankRunProvider,
  verbose,
} from "./rootHooks";
import {
  editStakedSettings,
  propagateStakedSettings,
} from "./utils/group-instructions";
import { deriveBankWithSeed, deriveStakedSettings } from "./utils/pdas";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import {
  bigNumberToWrappedI80F48,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import { assert } from "chai";
import {
  assertKeysEqual,
  assertI80F48Approx,
  assertBNEqual,
  assertBankrunTxFailed,
  getTokenBalance,
  assertI80F48Equal,
} from "./utils/genericTests";
import {
  defaultStakedInterestSettings,
  StakedSettingsEdit,
} from "./utils/types";
import { LST_ATA, USER_ACCOUNT } from "./utils/mocks";
import {
  borrowIx,
  depositIx,
  repayIx,
  withdrawIx,
} from "./utils/user-instructions";

describe("Withdraw staked asset", () => {
  let settingsKey: PublicKey;
  let bankKey: PublicKey;

  before(async () => {
    [settingsKey] = deriveStakedSettings(
      bankrunProgram.programId,
      marginfiGroup.publicKey
    );
    [bankKey] = deriveBankWithSeed(
      bankrunProgram.programId,
      marginfiGroup.publicKey,
      validators[0].splMint,
      new BN(0)
    );
  });

  it("(user 3) deposits some native staked and borrows SOL against it - happy path", async () => {
    const user = users[3];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    const userLstAta = user.accounts.get(LST_ATA);

    let depositTx = new Transaction().add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: userLstAta,
        amount: new BN(1 * 10 ** ecosystem.wsolDecimals),
      })
    );

    depositTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    depositTx.sign(user.wallet);
    await banksClient.tryProcessTransaction(depositTx);

    let borrowTx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: bankKeypairSol.publicKey,
        tokenAccount: user.wsolAccount,
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
        amount: new BN(0.5 * 10 ** ecosystem.wsolDecimals),
      })
    );
    borrowTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    borrowTx.sign(user.wallet);
    await banksClient.processTransaction(borrowTx);
  });

  it("(user 3) withdraws a small amount of native staked position - happy path", async () => {
    const amtNative = 0.1 * 10 ** ecosystem.wsolDecimals;
    const user = users[3];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    const userLstAta = user.accounts.get(LST_ATA);

    const lstBefore = await getTokenBalance(bankRunProvider, userLstAta);

    let tx = new Transaction().add(
      await withdrawIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: userLstAta,
        amount: new BN(amtNative),
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const lstAfter = await getTokenBalance(bankRunProvider, userLstAta);
    assert.equal(lstAfter, lstBefore + amtNative);

    const userAcc = await user.mrgnBankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[0].active, true);

    // TODO assert other balances changes as expected...
  });

  it("(user 3) repays a small amount of SOL borrowed against stake - happy path", async () => {
    const amtNative = 0.1 * 10 ** ecosystem.wsolDecimals;
    const user = users[3];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    const solBefore = await getTokenBalance(bankRunProvider, user.wsolAccount);

    let tx = new Transaction().add(
      await repayIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: bankKeypairSol.publicKey,
        tokenAccount: user.wsolAccount,
        amount: new BN(amtNative),
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const solAfter = await getTokenBalance(bankRunProvider, user.wsolAccount);
    assert.equal(solAfter, solBefore - amtNative);

    const userAcc = await user.mrgnBankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[1].active, true);

    // TODO assert other balances changes as expected...
  });

  it("(user 3) repays the entire borrowed SOL balance - happy path", async () => {
    const amtNative = 0.1 * 10 ** ecosystem.wsolDecimals;
    const user = users[3];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    const solBefore = await getTokenBalance(bankRunProvider, user.wsolAccount);
    const userAccBefore =
      await user.mrgnBankrunProgram.account.marginfiAccount.fetch(userAccount);
    const bankBefore = await user.mrgnBankrunProgram.account.bank.fetch(
      bankKeypairSol.publicKey
    );
    const amtExpected =
      wrappedI80F48toBigNumber(
        userAccBefore.lendingAccount.balances[1].liabilityShares
      ).toNumber() *
      wrappedI80F48toBigNumber(bankBefore.liabilityShareValue).toNumber();

    let tx = new Transaction().add(
      await repayIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: bankKeypairSol.publicKey,
        tokenAccount: user.wsolAccount,
        amount: new BN(amtNative),
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
        repayAll: true,
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const solAfter = await getTokenBalance(bankRunProvider, user.wsolAccount);
    assert.approximately(solAfter, solBefore - amtExpected, 2);

    const userAcc = await user.mrgnBankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const balances = userAcc.lendingAccount.balances;
    assertI80F48Equal(balances[1].liabilityShares, 0);
    assert.equal(balances[1].active, false);

    // TODO assert other balances changes as expected...
  });

  it("(user 3) withdraws the entire native staked position - happy path", async () => {
    const amtNative = 0.1 * 10 ** ecosystem.wsolDecimals;
    const user = users[3];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    const userLstAta = user.accounts.get(LST_ATA);

    const lstBefore = await getTokenBalance(bankRunProvider, userLstAta);
    const userAccBefore =
      await user.mrgnBankrunProgram.account.marginfiAccount.fetch(userAccount);
    const bankBefore = await user.mrgnBankrunProgram.account.bank.fetch(
      bankKeypairSol.publicKey
    );
    const amtExpected =
      wrappedI80F48toBigNumber(
        userAccBefore.lendingAccount.balances[0].assetShares
      ).toNumber() *
      wrappedI80F48toBigNumber(bankBefore.assetShareValue).toNumber();

    let tx = new Transaction().add(
      await withdrawIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: userLstAta,
        amount: new BN(amtNative),
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
        withdrawAll: true,
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const lstAfter = await getTokenBalance(bankRunProvider, userLstAta);
    assert.equal(lstAfter, lstBefore + amtExpected);

    const userAcc = await user.mrgnBankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const balances = userAcc.lendingAccount.balances;
    assertI80F48Equal(balances[0].assetShares, 0);
    assert.equal(balances[0].active, false);

    // TODO assert other balances changes as expected...
  });
});
