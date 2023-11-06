<div align="center">
  <img height="170" src="./images/logo.png" />

  <h1>Liquidity Incentive Program (LIP)</h1>
  
  <p>
    <!-- Discord -->
    <a href="https://discord.com/channels/882369954916212737"><img alt="Discord Chat" src="https://img.shields.io/discord/882369954916212737?color=blueviolet&style=flat-square"/></a>
    <!-- License -->
    <a href="http://www.apache.org/licenses/LICENSE-2.0"><img alt="License" src="https://img.shields.io/github/license/mrgnlabs/mrgn-ts?style=flat-square&color=ffff00"/></a>
    <!-- Twitter -->
    <a href="https://twitter.com/intent/tweet?text=Wow:&url=https%3A%2F%2Ftwitter.com%2Fmarginfi"><img alt="Twitter" src="https://img.shields.io/twitter/url?style=social&url=https%3A%2F%2Ftwitter.com%2Fmarginfi"/></a>
    <br>
  </p>

  <h4>
    <a href="https://app.marginfi.com/">app.marginfi.com</a>
  </h4>
</div>

A Solana smart contract that allows anyone to incentivize asset deposits into marginfi-v2, the decentralized borrow/lend protocol on Solana.

## About


`liquidity-incentive-program` is a proxy contract for `marginfi-v2` that allows users to create `campaign`s that incentivize other users to lock up deposits into `marginfi-v2` for guaranteed yields.

## Features

- Permissionless campaign creation
- Arbitrary asset support
- Configurable lock-up periods

## How it works

`liquidity-incentive-program` primarily works off of the `Campaign` concept.

Think of a `Campaign` as a marketing campaign:

1. Each campaign has a creator, who becomes the `admin` of that campaign.
2. The creator selects:
    * the type of asset to incentivize deposits for (e.g. $SOL) 
    * the lockup period that depositors' funds will be locked up for
    * the maximum amount of user deposits allowed
    * The maximum rewards to be paid out to users (together with the maximum amount of deposits allowed, this calculates the guaranteed fixed yield).

> NOTE: LIP works off of the concept of a _minimum_ guaranteed yield, but depositors may earn higher yield if marginfi's native lender yield for the related asset exceeds the yield guaranteed by the `Campaign`. This is a win-win for depositors.

3. As a proof of reward reserves, campaign creators **lock up maximum rewards to be paid out upon campaign creation**, making it easy for campaign depositors to know the source of yield.

4. In product UIs, each `Campaign` typically highlights a fixed `APY`, but there is no compounding involved in the guaranteed yield. Since `APY` accounts for compounding effects even if there are none, measuring yield in `APY` gives depositors the correct impression that they should expect the yield they see. In the smart contract, yield is specified via the `max_rewards` parameter of each `Campaign`.

5. When users deposit funds into an LIP `Campaign`, funds are stored directly in `marginfi`. Funds earn `marginfi` lender yield. When lockups expire, depositors are paid `max(guarenteed yield, earned lender yield)` for the assets they deposited. As earned lender yield grows above `0%`, it subsidizes the expense that campaign creators pay out of the rewards they've escrowed. **This is a win-win for campaign creators**.

## License

`marginfi-v2` and the `liquidity-incentive-program` are open source software licensed under the Apache 2.0 license.
