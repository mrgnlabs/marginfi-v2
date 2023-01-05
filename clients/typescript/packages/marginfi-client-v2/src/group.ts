import { BorshCoder } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";
import { MARGINFI_IDL } from "./idl";
// import instructions from "./instructions";
import {
  AccountType,
  MarginfiConfig,
  MarginfiGroupData,
  MarginfiProgram,
} from "./types";

/**
 * Wrapper class around a specific marginfi group.
 */
class MarginfiGroup {
  public readonly publicKey: PublicKey;
  public readonly admin: PublicKey;

  private _program: MarginfiProgram;
  private _config: MarginfiConfig;

  /**
   * @internal
   */
  private constructor(
    config: MarginfiConfig,
    program: MarginfiProgram,
    rawData: MarginfiGroupData
  ) {
    this.publicKey = config.group;
    this._config = config;
    this._program = program;

    this.admin = rawData.admin;
  }

  // --- Factories

  /**
   * MarginfiGroup network factory
   *
   * Fetch account data according to the config and instantiate the corresponding MarginfiGroup.
   *
   * @param config marginfi config
   * @param program marginfi Anchor program
   * @return MarginfiGroup instance
   */
  static async fetch(config: MarginfiConfig, program: MarginfiProgram) {
    const debug = require("debug")(`mfi:margin-group`);
    debug("Loading Marginfi Group %s", config.group);
    const accountData = await MarginfiGroup._fetchAccountData(config, program);
    return new MarginfiGroup(config, program, accountData);
  }

  /**
   * MarginfiGroup local factory (decoded)
   *
   * Instantiate a MarginfiGroup according to the provided decoded data.
   * Check sanity against provided config.
   *
   * @param config marginfi config
   * @param program marginfi Anchor program
   * @param accountData Decoded marginfi group data
   * @return MarginfiGroup instance
   */
  static fromAccountData(
    config: MarginfiConfig,
    program: MarginfiProgram,
    accountData: MarginfiGroupData
  ) {
    return new MarginfiGroup(config, program, accountData);
  }

  /**
   * MarginfiGroup local factory (encoded)
   *
   * Instantiate a MarginfiGroup according to the provided encoded data.
   * Check sanity against provided config.
   *
   * @param config marginfi config
   * @param program marginfi Anchor program
   * @param data Encoded marginfi group data
   * @return MarginfiGroup instance
   */
  static fromAccountDataRaw(
    config: MarginfiConfig,
    program: MarginfiProgram,
    rawData: Buffer
  ) {
    const data = MarginfiGroup.decode(rawData);
    return MarginfiGroup.fromAccountData(config, program, data);
  }

  // --- Others

  /**
   * Fetch marginfi group account data according to the config.
   * Check sanity against provided config.
   *
   * @param config marginfi config
   * @param program marginfi Anchor program
   * @return Decoded marginfi group account data struct
   */
  private static async _fetchAccountData(
    config: MarginfiConfig,
    program: MarginfiProgram
  ): Promise<MarginfiGroupData> {
    const data: MarginfiGroupData = (await program.account.marginfiGroup.fetch(
      config.group
    )) as any;

    return data;
  }

  /**
   * Decode marginfi group account data according to the Anchor IDL.
   *
   * @param encoded Raw data buffer
   * @return Decoded marginfi group account data struct
   */
  static decode(encoded: Buffer): MarginfiGroupData {
    const coder = new BorshCoder(MARGINFI_IDL);
    return coder.accounts.decode(AccountType.MarginfiGroup, encoded);
  }

  /**
   * Encode marginfi group account data according to the Anchor IDL.
   *
   * @param decoded Encoded marginfi group account data buffer
   * @return Raw data buffer
   */
  static async encode(decoded: MarginfiGroupData): Promise<Buffer> {
    const coder = new BorshCoder(MARGINFI_IDL);
    return await coder.accounts.encode(AccountType.MarginfiGroup, decoded);
  }

  /**
   * Update instance data by fetching and storing the latest on-chain state.
   */
  async fetch() {
    const data = await MarginfiGroup._fetchAccountData(
      this._config,
      this._program
    );
    // this.bank = new Bank(data.bank);
  }
}

export default MarginfiGroup;
