import {
  AnchorProvider,
  BorshAccountsCoder,
  Program,
} from "@project-serum/anchor";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import {
  AddressLookupTableAccount,
  ConfirmOptions,
  Connection,
  Keypair,
  PublicKey,
  Signer,
  Transaction,
  TransactionMessage,
  TransactionSignature,
  VersionedTransaction,
} from "@solana/web3.js";
import { Wallet } from "./types";
import { MARGINFI_IDL } from "./idl";
import { NodeWallet } from "./nodeWallet";
import {
  AccountType,
  Environment,
  MarginfiConfig,
  MarginfiProgram,
  TransactionOptions,
} from "./types";
import { getEnvFromStr, loadKeypair } from "./utils";
import { getConfig } from "./config";
import MarginfiGroup from "./group";

/**
 * Entrypoint to interact with the marginfi contract.
 */
class MarginfiClient {
  public readonly programId: PublicKey;
  private _group: MarginfiGroup;

  /**
   * @internal
   */
  private constructor(
    readonly config: MarginfiConfig,
    readonly program: MarginfiProgram,
    readonly wallet: Wallet,
    group: MarginfiGroup
  ) {
    this.programId = config.programId;
    this._group = group;
  }

  // --- Factories

  /**
   * MarginfiClient factory
   *
   * Fetch account data according to the config and instantiate the corresponding MarginfiAccount.
   *
   * @param config marginfi config
   * @param wallet User wallet (used to pay fees and sign transations)
   * @param connection Solana web.js Connection object
   * @param opts Solana web.js ConfirmOptions object
   * @returns MarginfiClient instance
   */
  static async fetch(
    config: MarginfiConfig,
    wallet: Wallet,
    connection: Connection,
    opts?: ConfirmOptions
  ) {
    const debug = require("debug")("mfi:client");
    debug(
      "Loading Marginfi Client\n\tprogram: %s\n\tenv: %s\n\tgroup: %s\n\turl: %s",
      config.programId,
      config.environment,
      config.groupPk,
      connection.rpcEndpoint
    );
    const provider = new AnchorProvider(
      connection,
      wallet,
      opts || AnchorProvider.defaultOptions()
    );

    const program = new Program(
      MARGINFI_IDL,
      config.programId,
      provider
    ) as any as MarginfiProgram;
    return new MarginfiClient(
      config,
      program,
      wallet,
      await MarginfiGroup.fetch(config, program)
    );
  }

  static async fromEnv(
    overrides?: Partial<{
      env: Environment;
      connection: Connection;
      programId: PublicKey;
      marginfiGroup: PublicKey;
      wallet: Wallet;
    }>
  ): Promise<MarginfiClient> {
    const debug = require("debug")("mfi:client");
    const env = overrides?.env ?? getEnvFromStr(process.env.MARGINFI_ENV!);
    const connection =
      overrides?.connection ??
      new Connection(process.env.MARGINFI_RPC_ENDPOINT!, {
        commitment: "confirmed",
      });
    const programId =
      overrides?.programId ?? new PublicKey(process.env.MARGINFI_PROGRAM!);
    const groupPk =
      overrides?.marginfiGroup ??
      (process.env.MARGINFI_GROUP
        ? new PublicKey(process.env.MARGINFI_GROUP)
        : PublicKey.default);
    const wallet =
      overrides?.wallet ??
      new NodeWallet(
        process.env.MARGINFI_WALLET_KEY
          ? Keypair.fromSecretKey(
              new Uint8Array(JSON.parse(process.env.MARGINFI_WALLET_KEY))
            )
          : loadKeypair(process.env.MARGINFI_WALLET!)
      );

    debug("Loading the marginfi client from env vars");
    debug(
      "Env: %s\nProgram: %s\nGroup: %s\nSigner: %s",
      env,
      programId,
      groupPk,
      wallet.publicKey
    );

    const config = await getConfig(env, {
      groupPk,
      programId,
    });

    return MarginfiClient.fetch(config, wallet, connection, {
      commitment: connection.commitment,
    });
  }

  // --- Getters and setters

  /**
   * Marginfi account group address
   */
  get group(): MarginfiGroup {
    return this._group;
  }

  get provider(): AnchorProvider {
    return this.program.provider as AnchorProvider;
  }

  // --- Others

