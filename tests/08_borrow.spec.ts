import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairUsdc,
  ecosystem,
  marginfiGroup,
  oracles,
  users,
  verbose,
} from "./rootHooks";
import {
  assertBNApproximately,
  assertI80F48Approx,
  assertI80F48Equal,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import { borrowIx, depositIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { createMintToInstruction } from "@solana/spl-token";
import { updatePriceAccount } from "./utils/pyth_mocks";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";

describe("Borrow funds", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  // Bank has 100 USDC available to borrow
  // User has 2 Token A (worth $20) deposited
  const borrowAmountUsdc = 5;
  const borrowAmountUsdc_native = new BN(
    borrowAmountUsdc * 10 ** ecosystem.usdcDecimals
  );

  it("Oracle data refreshes", async () => {
    const usdcPrice = BigInt(oracles.usdcPrice * 10 ** oracles.usdcDecimals);
    await updatePriceAccount(
      oracles.usdcOracle,
      {
        exponent: -oracles.usdcDecimals,
        aggregatePriceInfo: {
          price: usdcPrice,
          conf: usdcPrice / BigInt(100), // 1% of the price
        },
        twap: {
          // aka ema
          valueComponent: usdcPrice,
        },
      },
      wallet
    );

    const tokenAPrice = BigInt(
      oracles.tokenAPrice * 10 ** oracles.tokenADecimals
    );
    await updatePriceAccount(
      oracles.tokenAOracle,
      {
        exponent: -oracles.tokenADecimals,
        aggregatePriceInfo: {
          price: tokenAPrice,
          conf: tokenAPrice / BigInt(100), // 1% of the price
        },
        twap: {
          // aka ema
          valueComponent: tokenAPrice,
        },
      },
      wallet
    );
  });

  it("(user 0) borrows USDC against their token A position - happy path", async () => {
    const user = users[0];
    const bank = bankKeypairUsdc.publicKey;
    const userUsdcBefore = await getTokenBalance(provider, user.usdcAccount);
    const bankBefore = await program.account.bank.fetch(bank);
    if (verbose) {
      console.log("user 0 USDC before: " + userUsdcBefore.toLocaleString());
      console.log(
        "usdc fees owed to bank: " +
          wrappedI80F48toBigNumber(
            bankBefore.collectedGroupFeesOutstanding
          ).toString()
      );
      console.log(
        "usdc fees owed to program: " +
          wrappedI80F48toBigNumber(
            bankBefore.collectedProgramFeesOutstanding
          ).toString()
      );
    }

    const user0Account = user.accounts.get(USER_ACCOUNT);

    await users[0].mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await borrowIx(program, {
          marginfiGroup: marginfiGroup.publicKey,
          marginfiAccount: user0Account,
          authority: user.wallet.publicKey,
          bank: bank,
          tokenAccount: user.usdcAccount,
          remaining: [
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
            bank,
            oracles.usdcOracle.publicKey,
          ],
          amount: borrowAmountUsdc_native,
        })
      )
    );

    const userAcc = await program.account.marginfiAccount.fetch(user0Account);
    const bankAfter = await program.account.bank.fetch(bank);
    const balances = userAcc.lendingAccount.balances;
    const userUsdcAfter = await getTokenBalance(provider, user.usdcAccount);
    if (verbose) {
      console.log("user 0 USDC after: " + userUsdcAfter.toLocaleString());
      console.log(
        "usdc fees owed to bank: " +
          wrappedI80F48toBigNumber(
            bankAfter.collectedGroupFeesOutstanding
          ).toString()
      );
      console.log(
        "usdc fees owed to program: " +
          wrappedI80F48toBigNumber(
            bankAfter.collectedProgramFeesOutstanding
          ).toString()
      );
    }

    assert.equal(balances[1].active, true);
    assertI80F48Equal(balances[1].assetShares, 0);
    // Note: The first borrow issues shares 1:1 and the shares use the same decimals
    // Note: An origination fee of 0.01 is also incurred here (configured during addBank)
    const originationFee_native = borrowAmountUsdc_native.toNumber() * 0.01;
    const amtUsdcWithFee_native = new BN(
      borrowAmountUsdc_native.toNumber() + originationFee_native
    );
    assertI80F48Approx(balances[1].liabilityShares, amtUsdcWithFee_native);
    assertI80F48Equal(balances[1].emissionsOutstanding, 0);

    let now = Math.floor(Date.now() / 1000);
    assertBNApproximately(balances[1].lastUpdate, now, 2);

    assert.equal(
      userUsdcAfter - borrowAmountUsdc_native.toNumber(),
      userUsdcBefore
    );

    // The origination fee is recorded on the bank. The group gets 98%, the program gets the
    // remaining 2% (see PROGRAM_FEE_RATE)
    const origination_fee_group = originationFee_native * 0.98;
    const origination_fee_program = originationFee_native * 0.02;
    assertI80F48Approx(
      bankAfter.collectedGroupFeesOutstanding,
      origination_fee_group
    );
    assertI80F48Approx(
      bankAfter.collectedProgramFeesOutstanding,
      origination_fee_program
    );
  });
});
