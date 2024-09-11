import {
  AnchorProvider,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { PublicKey, StakeProgram, Transaction } from "@solana/web3.js";
import {
  bankrunProgram,
  bankrunContext,
  groupAdmin,
  validators,
  banksClient,
  users,
} from "./rootHooks";
import { StakingCollatizer } from "../target/types/staking_collatizer";
import { assertBNEqual, assertKeysEqual } from "./utils/genericTests";

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
          stakeProgram: StakeProgram.programId,
        })
        .instruction()
    );

    tx.recentBlockhash = bankrunContext.lastBlockhash;
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    const [stakeholderKey] = deriveStakeHolder(
      program.programId,
      validators[0].voteAccount,
      groupAdmin.wallet.publicKey
    );
    const [stakeholderStakeAcc] = deriveStakeHolderStakeAccount(
      program.programId,
      stakeholderKey
    );
    let sh = await bankrunProgram.account.stakeHolder.fetch(stakeholderKey);
    assertKeysEqual(sh.key, stakeholderKey);
    assertKeysEqual(sh.admin, groupAdmin.wallet.publicKey);
    assertKeysEqual(sh.voteAccount, validators[0].voteAccount);
    assertKeysEqual(sh.stakeAccount, stakeholderStakeAcc);
    assertBNEqual(sh.netDelegation, 0);
  });

  // TODO move to new file
  it("(user 0) deposit stake to holder in exchange for collateral", async () => {
    let tx = new Transaction();

    let stakeAcc = users[0].accounts.get("v0_stakeacc");
    console.log("read stake acc: " + stakeAcc);
    const [stakeholderKey] = deriveStakeHolder(
      program.programId,
      validators[0].voteAccount,
      groupAdmin.wallet.publicKey
    );

    tx.add(
      await program.methods
        .depositStake()
        .accounts({
          admin: users[0].wallet.publicKey,
          stakeAuthority: users[0].wallet.publicKey,
          stakeholder: stakeholderKey,
          // userStakeAccount: stakeAcc,
          stakeProgram: StakeProgram.programId,
        })
        .instruction()
    );

    tx.recentBlockhash = bankrunContext.lastBlockhash;
    tx.sign(users[0].wallet);
    let res = await banksClient.processTransaction(tx);
    console.log("res " + res);


  });
});
