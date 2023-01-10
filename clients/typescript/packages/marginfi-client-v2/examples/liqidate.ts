import { Connection, PublicKey } from "@solana/web3.js";
import BigNumber from "bignumber.js";
import { AccountType, getConfig, MarginfiClient, NodeWallet } from "../src";
import MarginfiAccount, { MarginRequirementType } from "../src/account";

const LIQUIDATOR_PK = new PublicKey("");

async function main() {
  const connection = new Connection(
    "https://devnet.genesysgo.net/",
    "confirmed"
  );
  const wallet = NodeWallet.local();
  const config = await getConfig("devnet1");
  const client = await MarginfiClient.fetch(config, wallet, connection);

  const programAddresses = await client.getAllProgramAccountAddresses(
    AccountType.MarginfiGroup
  );
  console.log(programAddresses.map((key) => key.toBase58()));

  const liquidatorAccount = await MarginfiAccount.fetch(LIQUIDATOR_PK, client);
  const marginfiAccount = await MarginfiAccount.fetch(
    "6tgsmyfNHVzZaDJ6bjSVrKBGVsrgpqHNzr7WDz3BeT7t",
    client
  );

  const group = marginfiAccount.group;

  const bankLabel1 = "SOL";
  const bank1 = group.getBankByLabel(bankLabel1);
  if (!bank1) throw Error(`${bankLabel1} bank not found`);

  const bankLabel2 = "USDC";
  const bank2 = group.getBankByLabel(bankLabel2);
  if (!bank2) throw Error(`${bankLabel2} bank not found`);

  if (marginfiAccount.canBeLiquidated()) {
    const [assets, liabs] = marginfiAccount.getHealthComponents(
      MarginRequirementType.Maint
    );

    const maxLiabilityPaydown = liabs.minus(assets);
    console.log(
      "Account can be liquidated, max liability paydown: %d",
      maxLiabilityPaydown
    );
  } else {
    console.log("Account cannot be liquidated");
  }

  if (!marginfiAccount.canBeLiquidated()) {
    console.log("Account is healthy");
  }

  const [liquidatorAssets, liquidatorLiab] =
    liquidatorAccount.getHealthComponents(MarginRequirementType.Init);

  const freeCollateral = BigNumber.max(
    liquidatorAssets.minus(liquidatorLiab),
    0
  );

  marginfiAccount.lendingAccount.map((balance, i) => {
    const bank = group.banks.get(balance.bankPk.toString())!;
    const [_, liabs] = balance.getValue(bank);
    const maxLiquidatorWithdrawCapacity =
      liquidatorAccount.getMaxWithdrawForBank(bank);
    const maxLiabCoverage = BigNumber.min(liabs, maxLiquidatorWithdrawCapacity);

    // TODO: Find max usd amount that can be covered by liquidator, out of all liabilities held by the liquidatee
  });
}

main();
