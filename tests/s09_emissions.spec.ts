/**
 * The "point" of this test is to additional test emissions with Bankrun time warps, but it also
 * serves a secondary purpose of validating that emissions works with eccentric bank setups like
 * staked collateral.
 */

import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { LAMPORTS_PER_SOL, PublicKey, Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairSol,
  bankKeypairUsdc,
  bankrunContext,
  bankrunProgram,
  bankRunProvider,
  banksClient,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
  users,
  validators,
  verbose,
} from "./rootHooks";
import {
  assertBNApproximately,
  assertI80F48Equal,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import { liquidateIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import {
  bigNumberToWrappedI80F48,
  getMint,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import {
  defaultStakedInterestSettings,
  EMISSIONS_FLAG_BORROW_ACTIVE,
  EMISSIONS_FLAG_LENDING_ACTIVE,
  StakedSettingsEdit,
} from "./utils/types";
import {
  editStakedSettings,
  propagateStakedSettings,
  setupEmissions,
} from "./utils/group-instructions";
import { deriveStakedSettings } from "./utils/pdas";
import { getStakeAccount } from "./utils/stake-utils";
import { createMintToInstruction } from "@solana/spl-token";

describe("Set up emissions on staked collateral assets", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  const emissionRate = new BN(500_000 * 10 ** ecosystem.tokenBDecimals);
  const totalEmissions = new BN(1_000_000 * 10 ** ecosystem.tokenBDecimals);

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
  });

  it("(user 1) liquidates user 2 with staked SOL against their SOL position - succeeds", async () => {
    // TODO
    console.log("hello world");
  });
});

// TODO: 0,1 - should fail
