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

  const group = marginfiAccount.group;

  const bankLabel1 = "SOL";
  const bank1 = group.getBankByLabel(bankLabel1);
  if (!bank1) throw Error(`${bankLabel1} bank not found`);

  const bankLabel2 = "USDC";
  const bank2 = group.getBankByLabel(bankLabel2);
  if (!bank2) throw Error(`${bankLabel2} bank not found`);

  const sig1 = await marginfiAccount.deposit(1, bank1);
  console.log("deposit", sig1);

  await marginfiAccount.reload();

  const sig2 = await marginfiAccount.withdraw(5, bank2);
  console.log("withdraw", sig2);
}

main();
