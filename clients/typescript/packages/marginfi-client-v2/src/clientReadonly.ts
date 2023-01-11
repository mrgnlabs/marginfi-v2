import {
  Address,
  AnchorProvider,
  BorshAccountsCoder,
  Program,
  translateAddress,
} from "@project-serum/anchor";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import {
  ConfirmOptions,
  Connection,
  Keypair,
  PublicKey,
} from "@solana/web3.js";
import { InstructionsWrapper } from "./types";
import { MARGINFI_IDL } from "./idl";
import {
  AccountType,
  Environment,
  MarginfiConfig,
  MarginfiProgram,
} from "./types";
import { getConfig } from "./config";
import MarginfiGroup from "./group";
import instructions from "./instructions";
import { DEFAULT_COMMITMENT } from "./constants";
import { MarginfiAccountData } from "./account";
import MarginfiAccountReadonly from "./accountReadonly";

/**
 * Entrypoint to interact with the marginfi contract.
 */
class MarginfiClientReadonly {
  public readonly programId: PublicKey;
  private _group: MarginfiGroup;

  /**
   * @internal
   */
  private constructor(
    readonly config: MarginfiConfig,
    readonly program: MarginfiProgram,
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

    const provider = new AnchorProvider(connection, {} as any, {
      ...AnchorProvider.defaultOptions(),
      commitment:
        connection.commitment ?? AnchorProvider.defaultOptions().commitment,
      ...opts,
    });

    const program = new Program(
      MARGINFI_IDL,
      config.programId,
      provider
    ) as any as MarginfiProgram;
    return new MarginfiClientReadonly(
      config,
      program,
      await MarginfiGroup.fetch(config, program, opts?.commitment)
    );
  }

  static async fromEnv(
    overrides?: Partial<{
      env: Environment;
      connection: Connection;
      programId: Address;
      marginfiGroup: Address;
    }>
  ): Promise<MarginfiClientReadonly> {
    const debug = require("debug")("mfi:client");
    const env = overrides?.env ?? (process.env.MARGINFI_ENV! as Environment);
    const connection =
      overrides?.connection ??
      new Connection(process.env.MARGINFI_RPC_ENDPOINT!, {
        commitment: DEFAULT_COMMITMENT,
      });
    const programId =
      overrides?.programId ?? new PublicKey(process.env.MARGINFI_PROGRAM!);
    const groupPk =
      overrides?.marginfiGroup ??
      (process.env.MARGINFI_GROUP
        ? new PublicKey(process.env.MARGINFI_GROUP)
        : PublicKey.default);

    debug("Loading the marginfi client from env vars");
    debug("Env: %s\nProgram: %s\nGroup: %s", env, programId, groupPk);

    const config = await getConfig(env, {
      groupPk: translateAddress(groupPk),
      programId: translateAddress(programId),
    });

    return MarginfiClientReadonly.fetch(config, connection, {
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
   * Create transaction instruction to create a new marginfi account under the authority of the user.
   *
   * @returns transaction instruction
   */
  async makeCreateMarginfiAccountIx(
    marginfiAccountKeypair?: Keypair
  ): Promise<InstructionsWrapper> {
    const dbg = require("debug")("mfi:client");
    const accountKeypair = marginfiAccountKeypair || Keypair.generate();

    dbg("Generating marginfi account ix for %s", accountKeypair.publicKey);

    const initMarginfiAccountIx = await instructions.makeInitMarginfiAccountIx(
      this.program,
      {
        marginfiGroupPk: this._group.publicKey,
        marginfiAccountPk: accountKeypair.publicKey,
        signerPk: this.provider.wallet.publicKey,
      }
    );

    const ixs = [initMarginfiAccountIx];

    return {
      instructions: ixs,
      keys: [accountKeypair],
    };
  }

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
   * Retrieves all marginfi accounts under the specified authority.
   *
   * @returns MarginfiAccount instances
   */
  async getMarginfiAccountsForAuthority(
    authority: Address
  ): Promise<MarginfiAccountReadonly[]> {
    const marginfiGroup = await MarginfiGroup.fetch(this.config, this.program);
    const _authority = translateAddress(authority);
    return (
      await this.program.account.marginfiAccount.all([
        {
          memcmp: {
            bytes: _authority.toBase58(),
            offset: 8, // authority is the first field in the account, so only offset is the discriminant
          },
        },
        {
          memcmp: {
            bytes: this._group.publicKey.toBase58(),
            offset: 8 + 32, // marginfiGroup is the second field in the account after the authority, so offset by the discriminant and a pubkey
          },
        },
      ])
    ).map((a) =>
      MarginfiAccountReadonly.fromAccountData(
        a.publicKey,
        this,
        a.account as MarginfiAccountData,
        marginfiGroup
      )
    );
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
}

export default MarginfiClientReadonly;
