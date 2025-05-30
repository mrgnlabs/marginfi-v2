import { getProvider, AnchorProvider, Wallet } from "@coral-xyz/anchor";
import { createMintToInstruction } from "@solana/spl-token";
import { PublicKey, Keypair, Transaction, AccountMeta } from "@solana/web3.js";
import BN from "bn.js";
import {
  groupAdmin,
  bankrunContext,
  banksClient,
  verbose,
  ecosystem,
  oracles,
  bankrunProgram,
  users,
} from "./rootHooks";
import { addBankWithSeed, groupInitialize } from "./utils/group-instructions";
import { deriveBankWithSeed } from "./utils/pdas";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { accountInit } from "./utils/user-instructions";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import {
  defaultBankConfig,
  I80F48_ZERO,
  ORACLE_SETUP_PYTH_PUSH,
} from "./utils/types";

/**
 * Initialize a group, create N banks, fund everyone, and init accounts.
 *
 * @param numberOfBanks - how many banks to add (≤ MAX_BALANCES)
 * @returns the three globals you’ll want in your tests
 */
export const genericMultiBankTestSetup = async (
  numberOfBanks: number,
  userAccountName: string
): Promise<{
  startingSeed: number;
  banks: PublicKey[];
  throwawayGroup: Keypair;
}> => {
  /** Banks in this test setup use a "random" seed so their key is non-deterministic. */
  let startingSeed: number = Math.floor(Math.random() * 16000);

  const USER_ACCOUNT_THROWAWAY = userAccountName;

  let banks: PublicKey[] = [];
  const throwawayGroup = Keypair.generate();

  // 1) init the group
  {
    const tx = new Transaction();
    tx.add(
      await groupInitialize(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: throwawayGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, throwawayGroup);
    await banksClient.processTransaction(tx);

    if (verbose) console.log(`*init group: ${throwawayGroup.publicKey}`);
  }

  // 2) add banks
  {
    const tasks: Promise<any>[] = [];
    for (let i = 0; i < numberOfBanks; i++) {
      const seed = startingSeed + i;
      tasks.push(
        addGenericBank(throwawayGroup, {
          bankMint: ecosystem.lstAlphaMint.publicKey,
          oracle: oracles.pythPullLstOracleFeed.publicKey,
          oracleMeta: {
            pubkey: oracles.pythPullLst.publicKey,
            isSigner: false,
            isWritable: false,
          },
          feedOracle: oracles.pythPullLstOracleFeed.publicKey,
          seed: new BN(seed),
          verboseMessage: verbose ? `*init LST #${seed}:` : undefined,
        })
      );

      const [bankPk] = deriveBankWithSeed(
        bankrunProgram.programId,
        throwawayGroup.publicKey,
        ecosystem.lstAlphaMint.publicKey,
        new BN(seed)
      );
      banks.push(bankPk);
    }
    await Promise.all(tasks);
  }

  // 3) fund users + admin
  {
    const provider = getProvider() as AnchorProvider;
    const wallet = provider.wallet as Wallet;

    for (const u of users) {
      const tx = new Transaction();
      tx.add(
        createMintToInstruction(
          ecosystem.lstAlphaMint.publicKey,
          u.lstAlphaAccount,
          wallet.publicKey,
          10_000 * 10 ** ecosystem.lstAlphaDecimals
        )
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(wallet.payer);
      await banksClient.processTransaction(tx);
    }

    const txAdmin = new Transaction();
    txAdmin.add(
      createMintToInstruction(
        ecosystem.lstAlphaMint.publicKey,
        groupAdmin.lstAlphaAccount,
        wallet.publicKey,
        10_000 * 10 ** ecosystem.lstAlphaDecimals
      )
    );
    txAdmin.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    txAdmin.sign(wallet.payer);
    await banksClient.processTransaction(txAdmin);
  }

  // 4) init user accounts (and admin) if not already init
  {
    for (const [i, u] of users.entries()) {
      if (u.accounts.has(USER_ACCOUNT_THROWAWAY)) {
        if (verbose) console.log(`Skipped creating user account #${i}`);
      } else {
        const kp = Keypair.generate();
        u.accounts.set(USER_ACCOUNT_THROWAWAY, kp.publicKey);
        if (verbose) console.log(`Initialized user #${i}: ${kp.publicKey}`);

        const tx = new Transaction();
        tx.add(
          await accountInit(u.mrgnBankrunProgram, {
            marginfiGroup: throwawayGroup.publicKey,
            marginfiAccount: kp.publicKey,
            authority: u.wallet.publicKey,
            feePayer: u.wallet.publicKey,
          })
        );
        tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
        tx.sign(u.wallet, kp);
        await banksClient.processTransaction(tx);
      }
    }

    if (!groupAdmin.accounts.has(USER_ACCOUNT_THROWAWAY)) {
      const adminKp = Keypair.generate();
      groupAdmin.accounts.set(USER_ACCOUNT_THROWAWAY, adminKp.publicKey);
      if (verbose)
        console.log(`Initialized admin account: ${adminKp.publicKey}`);

      const tx = new Transaction();
      tx.add(
        await accountInit(groupAdmin.mrgnBankrunProgram, {
          marginfiGroup: throwawayGroup.publicKey,
          marginfiAccount: adminKp.publicKey,
          authority: groupAdmin.wallet.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(groupAdmin.wallet, adminKp);
      await banksClient.processTransaction(tx);
    } else if (verbose) {
      console.log("Skipped creating admin account");
    }
  }

  return { startingSeed, banks, throwawayGroup };
};

/**
 * Sets up a generic bank for the provided group.
 * * deposit/borrow limit = 100_000_000_000_000
 * * protocolOriginationFee = 0
 * * all fees = 0
 * * Plataeu rate = 20%
 * * Optimal rate = 80%
 * * asset tag over-rides if provided.
 * * otherwise uses `defaultBankConfig()`
 * @param throwawayGroup
 * @param options
 */
async function addGenericBank(
  throwawayGroup: Keypair,
  options: {
    assetTag?: number;
    bankMint: PublicKey;
    oracle: PublicKey;
    oracleMeta: AccountMeta;
    // Optional feed oracle in case the instruction requires it (i.e. for pull)
    feedOracle?: PublicKey;
    // Function to adjust the seed (for example, seed.addn(1))
    seed: BN;
    verboseMessage: string;
  }
) {
  const {
    assetTag,
    bankMint,
    oracle,
    oracleMeta,
    feedOracle,
    seed,
    verboseMessage,
  } = options;

  const config = defaultBankConfig();
  config.assetWeightInit = bigNumberToWrappedI80F48(0.5);
  config.assetWeightMaint = bigNumberToWrappedI80F48(0.6);

  // The default limit is somewhat small for SOL/LST with 9 decimals, so we bump it here.
  config.depositLimit = new BN(100_000_000_000_000);
  config.borrowLimit = new BN(100_000_000_000_000);
  // We don't want origination fees messing with debt here
  config.interestRateConfig.protocolOriginationFee = I80F48_ZERO;
  config.interestRateConfig.plateauInterestRate = bigNumberToWrappedI80F48(0.2);
  config.interestRateConfig.optimalUtilizationRate =
    bigNumberToWrappedI80F48(0.8);
  if (assetTag) {
    config.assetTag = assetTag;
  }

  // Calculate bank key using the (optionally modified) seed
  const [bankKey] = deriveBankWithSeed(
    bankrunProgram.programId,
    throwawayGroup.publicKey,
    bankMint,
    seed
  );

  const setupType = ORACLE_SETUP_PYTH_PUSH;
  const targetOracle = feedOracle ?? oracle;
  const config_ix = await groupAdmin.mrgnProgram.methods
    .lendingPoolConfigureBankOracle(setupType, targetOracle)
    .accountsPartial({
      group: throwawayGroup.publicKey,
      bank: bankKey,
      admin: groupAdmin.wallet.publicKey,
    })
    .remainingAccounts([oracleMeta])
    .instruction();

  const addBankIx = await addBankWithSeed(groupAdmin.mrgnBankrunProgram, {
    marginfiGroup: throwawayGroup.publicKey,
    feePayer: groupAdmin.wallet.publicKey,
    bankMint: bankMint,
    bank: bankKey,
    config: config,
    seed,
  });

  const tx = new Transaction();
  tx.add(addBankIx, config_ix);
  tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
  tx.sign(groupAdmin.wallet);
  await banksClient.processTransaction(tx);

  if (verbose) {
    console.log(`${verboseMessage} ${bankKey}`);
  }
}
