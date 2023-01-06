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
  console.log(client.config);

  // const marginfiAccount = await client.createMarginfiAccount({
  //   dryRun: false,
  // });
  // console.log(marginfiAccount.publicKey);

  const marginfiAccount = await MarginfiAccount.fetch(
    "Bvpe2RPREvEUDpsP7PZKG6kbZRB933uSArrhqgyEfBUp",
    client
  );

  console.log(marginfiAccount.group);

  // marginfiAccount.
}

main();
