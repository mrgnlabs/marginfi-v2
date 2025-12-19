import {
  AnchorProvider,
  BN,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { createMintToInstruction } from "@solana/spl-token";
import { Keypair, Transaction } from "@solana/web3.js";
import { assert } from "chai";

import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairUsdc,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
} from "./rootHooks";
import {
  assertI80F48Approx,
  expectFailedTxWithError,
} from "./utils/genericTests";
import {
  accountInit,
  composeRemainingAccounts,
  depositIx,
  setAccountFreezeIx,
  withdrawIx,
} from "./utils/user-instructions";
import { ACCOUNT_FROZEN } from "./utils/types";
import { MockUser, SetupTestUserOptions, setupTestUser } from "./utils/mocks";

describe("Account freeze", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = program.provider as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  const frozenAccount = Keypair.generate();
  const initialDeposit = new BN(50 * 10 ** ecosystem.usdcDecimals);
  const adminTopUp = new BN(20 * 10 ** ecosystem.usdcDecimals);
  const adminWithdraw = new BN(10 * 10 ** ecosystem.usdcDecimals);
  const authorityAttemptWhileFrozen = new BN(
    1 * 10 ** ecosystem.usdcDecimals
  );
  const authorityUnfreezeDeposit = new BN(5 * 10 ** ecosystem.usdcDecimals);

  let freezeUser: MockUser;

  before("setup dedicated frozen account with liquidity", async () => {
    const options: SetupTestUserOptions = {
      marginProgram: program,
      forceWallet: undefined,
      wsolMint: ecosystem.wsolMint.publicKey,
      tokenAMint: ecosystem.tokenAMint.publicKey,
      tokenBMint: ecosystem.tokenBMint.publicKey,
      usdcMint: ecosystem.usdcMint.publicKey,
      lstAlphaMint: ecosystem.lstAlphaMint.publicKey,
    };

    freezeUser = await setupTestUser(provider, wallet.payer, options);

    await freezeUser.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await accountInit(freezeUser.mrgnProgram, {
          marginfiGroup: marginfiGroup.publicKey,
          marginfiAccount: frozenAccount.publicKey,
          authority: freezeUser.wallet.publicKey,
          feePayer: freezeUser.wallet.publicKey,
        })
      ),
      [frozenAccount]
    );

    const mintTx = new Transaction();
    mintTx.add(
      createMintToInstruction(
        ecosystem.usdcMint.publicKey,
        freezeUser.usdcAccount,
        wallet.publicKey,
        200 * 10 ** ecosystem.usdcDecimals
      )
    );
    mintTx.add(
      createMintToInstruction(
        ecosystem.usdcMint.publicKey,
        groupAdmin.usdcAccount,
        wallet.publicKey,
        200 * 10 ** ecosystem.usdcDecimals
      )
    );
    await program.provider.sendAndConfirm(mintTx);

    await freezeUser.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await depositIx(freezeUser.mrgnProgram, {
          marginfiAccount: frozenAccount.publicKey,
          bank: bankKeypairUsdc.publicKey,
          tokenAccount: freezeUser.usdcAccount,
          amount: initialDeposit,
          depositUpToLimit: false,
        })
      )
    );
  });

  it("(admin) toggles the account freeze flag", async () => {
    await groupAdmin.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await setAccountFreezeIx(groupAdmin.mrgnProgram, {
          group: marginfiGroup.publicKey,
          marginfiAccount: frozenAccount.publicKey,
          admin: groupAdmin.wallet.publicKey,
          frozen: true,
        })
      )
    );

    let account = await program.account.marginfiAccount.fetch(
      frozenAccount.publicKey
    );
    assert.equal(
      account.accountFlags.toNumber() & ACCOUNT_FROZEN,
      ACCOUNT_FROZEN
    );

    await groupAdmin.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await setAccountFreezeIx(groupAdmin.mrgnProgram, {
          group: marginfiGroup.publicKey,
          marginfiAccount: frozenAccount.publicKey,
          admin: groupAdmin.wallet.publicKey,
          frozen: false,
        })
      )
    );

    account = await program.account.marginfiAccount.fetch(
      frozenAccount.publicKey
    );
    assert.equal(account.accountFlags.toNumber() & ACCOUNT_FROZEN, 0);
  });

  it("(authority) cannot deposit when frozen but admin can", async () => {
    await groupAdmin.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await setAccountFreezeIx(groupAdmin.mrgnProgram, {
          group: marginfiGroup.publicKey,
          marginfiAccount: frozenAccount.publicKey,
          admin: groupAdmin.wallet.publicKey,
          frozen: true,
        })
      )
    );

    await expectFailedTxWithError(
      async () => {
        await freezeUser.mrgnProgram.provider.sendAndConfirm(
          new Transaction().add(
            await depositIx(freezeUser.mrgnProgram, {
              marginfiAccount: frozenAccount.publicKey,
              bank: bankKeypairUsdc.publicKey,
              tokenAccount: freezeUser.usdcAccount,
              amount: authorityAttemptWhileFrozen,
              depositUpToLimit: false,
            })
          )
        );
      },
      "AccountFrozen",
      6103
    );

    await groupAdmin.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await depositIx(groupAdmin.mrgnProgram, {
          marginfiAccount: frozenAccount.publicKey,
          bank: bankKeypairUsdc.publicKey,
          tokenAccount: groupAdmin.usdcAccount,
          amount: adminTopUp,
          depositUpToLimit: false,
        })
      )
    );

    const account = await program.account.marginfiAccount.fetch(
      frozenAccount.publicKey
    );
    const usdcBalance = account.lendingAccount.balances.find((bal) =>
      bal.bankPk.equals(bankKeypairUsdc.publicKey)
    );
    assert.ok(usdcBalance);
    assertI80F48Approx(
      usdcBalance.assetShares,
      initialDeposit.add(adminTopUp),
      10
    );
    assert.equal(account.accountFlags.toNumber() & ACCOUNT_FROZEN, ACCOUNT_FROZEN);
  });

  it("(authority) cannot withdraw when frozen; admin can and unfreeze restores access", async () => {
    await expectFailedTxWithError(
      async () => {
        await freezeUser.mrgnProgram.provider.sendAndConfirm(
          new Transaction().add(
            await withdrawIx(freezeUser.mrgnProgram, {
              marginfiAccount: frozenAccount.publicKey,
              bank: bankKeypairUsdc.publicKey,
              tokenAccount: freezeUser.usdcAccount,
              remaining: composeRemainingAccounts([
                [bankKeypairUsdc.publicKey, oracles.usdcOracle.publicKey],
              ]),
              amount: adminWithdraw,
              withdrawAll: false,
            })
          )
        );
      },
      "AccountFrozen",
      6103
    );

    await groupAdmin.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await withdrawIx(groupAdmin.mrgnProgram, {
          marginfiAccount: frozenAccount.publicKey,
          bank: bankKeypairUsdc.publicKey,
          tokenAccount: groupAdmin.usdcAccount,
          remaining: composeRemainingAccounts([
            [bankKeypairUsdc.publicKey, oracles.usdcOracle.publicKey],
          ]),
          amount: adminWithdraw,
          withdrawAll: false,
        })
      )
    );

    await groupAdmin.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await setAccountFreezeIx(groupAdmin.mrgnProgram, {
          group: marginfiGroup.publicKey,
          marginfiAccount: frozenAccount.publicKey,
          admin: groupAdmin.wallet.publicKey,
          frozen: false,
        })
      )
    );

    await freezeUser.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await depositIx(freezeUser.mrgnProgram, {
          marginfiAccount: frozenAccount.publicKey,
          bank: bankKeypairUsdc.publicKey,
          tokenAccount: freezeUser.usdcAccount,
          amount: authorityUnfreezeDeposit,
          depositUpToLimit: false,
        })
      )
    );

    const account = await program.account.marginfiAccount.fetch(
      frozenAccount.publicKey
    );
    const usdcBalance = account.lendingAccount.balances.find((bal) =>
      bal.bankPk.equals(bankKeypairUsdc.publicKey)
    );
    const expected = initialDeposit
      .add(adminTopUp)
      .sub(adminWithdraw)
      .add(authorityUnfreezeDeposit);
    assert.ok(usdcBalance);
    assertI80F48Approx(usdcBalance.assetShares, expected, 10);
    assert.equal(account.accountFlags.toNumber() & ACCOUNT_FROZEN, 0);
  });
});
