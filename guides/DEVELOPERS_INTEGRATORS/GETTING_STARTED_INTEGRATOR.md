# Integrator Quickstart Guide

Do you want to build on top of mrgnv2? Read on.

## Ecosystem Resources

Be aware of:
* Our TS packages: 
  * https://www.npmjs.com/package/@mrgnlabs/marginfi-client-v2
  * https://www.npmjs.com/package/@mrgnlabs/mrgn-common
* Our RUST CLI: https://github.com/mrgnlabs/marginfi-v2/tree/main/clients/rust/marginfi-cli
* Our example scripts: https://github.com/mrgnlabs/mrgn-ts-scripts/tree/master/scripts

## Important Instructions (click to learn more)

<details>
<summary> <b>lending_account_deposit</b> - deposit into any Bank EXCEPT integrator banks (Kamino, etc)</summary>

   - Check `bank.config.asset_tag`, ASSET_TAG_DEFAULT (0) or ASSET_TAG_SOL (1) ASSET_TAG_STAKED (2) are allowed with this instruction. Others have their own deposit instruction.
   * `amount` is in native token, in native decimal, e.g. 1 SOL = 1 * 10^9
   * Set `deposit_up_to_limit` to "true" to ignore your amount input if near the deposit cap and
     deposit only what is available. For example, if the deposit cap is 10 SOL, there is 6 SOL in
     the bank, and you attempt to deposit 10, `deposit_up_to_limit` = true will deposit 4,
     `deposit_up_to_limit` = false will fail and abort.
</details>

<details>
<summary> <b>kamino_deposit</b> - deposit into a Kamino Bank</summary>

   - Check `bank.config.asset_tag` ASSET_TAG_KAMINO (3) is allowed with this instruction. Others have their own deposit instruction.
   * `amount` is in native token (of the Kamino reserve), in native decimal, e.g. 1 SOL = 1 * 10^9
</details>