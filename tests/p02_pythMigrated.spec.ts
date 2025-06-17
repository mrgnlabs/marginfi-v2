import { BN, Program, workspace } from "@coral-xyz/anchor";
import { Transaction, PublicKey } from "@solana/web3.js";
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
import { borrowIx, composeRemainingAccounts } from "./utils/user-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { USER_ACCOUNT } from "./utils/mocks";
import { assert } from "chai";

const MIGRATION_SEED = new BN(55);

describe("Pyth push oracle migration", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  let bankKey: PublicKey;

  before(async () => {
    [bankKey] = deriveBankWithSeed(
      bankrunProgram.programId,
      marginfiGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      MIGRATION_SEED
    );
  });

  it("(user 0) borrows before migration", async () => {
    const user = users[0];
    const userAcc = user.accounts.get(USER_ACCOUNT);

    let tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAcc,
        bank: bankKey,
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts([
          [bankKeypairA.publicKey, oracles.tokenAOracle.publicKey],
          [bankKey, oracles.wsolOracle.publicKey],
        ]),
        amount: new BN(1 * 10 ** ecosystem.lstAlphaDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);
  });

  it("(admin) migrates oracle", async () => {
    let tx = new Transaction().add(
      await bankrunProgram.methods
        .migratePythPushOracle()
        .accounts({
          marginfiGroup: marginfiGroup.publicKey,
          bank: bankKey,
          oracle: oracles.wsolOracle.publicKey,
        })
        .instruction()
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
  });

  it("(user 0) borrows after migration", async () => {
    const user = users[0];
    const userAcc = user.accounts.get(USER_ACCOUNT);

    let tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAcc,
        bank: bankKey,
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts([
          [bankKeypairA.publicKey, oracles.tokenAOracle.publicKey],
          [bankKey, oracles.wsolOracle.publicKey],
        ]),
        amount: new BN(1 * 10 ** ecosystem.lstAlphaDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);
  });
});
