import {
  Keypair,
  PublicKey,
  Transaction,
  VersionedTransaction,
} from "@solana/web3.js";
import { Wallet } from "./types";

/**
 * NodeWallet
 *
 * Anchor-compliant wallet implementation.
 */
export class NodeWallet implements Wallet {
  /**
   * @param payer Keypair of the associated payer
   */
  constructor(readonly payer: Keypair) {}

  /**
   * Factory for the local wallet.
   * Makes use of the `MARGINFI_WALLET` env var, with fallback to `$HOME/.config/solana/id.json`.
   */
  static local(): NodeWallet {
    const process = require("process");
    const payer = Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(
          require("fs").readFileSync(
            process.env.MARGINFI_WALLET ||
              require("path").join(
                require("os").homedir(),
                ".config/solana/id.json"
              ),
            {
              encoding: "utf-8",
            }
          )
        )
      )
    );
    return new NodeWallet(payer);
  }

  /**
   * Factory for the Anchor local wallet.
   */
  static anchor(): NodeWallet {
    const process = require("process");
    if (!process.env.ANCHOR_WALLET || process.env.ANCHOR_WALLET === "") {
      throw new Error(
        "expected environment variable `ANCHOR_WALLET` is not set."
      );
    }
    const payer = Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(
          require("fs").readFileSync(process.env.ANCHOR_WALLET, {
            encoding: "utf-8",
          })
        )
      )
    );
    return new NodeWallet(payer);
  }

  async signTransaction<T extends Transaction | VersionedTransaction>(
    tx: T
  ): Promise<T> {
    if ("version" in tx) {
      tx.sign([this.payer]);
    } else {
      tx.partialSign(this.payer);
    }
    return tx;
  }

  async signAllTransactions<T extends Transaction | VersionedTransaction>(
    txs: T[]
  ): Promise<T[]> {
    return txs.map((tx) => {
      if ("version" in tx) {
        tx.sign([this.payer]);
        return tx;
      } else {
        tx.partialSign(this.payer);
        return tx;
      }
    });
  }

  get publicKey(): PublicKey {
    return this.payer.publicKey;
  }
}
