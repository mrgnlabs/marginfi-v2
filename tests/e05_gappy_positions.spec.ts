import { BN } from "@coral-xyz/anchor";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import {
  bankrunContext,
  bankrunProgram,
  bankRunProvider,
  banksClient,
  ecosystem,
  EMODE_INIT_RATE_LST_TO_LST,
  EMODE_INIT_RATE_SOL_TO_LST,
  EMODE_SEED,
  emodeGroup,
  groupAdmin,
  oracles,
  users,
  verbose,
} from "./rootHooks";
import {
  assertBankrunTxFailed,
  assertI80F48Approx,
} from "./utils/genericTests";
import { CONF_INTERVAL_MULTIPLE } from "./utils/types";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { deriveBankWithSeed } from "./utils/pdas";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { USER_ACCOUNT_E } from "./utils/mocks";
import {
  accountInit,
  borrowIx,
  depositIx,
  repayIx,
  withdrawIx,
} from "./utils/user-instructions";
import { bytesToF64, dumpBankrunLogs } from "./utils/tools";

const seed = new BN(EMODE_SEED);
let usdcBank: PublicKey;
let stableBank: PublicKey;
let solBank: PublicKey;
let lstABank: PublicKey;
let lstBBank: PublicKey;

describe("Gappy position setup", () => {
  before(async () => {
    [usdcBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      seed
    );
    [lstBBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      seed.addn(1)
    );
    [lstABank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      seed
    );
    [solBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.wsolMint.publicKey,
      seed
    );
    [stableBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      seed.addn(1)
    );

    console.log("stable bank: " + stableBank);
    console.log("sol bank: " + solBank);
    console.log("lst a bank: " + lstABank);
    console.log("lst b bank: " + lstBBank);
    console.log("usdc bank: " + usdcBank);
  });

  /*
Create a user like:
  ```
│ (index) │ Bank PK                                        │ Tag │ Liab Shares    │ Asset Shares      │ Emissions │
├─────────┼────────────────────────────────────────────────┼─────┼────────────────┼───────────────────┼───────────┤
│ 0       │ 'AS6TnrAic2NV4eppLbLwyNj6LYzDV9SdBRjZiJmavYSN' │ 0   │ '-'            │ '10000000.0000'   │ '-'       │
│ 1       │ 'empty'                                        │ '-' │ '-'            │ '-'               │ '-'       │
│ 2       │ '7rw9Bbg3R3vQ3LDApqfrYS1qTV3vFpigWJFUmh7eZfwE' │ 0   │ '-'            │ '1000000000.0000' │ '-'       │
│ 3       │ 'empty'                                        │ '-' │ '-'            │ '-'               │ '-'       │
│ 4       │ 'HPAZgpFHTA9Wqx22Gk1C75Wyonxz1L55LJhXom4uZfaL' │ 0   │ '1000000.0000' │ '-'               │ '-'       │

(lend stable/lst A, borrow usdc)
``` 
   */
  it("(user 3) creates an account with gaps to feed to gapless tests", async () => {
    const user = users[3];
    const stableDeposit = 10;
    const solDeposit = 1;
    const lstADeposit = 1;
    const lstBBorrow = 0.25;
    const usdcBorrow = 1;
    const userAccount = user.accounts.get(USER_ACCOUNT_E);

    console.log("wallet: " + user.wallet.publicKey);
    console.log("account: " + userAccount);

    let tx = new Transaction().add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: stableBank,
        tokenAccount: user.usdcAccount,
        amount: new BN(stableDeposit * 10 ** ecosystem.usdcDecimals),
        depositUpToLimit: false,
      }),
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: solBank,
        tokenAccount: user.wsolAccount,
        amount: new BN(solDeposit * 10 ** ecosystem.wsolDecimals),
        depositUpToLimit: false,
      }),
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: lstABank,
        tokenAccount: user.lstAlphaAccount,
        amount: new BN(lstADeposit * 10 ** ecosystem.lstAlphaDecimals),
        depositUpToLimit: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: lstBBank,
        tokenAccount: user.lstAlphaAccount,
        remaining: [
          stableBank,
          oracles.usdcOracle.publicKey,
          solBank,
          oracles.wsolOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
          lstBBank,
          oracles.pythPullLst.publicKey,
        ],
        amount: new BN(lstBBorrow * 10 ** ecosystem.lstAlphaDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: usdcBank,
        tokenAccount: user.usdcAccount,
        remaining: [
          stableBank,
          oracles.usdcOracle.publicKey,
          solBank,
          oracles.wsolOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
          lstBBank,
          oracles.pythPullLst.publicKey,
          usdcBank,
          oracles.usdcOracle.publicKey,
        ],
        amount: new BN(usdcBorrow * 10 ** ecosystem.usdcDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      await repayIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: lstBBank,
        tokenAccount: user.lstAlphaAccount,
        remaining: [],
        amount: new BN(lstBBorrow * 10 ** ecosystem.lstAlphaDecimals),
        repayAll: true,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      await withdrawIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: solBank,
        tokenAccount: user.wsolAccount,
        amount: new BN(solDeposit * 10 ** ecosystem.wsolDecimals),
        remaining: [
          stableBank,
          oracles.usdcOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
          usdcBank,
          oracles.usdcOracle.publicKey,
        ],
        withdrawAll: true,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    dumpAccBalances(userAccount);

    let acc = await bankRunProvider.connection.getAccountInfo(userAccount);
    const b64 = acc.data.toString("base64");
    console.log(
      JSON.stringify(
        {
          pubkey: userAccount.toBase58(),
          account: {
            lamports: acc.lamports,
            data: [b64, "base64"],
            owner: acc.owner.toBase58(),
            executable: acc.executable,
            rentEpoch: acc.rentEpoch,
            space: acc.data.length,
          },
        },
        null,
        2
      )
    );
  });

  /*
Create a user like:
  ```
│ (index) │ Bank PK                                        │ Tag │ Liab Shares      │ Asset Shares      │ Emissions │
├─────────┼────────────────────────────────────────────────┼─────┼──────────────────┼───────────────────┼───────────┤
│ 0       │ '2HDFUn7Ug3Ro2JJ4Kn1tYKuLybRdiHXDCEZ2xe7c4ify' │ 1   │ '-'              │ '1000000000.0000' │ '-'       │
│ 1       │ '7rw9Bbg3R3vQ3LDApqfrYS1qTV3vFpigWJFUmh7eZfwE' │ 0   │ '-'              │ '1000000000.0000' │ '-'       │
│ 2       │ 'empty'                                        │ '-' │ '-'              │ '-'               │ '-'       │
│ 3       │ 'empty'                                        │ '-' │ '-'              │ '-'               │ '-'       │
│ 4       │ 'Ga2Vuevw9mTjEf8wchQgYfXfWB7KqGbKq6sKombnCGPx' │ 0   │ '250000000.0000' │ '-'               │ '-'       │

(lend sol/lst A, borrow lst b)
``` 
   */
  it("(user 4) creates an account with gaps to feed to gapless tests", async () => {
    const user = users[4];
    const stableDeposit = 10;
    const solDeposit = 1;
    const lstADeposit = 1;
    const lstBBorrow = 0.25;
    const usdcBorrow = 1;
    const userAccount = user.accounts.get(USER_ACCOUNT_E);
    console.log("wallet: " + user.wallet.publicKey);
    console.log("account: " + userAccount);

    let tx = new Transaction().add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: solBank,
        tokenAccount: user.wsolAccount,
        amount: new BN(solDeposit * 10 ** ecosystem.wsolDecimals),
        depositUpToLimit: false,
      }),
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: lstABank,
        tokenAccount: user.lstAlphaAccount,
        amount: new BN(lstADeposit * 10 ** ecosystem.lstAlphaDecimals),
        depositUpToLimit: false,
      }),
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: stableBank,
        tokenAccount: user.usdcAccount,
        amount: new BN(stableDeposit * 10 ** ecosystem.usdcDecimals),
        depositUpToLimit: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: usdcBank,
        tokenAccount: user.usdcAccount,
        remaining: [
          solBank,
          oracles.wsolOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
          stableBank,
          oracles.usdcOracle.publicKey,
          usdcBank,
          oracles.usdcOracle.publicKey,
        ],
        amount: new BN(usdcBorrow * 10 ** ecosystem.usdcDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: lstBBank,
        tokenAccount: user.lstAlphaAccount,
        remaining: [
          solBank,
          oracles.wsolOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
          stableBank,
          oracles.usdcOracle.publicKey,
          usdcBank,
          oracles.usdcOracle.publicKey,
          lstBBank,
          oracles.pythPullLst.publicKey,
        ],
        amount: new BN(lstBBorrow * 10 ** ecosystem.lstAlphaDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      await repayIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: usdcBank,
        tokenAccount: user.usdcAccount,
        remaining: [],
        amount: new BN(usdcBorrow * 10 ** ecosystem.usdcDecimals),
        repayAll: true,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      await withdrawIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: stableBank,
        tokenAccount: user.usdcAccount,
        amount: new BN(stableDeposit * 10 ** ecosystem.usdcDecimals),
        remaining: [
          solBank,
          oracles.wsolOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
          lstBBank,
          oracles.pythPullLst.publicKey,
        ],
        withdrawAll: true,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    dumpAccBalances(userAccount);

    let acc = await bankRunProvider.connection.getAccountInfo(userAccount);
    const b64 = acc.data.toString("base64");
    console.log(
      JSON.stringify(
        {
          pubkey: userAccount.toBase58(),
          account: {
            lamports: acc.lamports,
            data: [b64, "base64"],
            owner: acc.owner.toBase58(),
            executable: acc.executable,
            rentEpoch: acc.rentEpoch,
            space: acc.data.length,
          },
        },
        null,
        2
      )
    );
  });

  async function dumpAccBalances(key: PublicKey) {
    let userAcc = await bankrunProgram.account.marginfiAccount.fetch(key);
    let balances = userAcc.lendingAccount.balances;
    let activeBalances = [];
    for (let i = 0; i < balances.length; i++) {
      if (balances[i].active == 0) {
        activeBalances.push({
          "Bank PK": "empty",
          Tag: "-",
          "Liab Shares ": "-",
          "Asset Shares": "-",
          Emissions: "-",
        });
        continue;
      }

      activeBalances.push({
        "Bank PK": balances[i].bankPk.toString(),
        Tag: balances[i].bankAssetTag,
        "Liab Shares ": formatNumber(
          wrappedI80F48toBigNumber(balances[i].liabilityShares)
        ),
        "Asset Shares": formatNumber(
          wrappedI80F48toBigNumber(balances[i].assetShares)
        ),
        Emissions: formatNumber(
          wrappedI80F48toBigNumber(balances[i].emissionsOutstanding)
        ),
      });

      function formatNumber(num) {
        const number = parseFloat(num).toFixed(4);
        return number === "0.0000" ? "-" : number;
      }
    }
    console.table(activeBalances);
  }
});
