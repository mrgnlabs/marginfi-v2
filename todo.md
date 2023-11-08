## TODO

## Release Scope

### Alpha

* USDC, wSOL, BONK
* Jup Liquidator
* UI

### Full Mainnet

* USDC, wSOL, USDT, ??BONK??
* Liquidator
* UI

### Alpha Release

* Bonk Oracle
  * Integrate Switchboard or coordinate with the Pyth team, ideally we use Pyth.
* Rust Client
    Ongoing -- developed as needed.
* TS Client
    Developed as needed for the UI, and any bots.
* Crank Bot
    Rust bot for accruing interest on all banks in a group.

* Stress Testing
  * Manual Testing - Part of alpha testing
  * Fuzz Testing - Stress testing the internal accounting model

### Features

* Share Tokenization (Pool Party)

### Risk Management Improvements

* Isolated Borrowing
* Borrow cap (instead of deposit cap)
* Liquidator
  * Jup TS Liquidator (good for alpha phase)
        Good enough for alpha testing, need a more robust solution for full public mainnet.
  * Live Mainnet Liquidator
        Bigger project, should start right after alpha mainnet launch, after jup liquidator.
