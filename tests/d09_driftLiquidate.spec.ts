import { BN } from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  ComputeBudgetProgram,
  Keypair,
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import { createMintToInstruction } from "@solana/spl-token";
import {
  ecosystem,
  groupAdmin,
  globalProgramAdmin,
  driftAccounts,
  DRIFT_TOKENA_SPOT_MARKET,
  DRIFT_TOKENA_PULL_ORACLE,
  oracles,
  verbose,
  bankrunContext,
  bankrunProgram,
  driftBankrunProgram,
  users,
  THROWAWAY_GROUP_SEED_D09,
} from "./rootHooks";
import {
  depositIx,
  borrowIx,
  liquidateIx,
  healthPulse,
  composeRemainingAccounts,
} from "./utils/user-instructions";
import {
  processBankrunTransaction as processBankrunTx,
  logHealthCache,
  dumpAccBalances,
} from "./utils/tools";
import { BanksTransactionResultWithMeta } from "solana-bankrun";
import {
  bigNumberToWrappedI80F48,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import { blankBankConfigOptRaw } from "./utils/types";
import { configureBank } from "./utils/group-instructions";
import {
  defaultDriftBankConfig,
  getDriftScalingFactor,
  getDriftUserAccount,
  formatTokenAmount,
  formatDepositAmount,
  getSpotMarketAccount,
  tokenAmountToScaledBalance,
  USDC_MARKET_INDEX,
  TOKEN_A_MARKET_INDEX,
} from "./utils/drift-utils";
import {
  makeAddDriftBankIx,
  makeDriftDepositIx,
  makeInitDriftUserIx,
} from "./utils/drift-instructions";
import { genericMultiBankTestSetup } from "./genericSetups";
import { deriveBankWithSeed } from "./utils/pdas";
import { ProgramTestContext } from "solana-bankrun";

const startingSeed: number = 10;
const USER_ACCOUNT_THROWAWAY_D = "throwaway_account_d";
let ctx: ProgramTestContext;

let banks: PublicKey[] = [];
let throwawayGroup: Keypair;
let driftTokenABank: PublicKey;
let mrgnID: PublicKey;

const seedAmountLst = new BN(50 * 10 ** ecosystem.lstAlphaDecimals);
const depositAmountTokenA = new BN(100 * 10 ** ecosystem.tokenADecimals);
const borrowAmountLst = new BN(1 * 10 ** ecosystem.lstAlphaDecimals);

describe("d09: Drift Liquidation", () => {
  before(async () => {
    ctx = bankrunContext;
    mrgnID = bankrunProgram.programId;
  });

  it("init group, init banks, and fund banks", async () => {
    const result = await genericMultiBankTestSetup(
      2,
      USER_ACCOUNT_THROWAWAY_D,
      THROWAWAY_GROUP_SEED_D09,
      startingSeed
    );
    banks = result.banks;
    throwawayGroup = result.throwawayGroup;

    [driftTokenABank] = deriveBankWithSeed(
      mrgnID,
      throwawayGroup.publicKey,
      ecosystem.tokenAMint.publicKey,
      new BN(startingSeed).addn(1)
    );
  });

  it("(admin) init drift Token A bank", async () => {
    const driftSpotMarket = driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET);

    let defaultConfig = defaultDriftBankConfig(oracles.tokenAOracle.publicKey);

    let tx = new Transaction().add(
      await makeAddDriftBankIx(
        groupAdmin.mrgnBankrunProgram,
        {
          group: throwawayGroup.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: ecosystem.tokenAMint.publicKey,
          driftSpotMarket: driftSpotMarket,
          oracle: oracles.tokenAOracle.publicKey,
        },
        {
          config: defaultConfig,
          seed: new BN(startingSeed).addn(1),
        }
      )
    );
    await processBankrunTx(ctx, tx, [groupAdmin.wallet]);

    const initUserAmount = new BN(100);
    const fundAdminTx = new Transaction().add(
      createMintToInstruction(
        ecosystem.tokenAMint.publicKey,
        groupAdmin.tokenAAccount,
        globalProgramAdmin.wallet.publicKey,
        initUserAmount.toNumber()
      )
    );
    await processBankrunTx(ctx, fundAdminTx, [globalProgramAdmin.wallet]);

    const initUserTx = new Transaction().add(
      await makeInitDriftUserIx(
        groupAdmin.mrgnBankrunProgram,
        {
          feePayer: groupAdmin.wallet.publicKey,
          bank: driftTokenABank,
          signerTokenAccount: groupAdmin.tokenAAccount,
          driftOracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE),
        },
        {
          amount: initUserAmount,
        },
        1
      )
    );
    await processBankrunTx(ctx, initUserTx, [groupAdmin.wallet]);
  });

  it("(admin) Seeds liquidity in all regular banks", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY_D);
    const depositsPerTx = 5;

    for (let i = 0; i < banks.length; i += depositsPerTx) {
      const chunk = banks.slice(i, i + depositsPerTx);
      const tx = new Transaction();
      let k = 0;
      for (const bank of chunk) {
        tx.add(
          await depositIx(user.mrgnBankrunProgram, {
            marginfiAccount: userAccount,
            bank,
            tokenAccount: user.lstAlphaAccount,
            amount: seedAmountLst,
            depositUpToLimit: false,
          })
        );
        k++;
      }
      await processBankrunTx(ctx, tx, [user.wallet]);
    }
  });

  it("(user 0) deposits Token A into Drift, borrows LST from bank [0]", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY_D);
    const driftSpotMarket = driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET);

    if (verbose) {
      console.log(
        `\n💰 Depositing ${formatTokenAmount(
          depositAmountTokenA,
          ecosystem.tokenADecimals,
          "Token A"
        )} through Drift`
      );
    }
    const bank = await bankrunProgram.account.bank.fetch(driftTokenABank);
    const driftUserBefore = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );
    const spotPositionBefore = driftUserBefore.spotPositions[0];
    const scaledBalanceBefore = new BN(
      spotPositionBefore.scaledBalance.toString()
    );

    let tx = new Transaction().add(
      await makeDriftDepositIx(
        user.mrgnBankrunProgram,
        {
          marginfiAccount: userAccount,
          bank: driftTokenABank,
          signerTokenAccount: user.tokenAAccount,
          driftOracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE),
        },
        depositAmountTokenA,
        TOKEN_A_MARKET_INDEX
      ),
      await healthPulse(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        remaining: composeRemainingAccounts([
          [driftTokenABank, oracles.tokenAOracle.publicKey, driftSpotMarket],
        ]),
      })
    );
    await processBankrunTx(ctx, tx, [user.wallet]);

    const driftUserAfter = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );
    const spotPositionAfter = driftUserAfter.spotPositions[0];
    const scaledBalanceAfter = new BN(
      spotPositionAfter.scaledBalance.toString()
    );
    const scaledBalanceChange = scaledBalanceAfter.sub(scaledBalanceBefore);

    if (verbose) {
      console.log(
        `   Scaled balance change: ${formatDepositAmount(scaledBalanceChange)}`
      );
    }

    const accBefore = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const cacheBefore = accBefore.healthCache;
    if (verbose) {
      logHealthCache("user 0 cache before borrow", cacheBefore);
    }

    tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[0],
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts([
          [driftTokenABank, oracles.tokenAOracle.publicKey, driftSpotMarket],
          [banks[0], oracles.pythPullLst.publicKey],
        ]),
        amount: borrowAmountLst,
      }),
      await healthPulse(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        remaining: composeRemainingAccounts([
          [driftTokenABank, oracles.tokenAOracle.publicKey, driftSpotMarket],
          [banks[0], oracles.pythPullLst.publicKey],
        ]),
      })
    );
    await processBankrunTx(ctx, tx, [user.wallet]);

    const accAfter = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const cacheAfter = accAfter.healthCache;
    if (verbose) {
      logHealthCache("user 0 cache after borrow", cacheAfter);
    }
  });

  it("(admin) increase bank [0] liability ratio to make user 0 just unhealthy", async () => {
    let config = blankBankConfigOptRaw();
    config.liabilityWeightInit = bigNumberToWrappedI80F48(6.0); // 600%
    config.liabilityWeightMaint = bigNumberToWrappedI80F48(5.5); // 550%

    let tx = new Transaction().add(
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: banks[0],
        bankConfigOpt: config,
      })
    );
    await processBankrunTx(ctx, tx, [groupAdmin.wallet]);

    const liquidatee = users[0];
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_THROWAWAY_D);
    const driftSpotMarket = driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET);
    tx = new Transaction().add(
      await healthPulse(liquidatee.mrgnBankrunProgram, {
        marginfiAccount: liquidateeAccount,
        remaining: composeRemainingAccounts([
          [driftTokenABank, oracles.tokenAOracle.publicKey, driftSpotMarket],
          [banks[0], oracles.pythPullLst.publicKey],
        ]),
      })
    );
    await processBankrunTx(ctx, tx, [liquidatee.wallet]);

    const accAfter = await bankrunProgram.account.marginfiAccount.fetch(
      liquidateeAccount
    );
    const cacheAfter = accAfter.healthCache;
    if (verbose) {
      logHealthCache("user 0 cache after liability rate raised ", cacheAfter);

      const assetValue = wrappedI80F48toBigNumber(cacheAfter.assetValueMaint);
      const liabilityValue = wrappedI80F48toBigNumber(
        cacheAfter.liabilityValueMaint
      );
      const health =
        parseFloat(assetValue.toString()) -
        parseFloat(liabilityValue.toString());
      console.log(
        `   Health calculation: ${assetValue} - ${liabilityValue} = ${health.toFixed(
          2
        )}`
      );
      console.log(
        `   Account is ${
          health > 0 ? "HEALTHY" : "UNHEALTHY"
        } (health: ${health.toFixed(2)})`
      );
    }
  });

  it("(user 1) Liquidates user 0", async () => {
    const liquidatee = users[0];
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_THROWAWAY_D);
    const liquidator = users[1];
    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT_THROWAWAY_D);
    const driftSpotMarket = driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET);
    const depositAmount = new BN(1 * 10 ** ecosystem.lstAlphaDecimals);

    const spotMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );
    let tx = new Transaction().add(
      await depositIx(liquidator.mrgnBankrunProgram, {
        marginfiAccount: liquidatorAccount,
        bank: banks[0],
        tokenAccount: liquidator.lstAlphaAccount,
        amount: depositAmount,
        depositUpToLimit: false,
      })
    );
    await processBankrunTx(ctx, tx, [liquidator.wallet]);

    let totalLiquidated = new BN(0);
    let liquidationCount = 0;
    const smallLiquidationAmount = new BN(
      0.01 * 10 ** ecosystem.tokenADecimals
    );

    while (true) {
      const liquidateAmount = tokenAmountToScaledBalance(
        smallLiquidationAmount,
        spotMarket
      );
      const liquidateTx = new Transaction().add(
        ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
        await liquidateIx(liquidator.mrgnBankrunProgram, {
          assetBankKey: driftTokenABank,
          liabilityBankKey: banks[0],
          liquidatorMarginfiAccount: liquidatorAccount,
          liquidateeMarginfiAccount: liquidateeAccount,
          remaining: [
            oracles.tokenAOracle.publicKey, // asset oracle
            driftSpotMarket, // asset spot market
            oracles.pythPullLst.publicKey, // liab oracle

            ...composeRemainingAccounts([
              [banks[0], oracles.pythPullLst.publicKey],
              [
                driftTokenABank,
                oracles.tokenAOracle.publicKey,
                driftSpotMarket,
              ],
            ]),

            ...composeRemainingAccounts([
              [banks[0], oracles.pythPullLst.publicKey],
              [
                driftTokenABank,
                oracles.tokenAOracle.publicKey,
                driftSpotMarket,
              ],
            ]),
          ],
          amount: liquidateAmount,
          liquidateeAccounts: 5,
          liquidatorAccounts: 5,
        })
      );

      const result = (await processBankrunTx(
        ctx,
        liquidateTx,
        [liquidator.wallet],
        true,
        false
      )) as BanksTransactionResultWithMeta;
      if (result.result && result.meta.logMessages) {
        const hasTooSevereError = result.meta.logMessages.some(
          (log) =>
            log.includes("custom program error: 0x17b7") ||
            log.includes("TooSevereLiquidation")
        );

        if (hasTooSevereError) {
          if (liquidationCount === 0) {
            console.error("\n❌ First liquidation attempt failed!");
            console.error("Error logs:");
            result.meta.logMessages.forEach((log) => console.error("  " + log));
            throw new Error(
              "First liquidation failed - check setup and amounts"
            );
          }
          if (verbose) {
            console.log(`\n📊 Liquidation Loop Summary:`);
            console.log(`   Total liquidations: ${liquidationCount}`);
            console.log(
              `   Total amount liquidated: ${formatTokenAmount(
                totalLiquidated,
                ecosystem.tokenADecimals,
                "Token A"
              )}`
            );
          }
          break;
        }
      }

      liquidationCount++;
      totalLiquidated = totalLiquidated.add(smallLiquidationAmount);

      if (liquidationCount >= 50) {
        if (verbose) {
          console.log(
            "\n⚠️  Safety limit reached (50 liquidations) - stopping loop"
          );
        }
        assert(false);
        break;
      }
    }

    const healthTx = new Transaction().add(
      await healthPulse(liquidatee.mrgnBankrunProgram, {
        marginfiAccount: liquidateeAccount,
        remaining: composeRemainingAccounts([
          [driftTokenABank, oracles.tokenAOracle.publicKey, driftSpotMarket],
          [banks[0], oracles.pythPullLst.publicKey],
        ]),
      })
    );
    await processBankrunTx(ctx, healthTx, [liquidatee.wallet]);

    const accAfter = await bankrunProgram.account.marginfiAccount.fetch(
      liquidateeAccount
    );
    const cacheAfter = accAfter.healthCache;
    if (verbose) {
      logHealthCache("user 0 cache post-liquidate ", cacheAfter);
    }

    if (verbose) {
      console.log("\n📊 Final State After All Liquidations:");

      const liquidateeAccAfter =
        await bankrunProgram.account.marginfiAccount.fetch(liquidateeAccount);
      const liquidatorAccAfter =
        await bankrunProgram.account.marginfiAccount.fetch(liquidatorAccount);

      console.log("Liquidatee final balances:");
      dumpAccBalances(liquidateeAccAfter);
      console.log("\nLiquidator final balances:");
      dumpAccBalances(liquidatorAccAfter);

      const driftBank = await bankrunProgram.account.bank.fetch(
        driftTokenABank
      );
      const driftUser = await getDriftUserAccount(
        driftBankrunProgram,
        driftBank.driftUser
      );
      const spotPosition = driftUser.spotPositions[0];
      const scalingFactor = getDriftScalingFactor(ecosystem.tokenADecimals);

      const liquidateeDriftBalance =
        liquidateeAccAfter.lendingAccount.balances.find(
          (b) => b.bankPk.equals(driftTokenABank) && b.active === 1
        );
      const liquidatorDriftBalance =
        liquidatorAccAfter.lendingAccount.balances.find(
          (b) => b.bankPk.equals(driftTokenABank) && b.active === 1
        );

      const preLiquidateeDriftShares = new BN("100000000000");

      if (liquidateeDriftBalance) {
        const assetSharesBigNumber = wrappedI80F48toBigNumber(
          liquidateeDriftBalance.assetShares
        );
        const assetShares = new BN(assetSharesBigNumber.toString());
        const sharesLost = preLiquidateeDriftShares.sub(assetShares);

        const approxTokens = assetShares.div(scalingFactor);
        const tokensLost = sharesLost.div(scalingFactor);
      } else {
        console.log(`   Liquidatee has no active drift balance`);
      }

      if (liquidatorDriftBalance) {
        const assetSharesBigNumber = wrappedI80F48toBigNumber(
          liquidatorDriftBalance.assetShares
        );
        const assetShares = new BN(assetSharesBigNumber.toString());
        const approxTokens = assetShares.div(scalingFactor);
      } else {
        console.log(`   Liquidator has no active drift balance`);
      }

      const liquidateeLstBalance =
        liquidateeAccAfter.lendingAccount.balances.find(
          (b) => b.bankPk.equals(banks[0]) && b.active === 1
        );
      const liquidatorLstBalance =
        liquidatorAccAfter.lendingAccount.balances.find(
          (b) => b.bankPk.equals(banks[0]) && b.active === 1
        );

      if (liquidateeLstBalance) {
        const liabSharesBigNumber = wrappedI80F48toBigNumber(
          liquidateeLstBalance.liabilityShares
        );
        const liabSharesStr = liabSharesBigNumber.toString().split(".")[0];
        const liabShares = new BN(liabSharesStr);
      }
      if (liquidatorLstBalance) {
        const assetSharesBigNumber = wrappedI80F48toBigNumber(
          liquidatorLstBalance.assetShares
        );
        const assetSharesStr = assetSharesBigNumber.toString().split(".")[0];
        const assetShares = new BN(assetSharesStr);
        const liabSharesBigNumber = wrappedI80F48toBigNumber(
          liquidatorLstBalance.liabilityShares
        );
        const liabSharesStr = liabSharesBigNumber.toString().split(".")[0];
        const liabShares = new BN(liabSharesStr);
      }
    }

    const finalLiquidateeAcc =
      await bankrunProgram.account.marginfiAccount.fetch(liquidateeAccount);
    const finalLiquidatorAcc =
      await bankrunProgram.account.marginfiAccount.fetch(liquidatorAccount);

    const liquidateeDriftBal = finalLiquidateeAcc.lendingAccount.balances.find(
      (b) => b.bankPk.equals(driftTokenABank) && b.active === 1
    );
    const liquidatorDriftBal = finalLiquidatorAcc.lendingAccount.balances.find(
      (b) => b.bankPk.equals(driftTokenABank) && b.active === 1
    );

    assert.ok(liquidateeDriftBal);
    const liquidateeShares = new BN(
      wrappedI80F48toBigNumber(liquidateeDriftBal.assetShares).toString()
    );
    assert.ok(liquidateeShares.lt(new BN("100000000000")));

    assert.ok(liquidatorDriftBal);
    const liquidatorShares = new BN(
      wrappedI80F48toBigNumber(liquidatorDriftBal.assetShares).toString()
    );
    assert.ok(liquidatorShares.gt(new BN(0)));

    assert.ok(liquidationCount > 0);

    if (verbose) {
      console.log(`   Performed ${liquidationCount} liquidations`);
      console.log(
        `   Total liquidated: ${formatTokenAmount(
          totalLiquidated,
          ecosystem.tokenADecimals,
          "Token A"
        )}`
      );
    }
  });

  it("(admin) restore bank 0 default liability ratios", async () => {
    let config = blankBankConfigOptRaw();

    let tx = new Transaction().add(
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: banks[0],
        bankConfigOpt: config,
      })
    );
    await processBankrunTx(ctx, tx, [groupAdmin.wallet]);
  });

  // TODO test OOM limits at max accounts
});
