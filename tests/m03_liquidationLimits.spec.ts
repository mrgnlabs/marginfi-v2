import { BN } from "@coral-xyz/anchor";
import { ComputeBudgetProgram, PublicKey, Transaction } from "@solana/web3.js";
import {
  groupAdmin,
  bankrunContext,
  banksClient,
  bankrunProgram,
  ecosystem,
  oracles,
  users,
  globalProgramAdmin,
  klendBankrunProgram,
  MARKET,
  TOKEN_A_RESERVE,
  kaminoAccounts,
  farmAccounts,
  A_FARM_STATE,
  FARMS_PROGRAM_ID,
  driftAccounts,
  DRIFT_TOKEN_A_PULL_ORACLE,
  DRIFT_TOKEN_A_SPOT_MARKET,
} from "./rootHooks";
import { configureBank, setFixedPrice } from "./utils/group-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { assert } from "chai";
import { defaultBankConfigOptRaw } from "./utils/types";
import {
  borrowIx,
  composeRemainingAccounts,
  depositIx,
  liquidateIx,
  withdrawIx,
} from "./utils/user-instructions";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { dumpAccBalances, processBankrunTransaction } from "./utils/tools";
import { genericMultiBankTestSetup } from "./genericSetups";
import { refreshPullOracles } from "./utils/pyth-pull-mocks";
import { assertI80F48Approx, assertKeyDefault } from "./utils/genericTests";
import {
  simpleRefreshObligation,
  simpleRefreshReserve,
} from "./utils/kamino-utils";
import { makeKaminoDepositIx } from "./utils/kamino-instructions";
import { makeDriftDepositIx } from "./utils/drift-instructions";
import { TOKEN_A_MARKET_INDEX } from "./utils/drift-utils";
import {
  deriveBaseObligation,
  deriveLiquidityVaultAuthority,
} from "./utils/pdas";

const startingSeed: number = 42;
const groupBuff = Buffer.from("MARGINFI_GROUP_SEED_1234000000M3");

/** This is the program-enforced maximum enforced number of balances per account. */
const MAX_BALANCES = 16;
const KAMINO_DEPOSITS = 8;
const DRIFT_DEPOSITS = 7;
const P0_BORROWS = MAX_BALANCES - KAMINO_DEPOSITS - DRIFT_DEPOSITS; // = 1
const USER_ACCOUNT_THROWAWAY = "throwaway_account2";

let banks: PublicKey[] = [];
let kaminoBanks: PublicKey[] = [];
let driftBanks: PublicKey[] = [];
let lendingMarket: PublicKey;
let reserveFarmState: PublicKey;
let tokenAReserve: PublicKey;
let liquidateeRemainingAccounts: PublicKey[] = [];
let liquidatorRemainingAccounts: PublicKey[] = [];
let driftSpotMarket: PublicKey;

