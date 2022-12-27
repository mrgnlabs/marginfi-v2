import { Connection } from "@solana/web3.js";
import {
  AccountType,
  Environment,
  getConfig,
  MarginfiClient,
  NodeWallet,
} from "../src";

async function main() {
  const connection = new Connection("https://devnet.genesysgo.net/");
  const wallet = NodeWallet.local();
  const config = await getConfig(Environment.DEVNET);
  const client = await MarginfiClient.fetch(config, wallet, connection);

  const programAddresses = await client.getAllProgramAccountAddresses(
    AccountType.MarginfiGroup
  );
  console.log(programAddresses.map((key) => key.toBase58()));
}

main();
