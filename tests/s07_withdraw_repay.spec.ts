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
} from "./rootHooks";
import {
  editStakedSettings,
  propagateStakedSettings,
} from "./utils/group-instructions";
import { deriveBankWithSeed, deriveStakedSettings } from "./utils/pdas";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { assert } from "chai";
import {
  assertKeysEqual,
  assertI80F48Approx,
  assertBNEqual,
  assertBankrunTxFailed,
} from "./utils/genericTests";
import {
  defaultStakedInterestSettings,
  StakedSettingsEdit,
} from "./utils/types";
import { LST_ATA, USER_ACCOUNT } from "./utils/mocks";
import { borrowIx, depositIx, withdrawIx } from "./utils/user-instructions";

describe("Withdraw staked asset", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  let settingsKey: PublicKey;
  let bankKey: PublicKey;

  before(async () => {
    [settingsKey] = deriveStakedSettings(
      program.programId,
      marginfiGroup.publicKey
    );
    [bankKey] = deriveBankWithSeed(
      program.programId,
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
    const user = users[3];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    const userLstAta = user.accounts.get(LST_ATA);

    let tx = new Transaction().add(
      await withdrawIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: userLstAta,
        amount: new BN(0.1 * 10 ** ecosystem.wsolDecimals),
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
    await banksClient.tryProcessTransaction(tx);

    // TODO assert balances changes as expected...
  });

  // TODO repay, withdraw all 
});
