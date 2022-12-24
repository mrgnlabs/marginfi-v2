import { Connection } from "@solana/web3.js";

async function main() {
  const connection = new Connection("https://devnet.genesysgo.net/");
  const epochInfo = await connection.getEpochInfo();
  console.log(epochInfo);
}

main();
