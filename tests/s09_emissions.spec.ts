/**
 * The "point" of this test is to additional test emissions with Bankrun time warps, but it also
 * serves a secondary purpose of validating that emissions works with eccentric bank setups like
 * staked collateral.
 */

import {
  AnchorProvider,
  BN,
  getProvider,
  Wallet,
} from "@coral-xyz/anchor";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  bankrunContext,
  bankrunProgram,
  bankRunProvider,
  banksClient,
  ecosystem,
  groupAdmin,
  numUsers,
  users,
  validators,
  verbose,
} from "./rootHooks";
import {
  assertBankrunTxFailed,
  assertKeyDefault,
  assertKeysEqual,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import {
  settleEmissionsIx,
  updateEmissionsDestination,
  withdrawEmissionsIx,
  withdrawEmissionsPermissionlessIx,
} from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import {
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import {
  EMISSIONS_FLAG_BORROW_ACTIVE,
  EMISSIONS_FLAG_LENDING_ACTIVE,
} from "./utils/types";
import {
  setupEmissions,
} from "./utils/group-instructions";
import { getEpochAndSlot } from "./utils/stake-utils";
import { createMintToInstruction } from "@solana/spl-token";

describe("Set up emissions on staked collateral assets", () => {
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  const emissionRate = new BN(500_000 * 10 ** ecosystem.tokenBDecimals);
  const totalEmissions = new BN(1_000_000 * 10 ** ecosystem.tokenBDecimals);

  // NOTE: these change slightly due to interest, but not enough to really matter for test purposes
  let userDeposits: number[] = [];
  let netDeposits: number;

  /// Some wallet the user wants to use for emissions. This could also be their own wallet, user can
  /// pick arbitrarily.
  const externalWallet: Keypair = Keypair.generate();
  const bAta = getAssociatedTokenAddressSync(
    ecosystem.tokenBMint.publicKey,
    externalWallet.publicKey
  );

  before(async () => {
    // Fund the group admin with a bunch of Token B for emissions
    let fundTx: Transaction = new Transaction().add(
      createMintToInstruction(
        ecosystem.tokenBMint.publicKey,
        groupAdmin.tokenBAccount,
        wallet.publicKey,
        BigInt(100_000_000) * BigInt(10 ** ecosystem.tokenBDecimals)
      )
    );
    fundTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    fundTx.sign(wallet.payer);
    await banksClient.processTransaction(fundTx);

    let setupTx = new Transaction().add(
      await setupEmissions(groupAdmin.mrgnBankrunProgram, {
        bank: validators[0].bank,
        emissionsMint: ecosystem.tokenBMint.publicKey,
        fundingAccount: groupAdmin.tokenBAccount,
        // Note: borrow emissions do nothing for staked collateral
        emissionsFlags: new BN(
          EMISSIONS_FLAG_BORROW_ACTIVE + EMISSIONS_FLAG_LENDING_ACTIVE
        ),
        emissionsRate: emissionRate,
        totalEmissions: totalEmissions,
      })
    );
    setupTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    setupTx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(setupTx);

    // Find all the validator 0 positions so we know how much each user is owed...
    for (let i = 0; i < numUsers; i++) {
      const userAccount = users[i].accounts.get(USER_ACCOUNT);

      const acc = await bankrunProgram.account.marginfiAccount.fetch(
        userAccount
      );
      const balances = acc.lendingAccount.balances;
      let foundBalance = false;
      for (let i = 0; i < balances.length; i++) {
        if (balances[i].bankPk.equals(validators[0].bank)) {
          const shares = wrappedI80F48toBigNumber(
            balances[i].assetShares
          ).toNumber();
          if (verbose) {
            console.log("user [" + i + "] shares: " + shares);
          }
          userDeposits.push(shares);
          foundBalance = true;
        }
      }

      // If the search loop above fails, then that user doesn't have any val 0 balance
      if (!foundBalance) {
        userDeposits.push(0);
        if (verbose) {
          console.log("user [" + i + "] has no validator 0 bank position");
        }
      }
    }

    const bankAcc = await bankrunProgram.account.bank.fetch(validators[0].bank);
    netDeposits = wrappedI80F48toBigNumber(bankAcc.totalAssetShares).toNumber();
    if (verbose) {
      console.log("net shares oustanding: " + netDeposits);
      console.log("");
    }
  });

  it("(user 2) claims emissions when no time elapsed - nothing happens", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    const tokenBefore = await getTokenBalance(
      bankRunProvider,
      users[2].tokenBAccount
    );

    let tx = new Transaction().add(
      await settleEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
      }),
      await withdrawEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: users[2].tokenBAccount,
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const tokenAfter = await getTokenBalance(
      bankRunProvider,
      users[2].tokenBAccount
    );
    const diff = tokenAfter - tokenBefore;
    assert.equal(diff, 0);

    if (verbose) {
      console.log("User 2 claimed token B emissions");
      console.log(
        "before: " + tokenBefore + "  after: " + tokenAfter + "  diff " + diff
      );
    }
  });

  it("time elapses", async () => {
    let { epoch } = await getEpochAndSlot(banksClient);
    const warpTo = BigInt(epoch + 5);
    bankrunContext.warpToEpoch(warpTo);
    if (verbose) {
      console.log("Warped to epoch: " + warpTo);
    }
  });

  let user2Claim: number;
  it("(user 2) claims again after some time - happy path", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    const tokenBefore = await getTokenBalance(
      bankRunProvider,
      user.tokenBAccount
    );

    let tx = new Transaction().add(
      await settleEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
      }),
      await withdrawEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: user.tokenBAccount,
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const tokenAfter = await getTokenBalance(
      bankRunProvider,
      user.tokenBAccount
    );
    const diff = tokenAfter - tokenBefore;
    user2Claim = diff;
    assert.isAtLeast(diff, 100); // assures the gain is non-zero

    const expectedShare = (userDeposits[2] / netDeposits) * 100;

    if (verbose) {
      console.log("User 2 claimed token B emissions");
      console.log(
        "User expected share of emissions: " + expectedShare.toFixed(2) + "%"
      );
      console.log(
        "before: " + tokenBefore + "  after: " + tokenAfter + "  diff " + diff
      );
    }
  });

  it("(user 1) claims at the same time - gets proportional fair share", async () => {
    const user = users[1];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    const tokenBefore = await getTokenBalance(
      bankRunProvider,
      user.tokenBAccount
    );

    let tx = new Transaction().add(
      await settleEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
      }),
      await withdrawEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: user.tokenBAccount,
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const tokenAfter = await getTokenBalance(
      bankRunProvider,
      user.tokenBAccount
    );
    const diff = tokenAfter - tokenBefore;
    const netClaimed = user2Claim + diff;
    const claimedShareActual = (diff / netClaimed) * 100;
    assert.isAtLeast(diff, 100);

    // This is true with 2 users....
    const expectedUser1Claim = user2Claim * (userDeposits[1] / userDeposits[2]);
    // User 1 gets ~
    assert.approximately(
      diff,
      expectedUser1Claim,
      expectedUser1Claim * 0.01,
      "User 1's claim is not the expected proportion of user 2's claim"
    );

    const expectedShare = (userDeposits[1] / netDeposits) * 100;

    if (verbose) {
      console.log("User 1 claimed token B emissions");
      console.log(
        "User expected share of emissions: " + expectedShare.toFixed(2) + "%"
      );
      console.log(
        "before: " + tokenBefore + "  after: " + tokenAfter + "  diff " + diff
      );
      console.log("actual claim share: " + claimedShareActual.toFixed(2) + "%");
    }
  });

  it("(user 2) settle is always permissionless (does nothing here, no time elapsed)", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    let tx = new Transaction().add(
      await settleEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(wallet.payer); // Provider wallet has to sign to pay tx fees
    await banksClient.processTransaction(tx);
  });

  // TODO explain why this happens in more detail:

  // Note: You may assume the claim amount should be the same the second time around, since the same
  // number of epochs have elapsed, but that's not typically the case (timing issues, interest
  // growth, etc)
  it("time elapses", async () => {
    let { epoch } = await getEpochAndSlot(banksClient);
    const warpTo = BigInt(epoch + 5);
    bankrunContext.warpToEpoch(warpTo);
    if (verbose) {
      console.log("Warped to epoch: " + warpTo);
    }
  });

  it("(user 2) sets up a wallet to accept permissionless emission claims", async () => {
    // Note that the payer wallet pays here, just to get some SOL into this wallet for rent since
    // this is what most users would do
    let tx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: wallet.publicKey,
        toPubkey: externalWallet.publicKey,
        lamports: 0.1 * LAMPORTS_PER_SOL,
      }),
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        bAta,
        externalWallet.publicKey,
        ecosystem.tokenBMint.publicKey
      )
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(wallet.payer);
    await banksClient.processTransaction(tx);
  });

  it("(user 2) permissionless withdraw before opt-in - fails", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    let tx = new Transaction().add(
      await settleEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
      }),
      await withdrawEmissionsPermissionlessIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: bAta,
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(tx);

    // InvalidEmissionsDestinationAccount 6063
    assertBankrunTxFailed(result, "0x17af");
  });

  it("(user 2) registers permissionless settle to some wallet - (happy path)", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    const accBefore = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    assertKeyDefault(accBefore.emissionsDestinationAccount);

    let tx = new Transaction().add(
      await updateEmissionsDestination(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        destinationAccount: externalWallet.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const accAfter = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    assertKeysEqual(
      accAfter.emissionsDestinationAccount,
      externalWallet.publicKey
    );
  });

  let user2PermissionlessClaim: number;
  it("(user 2) permissionless withdraw after opt-in - happy path", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    // Note: tokens are now going to the B ATA of the external wallet they picked
    const tokenBefore = await getTokenBalance(bankRunProvider, bAta);

    let tx = new Transaction().add(
      await settleEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
      }),
      await withdrawEmissionsPermissionlessIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: bAta,
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(wallet.payer); // Note: not the user!
    await banksClient.processTransaction(tx);

    const tokenAfter = await getTokenBalance(bankRunProvider, bAta);
    const diff = tokenAfter - tokenBefore;
    user2PermissionlessClaim = diff;

    if (verbose) {
      console.log("User 2 claimed token B emissions permissionlessly");
      console.log(
        "before: " + tokenBefore + "  after: " + tokenAfter + "  diff " + diff
      );
    }
  });

  it("(user 1) can still claim with permission - still gets proportional fair share", async () => {
    const user = users[1];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    const tokenBefore = await getTokenBalance(
      bankRunProvider,
      user.tokenBAccount
    );

    let tx = new Transaction().add(
      await settleEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
      }),
      await withdrawEmissionsIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: user.tokenBAccount,
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const tokenAfter = await getTokenBalance(
      bankRunProvider,
      user.tokenBAccount
    );
    const diff = tokenAfter - tokenBefore;
    const netClaimed = user2PermissionlessClaim + diff;
    const claimedShareActual = (diff / netClaimed) * 100;
    assert.isAtLeast(diff, 100);

    const expectedShare = (userDeposits[1] / netDeposits) * 100;

    if (verbose) {
      console.log("User 1 claimed token B emissions");
      console.log(
        "User expected share of emissions: " + expectedShare.toFixed(2) + "%"
      );
      console.log(
        "before: " + tokenBefore + "  after: " + tokenAfter + "  diff " + diff
      );
      console.log("actual claim share: " + claimedShareActual.toFixed(2) + "%");
    }

    const expectedUser1Claim =
      user2PermissionlessClaim * (userDeposits[1] / userDeposits[2]);
    // User 1 gets ~
    assert.approximately(
      diff,
      expectedUser1Claim,
      expectedUser1Claim * 0.01,
      "User 1's claim is not the expected proportion of user 2's claim"
    );
  });
});
