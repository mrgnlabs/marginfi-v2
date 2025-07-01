import { AnchorProvider, BN, getProvider, Program, workspace } from "@coral-xyz/anchor";
import { AccountMeta, Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
  users,
} from "./rootHooks";
import { defaultBankConfig, ORACLE_SETUP_PYTH_PUSH } from "./utils/types";
import { addBankWithSeed } from "./utils/group-instructions";
import { depositIx, withdrawIx } from "./utils/user-instructions";
import { deriveBankWithSeed } from "./utils/pdas";
import { assert } from "chai";
import { expectFailedTxWithError } from "./utils/genericTests";
import { closeBank } from "./utils/group-instructions";
import { USER_ACCOUNT } from "./utils/mocks";

describe("Close bank", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;

  it("bank cannot close with open positions", async () => {
    const seed = new BN(9876);
    const config = defaultBankConfig();
    const [bankKey] = deriveBankWithSeed(
      program.programId,
      marginfiGroup.publicKey,
      ecosystem.tokenAMint.publicKey,
      seed
    );
    await groupAdmin.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await addBankWithSeed(groupAdmin.mrgnProgram, {
          marginfiGroup: marginfiGroup.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: ecosystem.tokenAMint.publicKey,
          config: config,
          seed: seed,
        }),
        await program.methods
          .lendingPoolConfigureBankOracle(
            ORACLE_SETUP_PYTH_PUSH,
            oracles.tokenAOracle.publicKey
          )
          .accountsPartial({
            group: marginfiGroup.publicKey,
            bank: bankKey,
            admin: groupAdmin.wallet.publicKey,
          })
          .remainingAccounts([
            {
              pubkey: oracles.tokenAOracle.publicKey,
              isSigner: false,
              isWritable: false,
            } as AccountMeta,
          ])
          .instruction()
      )
    );

    const userAcc = users[0].accounts.get(USER_ACCOUNT);
    const amount = new BN(1 * 10 ** ecosystem.tokenADecimals);
    await users[0].mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await depositIx(users[0].mrgnProgram, {
          marginfiAccount: userAcc,
          bank: bankKey,
          tokenAccount: users[0].tokenAAccount,
          amount: amount,
          depositUpToLimit: false,
        })
      )
    );

    const bankAfterDeposit = await program.account.bank.fetch(bankKey);
    assert.equal(bankAfterDeposit.lendingPositionCount, 1);
    assert.equal(bankAfterDeposit.positionCount, 1);

    await expectFailedTxWithError(
      async () => {
        await groupAdmin.mrgnProgram.provider.sendAndConfirm(
          new Transaction().add(
            closeBank(groupAdmin.mrgnProgram, {
              group: marginfiGroup.publicKey,
              bank: bankKey,
              admin: groupAdmin.wallet.publicKey,
            })
          )
        );
      },
      "IllegalAction",
      6043
    );

    await users[0].mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await withdrawIx(users[0].mrgnProgram, {
          marginfiAccount: userAcc,
          bank: bankKey,
          tokenAccount: users[0].tokenAAccount,
          remaining: [bankKey, oracles.tokenAOracle.publicKey],
          amount: new BN(0),
          withdrawAll: true,
        })
      )
    );

    const bankAfterWithdraw = await program.account.bank.fetch(bankKey);
    assert.equal(bankAfterWithdraw.lendingPositionCount, 0);
    assert.equal(bankAfterWithdraw.positionCount, 0);

    await groupAdmin.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        closeBank(groupAdmin.mrgnProgram, {
          group: marginfiGroup.publicKey,
          bank: bankKey,
          admin: groupAdmin.wallet.publicKey,
        })
      )
    );

    const info = await provider.connection.getAccountInfo(bankKey);
    assert.isNull(info);
  });
});