  /**
   * Retrieves the addresses of all marginfi accounts in the udnerlying group.
   *
   * @returns Account addresses
   */
  async getAllMarginfiAccountAddresses(): Promise<PublicKey[]> {
    return (
      await this.program.provider.connection.getProgramAccounts(
        this.programId,
        {
          commitment: this.program.provider.connection.commitment,
          dataSlice: {
            offset: 0,
            length: 0,
          },
          filters: [
            {
              memcmp: {
                bytes: this._group.publicKey.toBase58(),
                offset: 8 + 32, // marginfiGroup is the second field in the account after the authority, so offset by the discriminant and a pubkey
              },
            },
            {
              memcmp: {
                offset: 0,
                bytes: bs58.encode(
                  BorshAccountsCoder.accountDiscriminator(
                    AccountType.MarginfiAccount
                  )
                ),
              },
            },
          ],
        }
      )
    ).map((a) => a.pubkey);
  }
  /**
   * Retrieves the addresses of all accounts owned by the marginfi program.
   *
   * @returns Account addresses
   */
  async getAllProgramAccountAddresses(type: AccountType): Promise<PublicKey[]> {
    return (
      await this.program.provider.connection.getProgramAccounts(
        this.programId,
        {
          commitment: this.program.provider.connection.commitment,
          dataSlice: {
            offset: 0,
            length: 0,
          },
          filters: [
            {
              memcmp: {
                offset: 0,
                bytes: bs58.encode(
                  BorshAccountsCoder.accountDiscriminator(type)
                ),
              },
            },
          ],
        }
      )
    ).map((a) => a.pubkey);
  }

  async processTransaction(
    transaction: Transaction,
    signers?: Array<Signer>,
    opts?: TransactionOptions
  ): Promise<TransactionSignature> {
    let signature: TransactionSignature = "";
    try {
      const connection = new Connection(
        this.provider.connection.rpcEndpoint,
        this.provider.opts
      );

      const addressLookupTableAddresses = [
        new PublicKey("8Lh1XmrLjeEFZrsVT7td4Ck4YbYgXrFrkkdjn64KAhav"),
      ];
      const addressLookupTableAccountInfos =
        await this.provider.connection.getMultipleAccountsInfo(
          addressLookupTableAddresses
        );
      const addressLookupTableAccounts = addressLookupTableAccountInfos.map(
        (accountInfo, index) => {
          const key: PublicKey = addressLookupTableAddresses[index];
          if (!accountInfo)
            throw new Error(
              `Missing address lookup table account ${key.toBase58()}`
            );
          return new AddressLookupTableAccount({
            key,
            state: AddressLookupTableAccount.deserialize(accountInfo.data),
          });
        }
      );

      const {
        context: { slot: minContextSlot },
        value: { blockhash, lastValidBlockHeight },
      } = await connection.getLatestBlockhashAndContext();

      const versionedMessage = new TransactionMessage({
        instructions: transaction.instructions,
        payerKey: this.provider.publicKey,
        recentBlockhash: blockhash,
      });
      const versionedTransaction = new VersionedTransaction(
        versionedMessage.compileToV0Message(addressLookupTableAccounts)
      );

      await this.wallet.signTransaction(versionedTransaction);
      if (signers) versionedTransaction.sign(signers);

      if (opts?.dryRun) {
        const response = await connection.simulateTransaction(
          versionedTransaction,
          opts ?? { minContextSlot, sigVerify: false }
        );
        console.log(
          response.value.err
            ? `❌ Error: ${response.value.err}`
            : `✅ Success - ${response.value.unitsConsumed} CU`
        );
        console.log("------ Logs 👇 ------");
        console.log(response.value.logs);

        const signaturesEncoded = encodeURIComponent(
          JSON.stringify(
            versionedTransaction.signatures.map((s) => bs58.encode(s))
          )
        );
        const messageEncoded = encodeURIComponent(
          Buffer.from(versionedTransaction.message.serialize()).toString(
            "base64"
          )
        );
        console.log(
          Buffer.from(versionedTransaction.message.serialize()).toString(
            "base64"
          )
        );

        const urlEscaped = `https://explorer.solana.com/tx/inspector?cluster=${this.config.environment}&signatures=${signaturesEncoded}&message=${messageEncoded}`;
        console.log("------ Inspect 👇 ------");
        console.log(urlEscaped);

        return versionedTransaction.signatures[0].toString();
      } else {
        signature = await connection.sendTransaction(
          versionedTransaction,
          opts ?? { minContextSlot }
        );
        await connection.confirmTransaction({
          blockhash,
          lastValidBlockHeight,
          signature,
        });
        return signature;
      }
    } catch (error: any) {
      throw `Transaction failed! ${error?.message}`;
    }
  }
}

export default MarginfiClient;
