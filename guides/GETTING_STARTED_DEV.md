## NEW DEV QUICKSTART GUIDE

New developer getting started working on the mrgnv2 program side? Read on.

### Things to Install (April 2025)

- rust/cargo - latest stable
- node - 23.0.0
- yarn - 1.22.22
- avm - 0.30.1 or later
- anchor - 0.31.1
- solana - 2.1.20
- cargo-nextest - use `cargo install cargo-nextest --version "0.9.81" --locked` exactly
- cargo-fuzz - 0.12.0

## Running tests

### For unit tests:

```
cargo test --lib
```

### For the TS test suite:

```
anchor build -p marginfi -- --no-default-features
anchor test --skip-build
```

Note: you may need to build the other programs (mock, liquidity incentive, etc) if you have never run anchor build before.

Segmentation fault? Just try again. That happens sometimes, generally on the first run of the day.

Each letter prefix is referred to as a "suite" and is broadly end-to-end. The localnet tests
multithread with bankrun and will create a fairly substantial CPU load. Completetion varies
substantially by hardware. If you your workflow is too slow, go to this portion of Anchor.toml and
comment out the top line, comment in the suite you actually want to run:

```
[scripts]
test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/*.spec.ts --exit --require tests/rootHooks.ts"

# Staked collateral tests only
# test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/s*.spec.ts --exit --require tests/rootHooks.ts"

# Pyth pull tests only
# test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/p*.spec.ts --exit --require tests/rootHooks.ts"

# Edmode tests only
# test = "RUST_LOG= yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/e*.spec.ts --exit --require tests/rootHooks.ts"
```

Note: You cannot run individual tests, most of the tests in a suite must run in order, where the
number after the prefix determines their run order through the magic of filenames.

### For the Rust test suite:

```
anchor build -p marginfi
./scripts/test-program.sh marginfi mainnet-beta
```

This is much slower than the remix test command, but stable on any system.

### Customize Your Rust testing experience:

```
./scripts/test-program-remix.sh -p marginfi -l warn -c mainnet-beta -f mainnet-beta
```

This will throttle your CPU and may error sporadically as a reminder to buy a better CPU if you try to do anything else (like say, compile another Rust repo) while this is running. It is approximately 10x faster than test-program.sh, so use this one if you value your time and sanity. Feel free to add your flex below:

Benchmarks:

- 9700X: `Summary [   6.302s] 238 tests run: 238 passed, 0 skipped`

### To run just one Rust test:

```
./scripts/single-test.sh marginfi accrue_interest --verbose
./scripts/single-test.sh test_name --verbose
```

### To run the fuzz suite

Don't.

If you really want to, open the fuzz directory in a new IDE window (it's not part of the workspace,
by design). Run the generate-corpus Python script (you may need to install Python), then cargo
build, then run it. It may take a very long time and use a very large amount of disk space.
Typically, we run this to see if there are errors, and close it early if there are not, there is
typically no need to wait for the suite to finish locally.


## Common issues

### The TS suite fails with `Environment supports crypto:  false` at the top

Update Node

### All the tests are failing in Rust and/or TS

Make sure you build the correct version, Rust requires the mainnet version (default features), TS wants localnet (no features)
