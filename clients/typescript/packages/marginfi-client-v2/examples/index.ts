import { Connection } from "@solana/web3.js";
import { AccountType, getConfig, MarginfiClient, NodeWallet } from "../src";
import MarginfiAccount from "../src/account";

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

  // const marginfiAccount = await client.createMarginfiAccount({
  //   dryRun: false,
  // });

  const marginfiAccount = await MarginfiAccount.fetch(
    "6tgsmyfNHVzZaDJ6bjSVrKBGVsrgpqHNzr7WDz3BeT7t",
    client
  );

  const bankLabel = "SOL";
  const group = marginfiAccount.group;

  const bank = group.getBankByLabel(bankLabel);
  if (!bank) throw Error(`${bankLabel} bank not found`);

  const sig1 = await marginfiAccount.deposit(1, bank);
  console.log("deposit", sig1);

  await marginfiAccount.reload();

  const sig2 = await marginfiAccount.withdraw(0.9, bank);
  console.log("withdraw", sig2);
}

main();
