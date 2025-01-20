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
  assertBNEqual,
  assertI80F48Approx,
  assertI80F48Equal,
  expectFailedTxWithError,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import {
  borrowIx,
  depositIx,
  repayIx,
  withdrawEmissionsIx,
  withdrawIx,
} from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { createMintToInstruction } from "@solana/spl-token";
import { updatePriceAccount } from "./utils/pyth_mocks";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { u64MAX_BN } from "./utils/types";

describe("Withdraw funds", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  const withdrawAmountTokenA = 0.1;
  const withdrawAmountTokenA_native = new BN(
    withdrawAmountTokenA * 10 ** ecosystem.tokenADecimals
  );

  const repayAmountUsdc = 0.1;
  const repayAmountUsdc_native = new BN(
    repayAmountUsdc * 10 ** ecosystem.usdcDecimals
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

  it("(user 0) withdraws some token A - happy path", async () => {
    const user = users[0];
    const userAccKey = user.accounts.get(USER_ACCOUNT);
    const userAccBefore = await program.account.marginfiAccount.fetch(
      userAccKey
    );
    const balancesBefore = userAccBefore.lendingAccount.balances;

    const bank = bankKeypairA.publicKey;
    const userTokenABefore = await getTokenBalance(
      provider,
      user.tokenAAccount
    );
    const bankBefore = await program.account.bank.fetch(bank);
    const vaultUsdcBefore = await getTokenBalance(
      provider,
      bankBefore.liquidityVault
    );

    await user.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await withdrawIx(user.mrgnProgram, {
          marginfiAccount: userAccKey,
          bank: bank,
          tokenAccount: user.tokenAAccount,
          remaining: [
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
          ],
          amount: withdrawAmountTokenA_native,
        })
      )
    );

    const userAccAfter = await program.account.marginfiAccount.fetch(
      userAccKey
    );
    const bankAfter = await program.account.bank.fetch(bank);
    const vaultUsdcAfter = await getTokenBalance(
      provider,
      bankAfter.liquidityVault
    );
    const balancesAfter = userAccAfter.lendingAccount.balances;
    const userTokenAAfter = await getTokenBalance(provider, user.tokenAAccount);

    const withdrawExpected = withdrawAmountTokenA_native.toNumber();
    if (verbose) {
      console.log(
        "User 0 withdrew " +
          withdrawAmountTokenA +
          " token A (" +
          withdrawExpected.toString() +
          ") native"
      );
    }

    // user gains the token A, the liquidity vault loses it....
    assert.equal(userTokenAAfter, userTokenABefore + withdrawExpected);
    assert.equal(vaultUsdcAfter, vaultUsdcBefore - withdrawExpected);

    // User loses the shares of Token A...
    // Since there hasn't been any interest (no Token A borrowed), shares and Token A are 1:1
    const sharesBefore = wrappedI80F48toBigNumber(
      balancesBefore[0].assetShares
    ).toNumber();
    const sharesAfter = wrappedI80F48toBigNumber(
      balancesAfter[0].assetShares
    ).toNumber();
    assert.equal(sharesAfter, sharesBefore - withdrawExpected);

    // The bank has also lost the same amount of shares...
    const bankSharesBefore = wrappedI80F48toBigNumber(
      bankBefore.totalAssetShares
    ).toNumber();
    const bankSharesAfter = wrappedI80F48toBigNumber(
      bankAfter.totalAssetShares
    ).toNumber();
    assert.equal(bankSharesAfter, bankSharesBefore - withdrawExpected);
  });

  it("(user 0) repays some USDC debt - happy path", async () => {
    const user = users[0];
    const userAccKey = user.accounts.get(USER_ACCOUNT);
    const userAccBefore = await program.account.marginfiAccount.fetch(
      userAccKey
    );
    const balancesBefore = userAccBefore.lendingAccount.balances;

    const bank = bankKeypairUsdc.publicKey;
    const userUsdcBefore = await getTokenBalance(provider, user.usdcAccount);
    const bankBefore = await program.account.bank.fetch(bank);
    const vaultUsdcBefore = await getTokenBalance(
      provider,
      bankBefore.liquidityVault
    );

    await user.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await repayIx(user.mrgnProgram, {
          marginfiAccount: userAccKey,
          bank: bank,
          tokenAccount: user.usdcAccount,
          remaining: [
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
          ],
          amount: repayAmountUsdc_native,
        })
      )
    );

    const userAccAfter = await program.account.marginfiAccount.fetch(
      userAccKey
    );
    const bankAfter = await program.account.bank.fetch(bank);
    const vaultUsdcAfter = await getTokenBalance(
      provider,
      bankAfter.liquidityVault
    );
    const balancesAfter = userAccAfter.lendingAccount.balances;
    const userUsdcAfter = await getTokenBalance(provider, user.usdcAccount);

    const repayExpected = repayAmountUsdc_native.toNumber();
    if (verbose) {
      console.log(
        "User 0 repaid " +
          repayAmountUsdc +
          " usdc (" +
          repayExpected.toString() +
          ") native"
      );
    }

    // user gains loses the USDC, the liquidity vault loses it....
    assert.equal(userUsdcAfter, userUsdcBefore - repayExpected);
    assert.equal(vaultUsdcAfter, vaultUsdcBefore + repayExpected);

    // User loses the liability shares of USDC...
    // USDC has some borrows, so there is trivial interest here that affects accounting
    const sharesBefore = wrappedI80F48toBigNumber(
      balancesBefore[1].liabilityShares
    ).toNumber();
    const sharesAfter = wrappedI80F48toBigNumber(
      balancesAfter[1].liabilityShares
    ).toNumber();
    assert.approximately(sharesAfter, sharesBefore - repayExpected, 1);

    // The bank has also lost the same amount of shares...
    const bankSharesBefore = wrappedI80F48toBigNumber(
      bankBefore.totalLiabilityShares
    ).toNumber();
    const bankSharesAfter = wrappedI80F48toBigNumber(
      bankAfter.totalLiabilityShares
    ).toNumber();
    assert.approximately(bankSharesAfter, bankSharesBefore - repayExpected, 1);
  });

  it("(user 0) tries to repay all without claiming emissions - should fail", async () => {
    const user = users[0];
    const userAccKey = user.accounts.get(USER_ACCOUNT);
    const bank = bankKeypairUsdc.publicKey;
    expectFailedTxWithError(async () => {
      await user.mrgnProgram.provider.sendAndConfirm(
        new Transaction().add(
          await repayIx(user.mrgnProgram, {
            marginfiAccount: userAccKey,
            bank: bank,
            tokenAccount: user.usdcAccount,
            remaining: [
              bankKeypairA.publicKey,
              oracles.tokenAOracle.publicKey,
              bankKeypairUsdc.publicKey,
              oracles.usdcOracle.publicKey,
            ],
            amount: u64MAX_BN,
            repayAll: true,
          })
        )
      );
    }, "CannotCloseOutstandingEmissions");
  });

  it("(user 0) claims emissions (in token B) before repaying their balance - happy path", async () => {
    const user = users[0];
    const userAccKey = user.accounts.get(USER_ACCOUNT);
    const bank = bankKeypairUsdc.publicKey;

    const userBBefore = await getTokenBalance(provider, user.tokenBAccount);

    await user.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await withdrawEmissionsIx(user.mrgnProgram, {
          marginfiAccount: userAccKey,
          bank: bank,
          tokenAccount: user.tokenBAccount,
        })
      )
    );

    const userBAfter = await getTokenBalance(provider, user.tokenBAccount);
    const diff = userBAfter - userBBefore;
    if (verbose) {
      console.log("Claimed Token B emissions: " + diff);
    }

    // TODO we can probably assert a more specific balance here with some maths...
    assert.ok(diff > 0);

    // TODO assert changes to the emissions accounts...
  });

  it("(user 0) repays all of their USDC debt - happy path", async () => {
    const user = users[0];
    const userAccKey = user.accounts.get(USER_ACCOUNT);
    const userAccBefore = await program.account.marginfiAccount.fetch(
      userAccKey
    );
    const balancesBefore = userAccBefore.lendingAccount.balances;

    const bank = bankKeypairUsdc.publicKey;
    const userUsdcBefore = await getTokenBalance(provider, user.usdcAccount);
    const bankBefore = await program.account.bank.fetch(bank);
    const vaultUsdcBefore = await getTokenBalance(
      provider,
      bankBefore.liquidityVault
    );
    const actualOwed =
      wrappedI80F48toBigNumber(balancesBefore[1].liabilityShares).toNumber() *
      wrappedI80F48toBigNumber(bankBefore.liabilityShareValue).toNumber();

    await user.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        // Needs to occur within the same tx bundle even though we just collected it (a trivial
        // amount can build up even in a single slot, and we don't want this to mess up accounting)
        await withdrawEmissionsIx(user.mrgnProgram, {
          marginfiAccount: userAccKey,
          bank: bank,
          tokenAccount: user.tokenBAccount,
        }),
        await repayIx(user.mrgnProgram, {
          marginfiAccount: userAccKey,
          bank: bank,
          tokenAccount: user.usdcAccount,
          remaining: [
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
          ],
          amount: u64MAX_BN,
          repayAll: true,
        })
      )
    );

    const userAccAfter = await program.account.marginfiAccount.fetch(
      userAccKey
    );
    const bankAfter = await program.account.bank.fetch(bank);
    const vaultUsdcAfter = await getTokenBalance(
      provider,
      bankAfter.liquidityVault
    );
    const balancesAfter = userAccAfter.lendingAccount.balances;
    const userUsdcAfter = await getTokenBalance(provider, user.usdcAccount);

    if (verbose) {
      console.log(
        "User 0 repaid entire USDC balance: ~" + actualOwed.toLocaleString()
      );
    }

    // USDC has some borrows, so there is trivial interest here that affects accounting

    // user gains loses the USDC, the liquidity vault loses it....
    assert.approximately(userUsdcAfter, userUsdcBefore - actualOwed, 2);
    assert.approximately(vaultUsdcAfter, vaultUsdcBefore + actualOwed, 2);

    // User loses the liability shares of USDC...
    const sharesBefore = wrappedI80F48toBigNumber(
      balancesBefore[1].liabilityShares
    ).toNumber();
    const sharesAfter = wrappedI80F48toBigNumber(
      balancesAfter[1].liabilityShares
    ).toNumber();
    assert.approximately(sharesAfter, sharesBefore - actualOwed, 2);

    // The bank has also lost the same amount of shares...
    const bankSharesBefore = wrappedI80F48toBigNumber(
      bankBefore.totalLiabilityShares
    ).toNumber();
    const bankSharesAfter = wrappedI80F48toBigNumber(
      bankAfter.totalLiabilityShares
    ).toNumber();
    assert.approximately(bankSharesAfter, bankSharesBefore - actualOwed, 2);
  });

  // TODO withdraw all, then restore the original balances via deposit/borrow
});
