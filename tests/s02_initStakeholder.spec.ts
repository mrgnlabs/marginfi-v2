import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import {
  Connection,
  LAMPORTS_PER_SOL,
  PublicKey,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_STAKE_HISTORY_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import { groupAdmin, users, validators, verbose } from "./rootHooks";
import { StakingCollatizer } from "../target/types/staking_collatizer";
import {
  createStakeAccount,
  delegateStake,
  getStakeAccount,
  getStakeActivation,
} from "./utils/stake-utils";
import { assertBNEqual, assertKeysEqual } from "./utils/genericTests";
import { u64MAX_BN } from "./utils/types";

import path from "path";
import { BankrunProvider } from "anchor-bankrun";
import type { ProgramTestContext } from "solana-bankrun";
import { Clock, startAnchor } from "solana-bankrun";
import {
  deriveStakeHolder,
  deriveStakeHolderStakeAccount,
} from "./utils/stakeCollatizer/pdas";

describe("Create a stake holder for validator", () => {
  const program = workspace.StakingCollatizer as Program<StakingCollatizer>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  it("(admin) Create stake holder for validator 0", async () => {
    let tx = new Transaction();

    tx.add(
      await program.methods
        .initStakeholder()
        .accounts({
          payer: groupAdmin.wallet.publicKey,
          admin: groupAdmin.wallet.publicKey,
          voteAccount: validators[0].voteAccount,
          stakeProgram: new PublicKey(
            "Stake11111111111111111111111111111111111111"
          ),
        })
        .instruction()
    );

    await groupAdmin.userCollatizerProgram.provider.sendAndConfirm(tx);

    const [stakeholderKey] = deriveStakeHolder(
      program.programId,
      validators[0].voteAccount,
      groupAdmin.wallet.publicKey
    );
    const [stakeholderStakeAcc] = deriveStakeHolderStakeAccount(
      program.programId,
      stakeholderKey
    );
    let sh = await program.account.stakeHolder.fetch(stakeholderKey);
    assertKeysEqual(sh.key, stakeholderKey);
    assertKeysEqual(sh.admin, groupAdmin.wallet.publicKey);
    assertKeysEqual(sh.voteAccount, validators[0].voteAccount);
    assertKeysEqual(sh.stakeAccount, stakeholderStakeAcc);
    assertBNEqual(sh.netDelegation, 0);
  });
});