describe("m03: Limits on number of accounts when using Kamino and Drift", () => {
  it("init group, init banks, and fund banks", async () => {
    const result = await genericMultiBankTestSetup(
      P0_BORROWS,
      USER_ACCOUNT_THROWAWAY,
      groupBuff,
      startingSeed,
      KAMINO_DEPOSITS,
      DRIFT_DEPOSITS,
    );
    banks = result.banks;
    kaminoBanks = result.kaminoBanks;
    driftBanks = result.driftBanks;
    lendingMarket = kaminoAccounts.get(MARKET);
    tokenAReserve = kaminoAccounts.get(TOKEN_A_RESERVE);
    reserveFarmState = farmAccounts.get(A_FARM_STATE);
    driftSpotMarket = driftAccounts.get(DRIFT_TOKEN_A_SPOT_MARKET);
  });

  it("Refresh oracles", async () => {
    let clock = await banksClient.getClock();
    await refreshPullOracles(
      oracles,
      globalProgramAdmin.wallet,
      new BN(Number(clock.slot)),
      Number(clock.unixTimestamp),
      bankrunContext,
      false,
    );
  });

  it("(admin) Seeds liquidity in all banks - happy path", async () => {
    const user = groupAdmin;
    const marginfiAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const depositLstAmount = new BN(10 * 10 ** ecosystem.lstAlphaDecimals);
    const depositTokenAAmount = new BN(100 * 10 ** ecosystem.tokenADecimals);

    const remainingAccounts: PublicKey[][] = [];
    for (let i = 0; i < banks.length; i += 1) {
      const bank = banks[i];
      const tx = new Transaction();
      tx.add(
        await depositIx(user.mrgnBankrunProgram, {
          marginfiAccount,
          bank,
          tokenAccount: user.lstAlphaAccount,
          amount: depositLstAmount,
          depositUpToLimit: false,
        }),
      );
      await processBankrunTransaction(bankrunContext, tx, [user.wallet]);
      remainingAccounts.push([bank, oracles.pythPullLst.publicKey]);
    }

    for (let i = 0; i < kaminoBanks.length; i += 1) {
      const bank = kaminoBanks[i];
      const tx = new Transaction();
      const [lendingVaultAuthority] = deriveLiquidityVaultAuthority(
        bankrunProgram.programId,
        bank,
      );
      const [obligation] = deriveBaseObligation(
        lendingVaultAuthority,
        lendingMarket,
      );
      const [obligationFarmUserState] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("user"),
          reserveFarmState.toBuffer(),
          obligation.toBuffer(),
        ],
        FARMS_PROGRAM_ID,
      );

      tx.add(
        await simpleRefreshReserve(
          klendBankrunProgram,
          tokenAReserve,
          lendingMarket,
          oracles.tokenAOracle.publicKey,
        ),
        await simpleRefreshObligation(
          klendBankrunProgram,
          lendingMarket,
          obligation,
          [tokenAReserve],
        ),
        await makeKaminoDepositIx(
          user.mrgnBankrunProgram,
          {
            marginfiAccount,
            bank,
            signerTokenAccount: user.tokenAAccount,
            lendingMarket,
            reserveLiquidityMint: ecosystem.tokenAMint.publicKey,
            obligationFarmUserState,
            reserveFarmState,
          },
          depositTokenAAmount,
        ),
      );
      await processBankrunTransaction(bankrunContext, tx, [user.wallet]);
      remainingAccounts.push([
        bank,
        oracles.tokenAOracle.publicKey,
        tokenAReserve,
      ]);
    }

    for (let i = 0; i < driftBanks.length; i += 1) {
      const bank = driftBanks[i];
      const tx = new Transaction();
      tx.add(
        await makeDriftDepositIx(
          user.mrgnBankrunProgram,
          {
            marginfiAccount,
            bank,
            signerTokenAccount: user.tokenAAccount,
            driftOracle: driftAccounts.get(DRIFT_TOKEN_A_PULL_ORACLE),
          },
          depositTokenAAmount,
          TOKEN_A_MARKET_INDEX,
        ),
      );
      await processBankrunTransaction(
        bankrunContext,
        tx,
        [user.wallet],
        false,
        true,
      );
      remainingAccounts.push([
        bank,
        oracles.tokenAOracle.publicKey,
        driftSpotMarket,
      ]);
    }

    liquidatorRemainingAccounts = composeRemainingAccounts(remainingAccounts);
  });

  it("(user 0) Deposits to all Kamino and Drift banks and borrows from a regular one - happy path", async () => {
    const user = users[0];
    const marginfiAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const depositTokenAAmount = new BN(10 * 10 ** ecosystem.tokenADecimals);
    const borrowLstAmount = new BN(1 * 10 ** ecosystem.lstAlphaDecimals);

    const remainingAccounts: PublicKey[][] = [];
    for (let i = 0; i < kaminoBanks.length; i += 1) {
      const bank = kaminoBanks[i];
      const tx = new Transaction();
      const [lendingVaultAuthority] = deriveLiquidityVaultAuthority(
        bankrunProgram.programId,
        bank,
      );
      const [obligation] = deriveBaseObligation(
        lendingVaultAuthority,
        lendingMarket,
      );
      const [obligationFarmUserState] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("user"),
          reserveFarmState.toBuffer(),
          obligation.toBuffer(),
        ],
        FARMS_PROGRAM_ID,
      );

      tx.add(
        await simpleRefreshReserve(
          klendBankrunProgram,
          tokenAReserve,
          lendingMarket,
          oracles.tokenAOracle.publicKey,
        ),
        await simpleRefreshObligation(
          klendBankrunProgram,
          lendingMarket,
          obligation,
          [tokenAReserve],
        ),
        await makeKaminoDepositIx(
          user.mrgnBankrunProgram,
          {
            marginfiAccount,
            bank,
            signerTokenAccount: user.tokenAAccount,
            lendingMarket,
            reserveLiquidityMint: ecosystem.tokenAMint.publicKey,
            obligationFarmUserState,
            reserveFarmState,
          },
          depositTokenAAmount,
        ),
      );
      remainingAccounts.push([
        bank,
        oracles.tokenAOracle.publicKey,
        tokenAReserve,
      ]);
      await processBankrunTransaction(bankrunContext, tx, [user.wallet]);
    }

    for (let i = 0; i < driftBanks.length; i += 1) {
      const bank = driftBanks[i];
      const tx = new Transaction();
      tx.add(
        await makeDriftDepositIx(
          user.mrgnBankrunProgram,
          {
            marginfiAccount,
            bank,
            signerTokenAccount: user.tokenAAccount,
            driftOracle: driftAccounts.get(DRIFT_TOKEN_A_PULL_ORACLE),
          },
          depositTokenAAmount,
          TOKEN_A_MARKET_INDEX,
        ),
      );
      remainingAccounts.push([
        bank,
        oracles.tokenAOracle.publicKey,
        driftSpotMarket,
      ]);
      await processBankrunTransaction(bankrunContext, tx, [user.wallet]);
    }

    remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
    liquidateeRemainingAccounts = composeRemainingAccounts(remainingAccounts);

    const tx = new Transaction();
    tx.add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }),
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount,
        bank: banks[0], // there is only one regular bank
        tokenAccount: user.lstAlphaAccount,
        remaining: liquidateeRemainingAccounts,
        amount: borrowLstAmount,
      }),
    );
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      false,
      true,
    );
  });

  it("(admin) Vastly increases regular bank liability ratio to make user 0 unhealthy", async () => {
    let config = defaultBankConfigOptRaw();
    config.liabilityWeightInit = bigNumberToWrappedI80F48(210); // 21000%
    config.liabilityWeightMaint = bigNumberToWrappedI80F48(200); // 20000%

    let tx = new Transaction().add(
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: banks[0],
        bankConfigOpt: config,
      }),
    );
    await processBankrunTransaction(bankrunContext, tx, [groupAdmin.wallet]);
  });

  it("(admin) Liquidates user 0", async () => {
    const liquidatee = users[0];
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_THROWAWAY);
    const liquidator = groupAdmin;
    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT_THROWAWAY);
    const liquidateAmount = new BN(0.1 * 10 ** ecosystem.lstAlphaDecimals);

    const liquidateeAcc = await bankrunProgram.account.marginfiAccount.fetch(
      liquidateeAccount,
    );
    dumpAccBalances(liquidateeAcc);
    const liquidatorAcc = await bankrunProgram.account.marginfiAccount.fetch(
      liquidatorAccount,
    );
    dumpAccBalances(liquidatorAcc);

    const kaminoTx = new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      await liquidateIx(liquidator.mrgnBankrunProgram, {
        assetBankKey: kaminoBanks[0],
        liabilityBankKey: banks[0],
        liquidatorMarginfiAccount: liquidatorAccount,
        liquidateeMarginfiAccount: liquidateeAccount,
        remaining: [
          oracles.tokenAOracle.publicKey, // asset oracle
          tokenAReserve, // Kamino-specific "oracle"
          oracles.pythPullLst.publicKey, // liab oracle
          ...liquidatorRemainingAccounts,
          ...liquidateeRemainingAccounts,
        ],
        amount: liquidateAmount,
        liquidateeAccounts: liquidateeRemainingAccounts.length,
        liquidatorAccounts: liquidatorRemainingAccounts.length,
      }),
    );
    await processBankrunTransaction(
      bankrunContext,
      kaminoTx,
      [groupAdmin.wallet]
    );

    const driftTx = new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      await liquidateIx(liquidator.mrgnBankrunProgram, {
        assetBankKey: driftBanks[0],
        liabilityBankKey: banks[0],
        liquidatorMarginfiAccount: liquidatorAccount,
        liquidateeMarginfiAccount: liquidateeAccount,
        remaining: [
          oracles.tokenAOracle.publicKey, // asset oracle
          driftSpotMarket, // Drift-specific "oracle"
          oracles.pythPullLst.publicKey, // liab oracle
          ...liquidatorRemainingAccounts,
          ...liquidateeRemainingAccounts,
        ],
        amount: liquidateAmount,
        liquidateeAccounts: liquidateeRemainingAccounts.length,
        liquidatorAccounts: liquidatorRemainingAccounts.length,
      }),
    );
    await processBankrunTransaction(
      bankrunContext,
      driftTx,
      [groupAdmin.wallet]
    );
  });
  // TODO try these with switchboard oracles.
});
