/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/exponent_core.json`.
 */
export type ExponentCore = {
  "address": "ExponentnaRg3CQbW6dqQNZKXp7gtZ9DGMp1cwC4HAS7",
  "metadata": {
    "name": "exponentCore",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "addEmission",
      "discriminator": [
        18
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "feePayer",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "admin"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "robotTokenAccount",
          "docs": [
            "Assert that the new token account is owned by the vault"
          ]
        },
        {
          "name": "treasuryTokenAccount"
        },
        {
          "name": "yieldPosition",
          "docs": [
            "Increase the robot's position size"
          ],
          "writable": true
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": [
        {
          "name": "cpiAccounts",
          "type": {
            "defined": {
              "name": "cpiAccounts"
            }
          }
        },
        {
          "name": "treasuryFeeBps",
          "type": "u16"
        }
      ]
    },
    {
      "name": "addFarm",
      "discriminator": [
        22
      ],
      "accounts": [
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "feePayer",
          "writable": true,
          "signer": true
        },
        {
          "name": "mintNew"
        },
        {
          "name": "adminState"
        },
        {
          "name": "tokenSource",
          "writable": true
        },
        {
          "name": "tokenFarm",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": [
        {
          "name": "tokenRate",
          "type": "u64"
        },
        {
          "name": "untilTimestamp",
          "type": "u32"
        }
      ]
    },
    {
      "name": "addLpTokensMetadata",
      "discriminator": [
        41
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "admin"
        },
        {
          "name": "market"
        },
        {
          "name": "mintLp"
        },
        {
          "name": "metadata",
          "writable": true
        },
        {
          "name": "tokenMetadataProgram"
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": [
        {
          "name": "name",
          "type": "string"
        },
        {
          "name": "symbol",
          "type": "string"
        },
        {
          "name": "uri",
          "type": "string"
        }
      ]
    },
    {
      "name": "addMarketEmission",
      "discriminator": [
        25
      ],
      "accounts": [
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "feePayer",
          "writable": true,
          "signer": true
        },
        {
          "name": "mintNew"
        },
        {
          "name": "adminState"
        },
        {
          "name": "tokenEmission",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": [
        {
          "name": "cpiAccounts",
          "type": {
            "defined": {
              "name": "cpiAccounts"
            }
          }
        }
      ]
    },
    {
      "name": "buyYt",
      "docs": [
        "Buy YT with SY"
      ],
      "discriminator": [
        0
      ],
      "accounts": [
        {
          "name": "trader",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenSyTrader",
          "docs": [
            "Source account for the trader's SY tokens"
          ],
          "writable": true
        },
        {
          "name": "tokenYtTrader",
          "docs": [
            "Destination for receiving YT tokens"
          ],
          "writable": true
        },
        {
          "name": "tokenPtTrader",
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "docs": [
            "Temporary SY account owned by market"
          ],
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "docs": [
            "PT liquidity owned by market"
          ],
          "writable": true
        },
        {
          "name": "tokenFeeTreasurySy",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "vaultAuthority",
          "writable": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "tokenSyEscrowVault",
          "writable": true
        },
        {
          "name": "mintYt",
          "writable": true
        },
        {
          "name": "mintPt",
          "writable": true
        },
        {
          "name": "addressLookupTableVault"
        },
        {
          "name": "yieldPosition",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "syIn",
          "type": "u64"
        },
        {
          "name": "ytOut",
          "type": "u64"
        }
      ],
      "returns": {
        "defined": {
          "name": "buyYtEvent"
        }
      }
    },
    {
      "name": "claimFarmEmissions",
      "discriminator": [
        24
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "market"
        },
        {
          "name": "lpPosition",
          "writable": true
        },
        {
          "name": "tokenDst",
          "writable": true
        },
        {
          "name": "mint"
        },
        {
          "name": "tokenFarm",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": {
            "defined": {
              "name": "amount"
            }
          }
        }
      ],
      "returns": {
        "defined": {
          "name": "claimFarmEmissionsEventV2"
        }
      }
    },
    {
      "name": "collectEmission",
      "discriminator": [
        19
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "position",
          "writable": true
        },
        {
          "name": "syProgram"
        },
        {
          "name": "authority"
        },
        {
          "name": "emissionEscrow",
          "writable": true
        },
        {
          "name": "emissionDst",
          "writable": true
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "treasuryEmissionTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u16"
        },
        {
          "name": "amount",
          "type": {
            "defined": {
              "name": "amount"
            }
          }
        }
      ],
      "returns": {
        "defined": {
          "name": "collectEmissionEventV2"
        }
      }
    },
    {
      "name": "collectInterest",
      "discriminator": [
        6
      ],
      "accounts": [
        {
          "name": "owner",
          "docs": [
            "Owner of the YieldTokenPosition"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "yieldPosition",
          "docs": [
            "Owner's position data for YT deposits"
          ],
          "writable": true
        },
        {
          "name": "vault",
          "docs": [
            "The vault that holds the SY tokens in escrow"
          ],
          "writable": true
        },
        {
          "name": "tokenSyDst",
          "docs": [
            "The receiving token account for SY withdrawn"
          ],
          "writable": true
        },
        {
          "name": "escrowSy",
          "writable": true
        },
        {
          "name": "authority",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "treasurySyTokenAccount",
          "writable": true
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": {
            "defined": {
              "name": "amount"
            }
          }
        }
      ],
      "returns": {
        "defined": {
          "name": "collectInterestEventV2"
        }
      }
    },
    {
      "name": "collectTreasuryEmission",
      "discriminator": [
        20
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "yieldPosition",
          "docs": [
            "constrained by vault"
          ],
          "writable": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "syProgram"
        },
        {
          "name": "authority",
          "docs": [
            "constrained by vault"
          ]
        },
        {
          "name": "emissionEscrow",
          "writable": true
        },
        {
          "name": "emissionDst",
          "writable": true
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "admin"
        }
      ],
      "args": [
        {
          "name": "emissionIndex",
          "type": "u16"
        },
        {
          "name": "amount",
          "type": {
            "defined": {
              "name": "amount"
            }
          }
        },
        {
          "name": "kind",
          "type": {
            "defined": {
              "name": "collectTreasuryEmissionKind"
            }
          }
        }
      ]
    },
    {
      "name": "collectTreasuryInterest",
      "discriminator": [
        21
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "yieldPosition",
          "writable": true
        },
        {
          "name": "vault",
          "docs": [
            "The vault that holds the SY tokens in escrow"
          ],
          "writable": true
        },
        {
          "name": "syDst",
          "docs": [
            "The receiving token account for SY withdrawn"
          ],
          "writable": true
        },
        {
          "name": "escrowSy",
          "writable": true
        },
        {
          "name": "authority"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "admin"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": {
            "defined": {
              "name": "amount"
            }
          }
        },
        {
          "name": "kind",
          "type": {
            "defined": {
              "name": "collectTreasuryInterestKind"
            }
          }
        }
      ]
    },
    {
      "name": "depositYt",
      "docs": [
        "Deposit YT into escrow in order to earn rewards & SY interest"
      ],
      "discriminator": [
        7
      ],
      "accounts": [
        {
          "name": "depositor",
          "docs": [
            "Permissionless depositor - not necessarily the owner of the YieldTokenPosition"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "vault",
          "docs": [
            "Vault that manages YT"
          ],
          "writable": true
        },
        {
          "name": "userYieldPosition",
          "docs": [
            "Position data for YT deposits, linked to vault"
          ],
          "writable": true
        },
        {
          "name": "ytSrc",
          "docs": [
            "Depositor's YT token account"
          ],
          "writable": true
        },
        {
          "name": "escrowYt",
          "docs": [
            "Vault-owned escrow account for YT"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram",
          "docs": [
            "The SY interface implementation for the vault's token"
          ]
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "yieldPosition",
          "docs": [
            "Vault-owned yield token position"
          ],
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ],
      "returns": {
        "defined": {
          "name": "depositYtEventV2"
        }
      }
    },
    {
      "name": "initLpPosition",
      "docs": [
        "Initialize a LP position for a user to deposit LP tokens into"
      ],
      "discriminator": [
        13
      ],
      "accounts": [
        {
          "name": "feePayer",
          "writable": true,
          "signer": true
        },
        {
          "name": "owner"
        },
        {
          "name": "market"
        },
        {
          "name": "lpPosition",
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [],
      "returns": {
        "defined": {
          "name": "initLpPositionEvent"
        }
      }
    },
    {
      "name": "initMarketTwo",
      "discriminator": [
        10
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "adminSigner",
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "There is 1 market per vault"
          ],
          "writable": true
        },
        {
          "name": "vault",
          "docs": [
            "Links the mint_sy & mint_pt & sy_program together"
          ]
        },
        {
          "name": "mintSy"
        },
        {
          "name": "mintPt"
        },
        {
          "name": "mintLp",
          "writable": true
        },
        {
          "name": "escrowPt",
          "writable": true
        },
        {
          "name": "escrowSy",
          "docs": [
            "This account for SY is only a temporary pass-through account",
            "It is used to transfer SY tokens from the signer to the market",
            "And then from the market to the SY program's escrow"
          ],
          "writable": true
        },
        {
          "name": "escrowLp",
          "docs": [
            "Holds activated LP tokens for farming & SY emissions"
          ],
          "writable": true
        },
        {
          "name": "ptSrc",
          "docs": [
            "Signer's PT token account"
          ],
          "writable": true
        },
        {
          "name": "sySrc",
          "docs": [
            "Signer's SY token account"
          ],
          "writable": true
        },
        {
          "name": "lpDst",
          "docs": [
            "Receiving account for LP tokens"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Use the old Token program as the implementation for PT & SY & LP tokens"
          ]
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "associatedTokenProgram"
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "admin"
        },
        {
          "name": "tokenTreasuryFeeSy"
        }
      ],
      "args": [
        {
          "name": "lnFeeRateRoot",
          "type": "f64"
        },
        {
          "name": "rateScalarRoot",
          "type": "f64"
        },
        {
          "name": "initRateAnchor",
          "type": "f64"
        },
        {
          "name": "syExchangeRate",
          "type": {
            "defined": {
              "name": "number"
            }
          }
        },
        {
          "name": "ptInit",
          "type": "u64"
        },
        {
          "name": "syInit",
          "type": "u64"
        },
        {
          "name": "feeTreasurySyBps",
          "type": "u16"
        },
        {
          "name": "cpiAccounts",
          "type": {
            "defined": {
              "name": "cpiAccounts"
            }
          }
        },
        {
          "name": "seedId",
          "type": "u8"
        }
      ]
    },
    {
      "name": "initializeVault",
      "docs": [
        "High-trust function to init vault"
      ],
      "discriminator": [
        2
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "admin"
        },
        {
          "name": "authority",
          "docs": [
            "The signer for the vault"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "signer": true
        },
        {
          "name": "mintPt",
          "writable": true
        },
        {
          "name": "mintYt",
          "writable": true
        },
        {
          "name": "escrowYt",
          "writable": true
        },
        {
          "name": "escrowSy",
          "writable": true
        },
        {
          "name": "mintSy"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "treasuryTokenAccount"
        },
        {
          "name": "associatedTokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "yieldPosition",
          "writable": true
        },
        {
          "name": "metadata",
          "writable": true
        },
        {
          "name": "tokenMetadataProgram"
        }
      ],
      "args": [
        {
          "name": "startTimestamp",
          "type": "u32"
        },
        {
          "name": "duration",
          "type": "u32"
        },
        {
          "name": "interestBpsFee",
          "type": "u16"
        },
        {
          "name": "cpiAccounts",
          "type": {
            "defined": {
              "name": "cpiAccounts"
            }
          }
        },
        {
          "name": "minOpSizeStrip",
          "type": "u64"
        },
        {
          "name": "minOpSizeMerge",
          "type": "u64"
        },
        {
          "name": "ptMetadataName",
          "type": "string"
        },
        {
          "name": "ptMetadataSymbol",
          "type": "string"
        },
        {
          "name": "ptMetadataUri",
          "type": "string"
        }
      ]
    },
    {
      "name": "initializeYieldPosition",
      "discriminator": [
        3
      ],
      "accounts": [
        {
          "name": "owner",
          "docs": [
            "Owner of the new YieldTokenPosition",
            "ie - the \"owner\" is not a Signer"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "vault"
        },
        {
          "name": "yieldPosition",
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [],
      "returns": {
        "defined": {
          "name": "initializeYieldPositionEvent"
        }
      }
    },
    {
      "name": "marketCollectEmission",
      "docs": [
        "Collect a staged emission"
      ],
      "discriminator": [
        16
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "lpPosition",
          "writable": true
        },
        {
          "name": "tokenEmissionEscrow",
          "writable": true
        },
        {
          "name": "tokenEmissionDst",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "emissionIndex",
          "type": "u16"
        }
      ],
      "returns": {
        "defined": {
          "name": "marketCollectEmissionEventV2"
        }
      }
    },
    {
      "name": "marketDepositLp",
      "docs": [
        "Deposit LP tokens into a personal LP position account"
      ],
      "discriminator": [
        14
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "lpPosition",
          "writable": true
        },
        {
          "name": "tokenLpSrc",
          "writable": true
        },
        {
          "name": "tokenLpEscrow",
          "writable": true
        },
        {
          "name": "mintLp"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ],
      "returns": {
        "defined": {
          "name": "depositLpEventV2"
        }
      }
    },
    {
      "name": "marketTwoDepositLiquidity",
      "discriminator": [
        11
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenPtSrc",
          "writable": true
        },
        {
          "name": "tokenSySrc",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "writable": true
        },
        {
          "name": "tokenLpDst",
          "writable": true
        },
        {
          "name": "mintLp",
          "writable": true
        },
        {
          "name": "addressLookupTable",
          "docs": [
            "Address lookup table for vault"
          ]
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "ptIntent",
          "type": "u64"
        },
        {
          "name": "syIntent",
          "type": "u64"
        },
        {
          "name": "minLpOut",
          "type": "u64"
        }
      ],
      "returns": {
        "defined": {
          "name": "depositLiquidityEvent"
        }
      }
    },
    {
      "name": "marketTwoWithdrawLiquidity",
      "discriminator": [
        12
      ],
      "accounts": [
        {
          "name": "withdrawer",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenPtDst",
          "writable": true
        },
        {
          "name": "tokenSyDst",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "docs": [
            "Market PT liquidity account"
          ],
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "docs": [
            "Market-owned interchange account for SY"
          ],
          "writable": true
        },
        {
          "name": "tokenLpSrc",
          "writable": true
        },
        {
          "name": "mintLp",
          "writable": true
        },
        {
          "name": "addressLookupTable",
          "docs": [
            "Address lookup table for vault"
          ]
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "lpIn",
          "type": "u64"
        },
        {
          "name": "minPtOut",
          "type": "u64"
        },
        {
          "name": "minSyOut",
          "type": "u64"
        }
      ],
      "returns": {
        "defined": {
          "name": "withdrawLiquidityEvent"
        }
      }
    },
    {
      "name": "marketWithdrawLp",
      "docs": [
        "Withdraw LP tokens from personal LP position account"
      ],
      "discriminator": [
        15
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "mintLp"
        },
        {
          "name": "lpPosition",
          "writable": true
        },
        {
          "name": "tokenLpDst",
          "writable": true
        },
        {
          "name": "tokenLpEscrow",
          "writable": true
        },
        {
          "name": "syProgram"
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ],
      "returns": {
        "defined": {
          "name": "withdrawLpEventV2"
        }
      }
    },
    {
      "name": "merge",
      "docs": [
        "Merge PT + YT into SY",
        "Redeems & burns them, in exchange for SY"
      ],
      "discriminator": [
        5
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "authority",
          "docs": [
            "This authority owns the escrow_sy account & the robot account with the SY program",
            "Needs to be mutable to be used in deposit_sy CPI"
          ],
          "writable": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "syDst",
          "docs": [
            "Destination account for SY withdrawn from vault"
          ],
          "writable": true
        },
        {
          "name": "escrowSy",
          "docs": [
            "Vault-owned account for SY tokens"
          ],
          "writable": true
        },
        {
          "name": "ytSrc",
          "docs": [
            "The owner's YT token account"
          ],
          "writable": true
        },
        {
          "name": "ptSrc",
          "docs": [
            "The owner's PT token account"
          ],
          "writable": true
        },
        {
          "name": "mintYt",
          "docs": [
            "Mint for YT -- needed for burning"
          ],
          "writable": true
        },
        {
          "name": "mintPt",
          "docs": [
            "Mint for PT -- needed for burning"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "yieldPosition",
          "docs": [
            "Yield position for the vault robot account"
          ],
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ],
      "returns": {
        "defined": {
          "name": "mergeEvent"
        }
      }
    },
    {
      "name": "modifyFarm",
      "discriminator": [
        23
      ],
      "accounts": [
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "mint"
        },
        {
          "name": "adminState"
        },
        {
          "name": "tokenSource",
          "writable": true
        },
        {
          "name": "tokenFarm",
          "writable": true
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": [
        {
          "name": "untilTimestamp",
          "type": "u32"
        },
        {
          "name": "newRate",
          "type": "u64"
        }
      ]
    },
    {
      "name": "modifyMarketSetting",
      "discriminator": [
        27
      ],
      "accounts": [
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "adminState"
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": [
        {
          "name": "action",
          "type": {
            "defined": {
              "name": "marketAdminAction"
            }
          }
        }
      ]
    },
    {
      "name": "modifyVaultSetting",
      "discriminator": [
        26
      ],
      "accounts": [
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "adminState"
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": [
        {
          "name": "action",
          "type": {
            "defined": {
              "name": "adminAction"
            }
          }
        }
      ]
    },
    {
      "name": "reallocMarket",
      "discriminator": [
        40
      ],
      "accounts": [
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "adminState"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "rent"
        }
      ],
      "args": [
        {
          "name": "additionalBytes",
          "type": "u64"
        }
      ]
    },
    {
      "name": "sellYt",
      "docs": [
        "Sell YT for SY"
      ],
      "discriminator": [
        1
      ],
      "accounts": [
        {
          "name": "trader",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenYtTrader",
          "writable": true
        },
        {
          "name": "tokenPtTrader",
          "writable": true
        },
        {
          "name": "tokenSyTrader",
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "docs": [
            "Account owned by market",
            "This is a temporary account that receives SY tokens from the user, after which it transfers them to the SY program"
          ],
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "docs": [
            "PT liquidity account"
          ],
          "writable": true
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "tokenFeeTreasurySy",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "authorityVault",
          "docs": [
            "Vault-robot authority"
          ],
          "writable": true
        },
        {
          "name": "tokenSyEscrowVault",
          "docs": [
            "Vault-robot owned escrow account for SY"
          ],
          "writable": true
        },
        {
          "name": "mintYt",
          "writable": true
        },
        {
          "name": "mintPt",
          "writable": true
        },
        {
          "name": "addressLookupTableVault",
          "docs": [
            "ALT owned by vault"
          ]
        },
        {
          "name": "yieldPositionVault",
          "docs": [
            "Yield position owned by vault robot"
          ],
          "writable": true
        },
        {
          "name": "syProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "ytIn",
          "type": "u64"
        },
        {
          "name": "minSyOut",
          "type": "u64"
        }
      ],
      "returns": {
        "defined": {
          "name": "sellYtEvent"
        }
      }
    },
    {
      "name": "stageYtYield",
      "discriminator": [
        9
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "userYieldPosition",
          "docs": [
            "Position data for YT deposits, linked to vault"
          ],
          "writable": true
        },
        {
          "name": "yieldPosition",
          "docs": [
            "Yield position for the vault robot account"
          ],
          "writable": true
        },
        {
          "name": "syProgram"
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [],
      "returns": {
        "defined": {
          "name": "stageYieldEventV2"
        }
      }
    },
    {
      "name": "strip",
      "docs": [
        "Strip SY into PT + YT"
      ],
      "discriminator": [
        4
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "authority",
          "docs": [
            "This account owns the mints for PT & YT",
            "And owns the robot account with the SY program",
            "Needs to be mutable to be used in deposit_sy CPI"
          ],
          "writable": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "sySrc",
          "docs": [
            "Depositor's SY token account"
          ],
          "writable": true
        },
        {
          "name": "escrowSy",
          "docs": [
            "Temporary account owned by the vault for receiving SY tokens before depositing into the SY Program's escrow account"
          ],
          "writable": true
        },
        {
          "name": "ytDst",
          "docs": [
            "Final destination for receiving YT (probably, but not necessarily, owned by the depositor)"
          ],
          "writable": true
        },
        {
          "name": "ptDst",
          "docs": [
            "Final destination for receiving PT (probably, but not necessarily, owned by the depositor)"
          ],
          "writable": true
        },
        {
          "name": "mintYt",
          "writable": true
        },
        {
          "name": "mintPt",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "addressLookupTable",
          "docs": [
            "Address lookup table for vault"
          ]
        },
        {
          "name": "syProgram"
        },
        {
          "name": "yieldPosition",
          "docs": [
            "Vault-owned yield position account"
          ],
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ],
      "returns": {
        "defined": {
          "name": "stripEvent"
        }
      }
    },
    {
      "name": "tradePt",
      "discriminator": [
        17
      ],
      "accounts": [
        {
          "name": "trader",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenSyTrader",
          "docs": [
            "Trader's SY token account",
            "Mint is constrained by TokenProgram"
          ],
          "writable": true
        },
        {
          "name": "tokenPtTrader",
          "docs": [
            "Receiving PT token account",
            "Mint is constrained by TokenProgram"
          ],
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "docs": [
            "Account owned by market",
            "This is a temporary account that receives SY tokens from the user, after which it transfers them to the SY program"
          ],
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "docs": [
            "PT liquidity account"
          ],
          "writable": true
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "tokenFeeTreasurySy",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "netTraderPt",
          "type": "i64"
        },
        {
          "name": "syConstraint",
          "type": "i64"
        }
      ],
      "returns": {
        "defined": {
          "name": "tradePtEvent"
        }
      }
    },
    {
      "name": "withdrawYt",
      "discriminator": [
        8
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault",
          "docs": [
            "Vault that manages YT"
          ],
          "writable": true
        },
        {
          "name": "userYieldPosition",
          "docs": [
            "Position data for YT deposits, linked to vault"
          ],
          "writable": true
        },
        {
          "name": "ytDst",
          "docs": [
            "Withdrawer's YT token account"
          ],
          "writable": true
        },
        {
          "name": "escrowYt",
          "docs": [
            "Vault-owned escrow account for YT"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "authority"
        },
        {
          "name": "syProgram",
          "docs": [
            "The SY interface implementation for the vault's token"
          ]
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "yieldPosition",
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ],
      "returns": {
        "defined": {
          "name": "withdrawYtEventV2"
        }
      }
    },
    {
      "name": "wrapperBuyPt",
      "discriminator": [
        29
      ],
      "accounts": [
        {
          "name": "buyer",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenSyTrader",
          "writable": true
        },
        {
          "name": "tokenPtTrader",
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "writable": true
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "tokenFeeTreasurySy",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "ptAmount",
          "type": "u64"
        },
        {
          "name": "maxBaseAmount",
          "type": "u64"
        },
        {
          "name": "mintSyRemAccountsUntil",
          "type": "u8"
        }
      ]
    },
    {
      "name": "wrapperBuyYt",
      "discriminator": [
        31
      ],
      "accounts": [
        {
          "name": "buyer",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenSyTrader",
          "writable": true
        },
        {
          "name": "tokenYtTrader",
          "writable": true
        },
        {
          "name": "tokenPtTrader",
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "writable": true
        },
        {
          "name": "marketAddressLookupTable"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "vaultAuthority",
          "writable": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "tokenSyEscrowVault",
          "writable": true
        },
        {
          "name": "mintYt",
          "writable": true
        },
        {
          "name": "mintPt",
          "writable": true
        },
        {
          "name": "vaultAddressLookupTable"
        },
        {
          "name": "userYieldPosition",
          "writable": true
        },
        {
          "name": "yieldPosition",
          "writable": true
        },
        {
          "name": "escrowYt",
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "tokenFeeTreasurySy",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "ytOut",
          "type": "u64"
        },
        {
          "name": "maxBaseAmount",
          "type": "u64"
        },
        {
          "name": "mintSyAccountsLength",
          "type": "u8"
        }
      ]
    },
    {
      "name": "wrapperCollectInterest",
      "discriminator": [
        33
      ],
      "accounts": [
        {
          "name": "claimer",
          "writable": true,
          "signer": true
        },
        {
          "name": "authority",
          "docs": [
            "The authority owned by the vault for minting PT/YT"
          ],
          "writable": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "escrowSy",
          "writable": true
        },
        {
          "name": "syProgram"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "yieldPosition",
          "writable": true
        },
        {
          "name": "tokenSyDst",
          "writable": true
        },
        {
          "name": "treasurySyTokenAccount",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "redeemSyAccountsLength",
          "type": "u8"
        }
      ]
    },
    {
      "name": "wrapperMerge",
      "discriminator": [
        39
      ],
      "accounts": [
        {
          "name": "merger",
          "writable": true,
          "signer": true
        },
        {
          "name": "tokenSyMerger",
          "docs": [
            "Token account for SY owned by the merger"
          ],
          "writable": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "escrowSy",
          "writable": true
        },
        {
          "name": "tokenYtMerger",
          "writable": true
        },
        {
          "name": "tokenPtMerger",
          "writable": true
        },
        {
          "name": "mintYt",
          "writable": true
        },
        {
          "name": "mintPt",
          "writable": true
        },
        {
          "name": "authority",
          "writable": true
        },
        {
          "name": "vaultAddressLookupTable"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "vaultRobotYieldPosition",
          "writable": true
        },
        {
          "name": "syProgram"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amountPy",
          "type": "u64"
        },
        {
          "name": "redeemSyAccountsUntil",
          "type": "u8"
        }
      ]
    },
    {
      "name": "wrapperProvideLiquidity",
      "docs": [
        "Provide liquidity to a market starting with a base asset",
        "This instruction",
        "- deposits base asset for SY",
        "- strips a portion of the SY into PT & YT",
        "- provides the remaining SY with the PT into the market",
        "- keeps the YT",
        "",
        "# Arguments",
        "- `amount_base` - The amount of base asset to deposit",
        "- `min_lp_out` - The minimum amount of LP tokens to receive",
        "- `mint_base_accounts_until` - The index of the account to use for the base asset mint"
      ],
      "discriminator": [
        28
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "authority",
          "docs": [
            "The authority owned by the vault for minting PT/YT"
          ]
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "docs": [
            "PT liquidity account"
          ],
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "docs": [
            "SY token account owned by the market"
          ],
          "writable": true
        },
        {
          "name": "tokenLpDst",
          "writable": true
        },
        {
          "name": "mintLp",
          "writable": true
        },
        {
          "name": "tokenSyDepositor",
          "docs": [
            "Token account for SY owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "escrowSy",
          "writable": true
        },
        {
          "name": "tokenYtDepositor",
          "docs": [
            "Token account for YT owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "tokenPtDepositor",
          "docs": [
            "Token account for PT owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "mintYt",
          "writable": true
        },
        {
          "name": "mintPt",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "vaultAddressLookupTable"
        },
        {
          "name": "marketAddressLookupTable"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "userYieldPosition",
          "writable": true
        },
        {
          "name": "escrowYt",
          "writable": true
        },
        {
          "name": "tokenLpEscrow",
          "writable": true
        },
        {
          "name": "lpPosition",
          "writable": true
        },
        {
          "name": "vaultRobotYieldPosition",
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amountBase",
          "type": "u64"
        },
        {
          "name": "minLpOut",
          "type": "u64"
        },
        {
          "name": "mintBaseAccountsUntil",
          "type": "u8"
        }
      ]
    },
    {
      "name": "wrapperProvideLiquidityBase",
      "discriminator": [
        36
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "docs": [
            "PT liquidity account"
          ],
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "docs": [
            "SY token account owned by the market"
          ],
          "writable": true
        },
        {
          "name": "tokenLpDst",
          "writable": true
        },
        {
          "name": "mintLp",
          "writable": true
        },
        {
          "name": "tokenSyDepositor",
          "docs": [
            "Token account for SY owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "tokenPtDepositor",
          "docs": [
            "Token account for PT owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "marketAddressLookupTable"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "tokenFeeTreasurySy",
          "writable": true
        },
        {
          "name": "tokenLpEscrow",
          "writable": true
        },
        {
          "name": "lpPosition",
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amountBase",
          "type": "u64"
        },
        {
          "name": "minLpOut",
          "type": "u64"
        },
        {
          "name": "mintSyAccountsUntil",
          "type": "u8"
        },
        {
          "name": "externalPtToBuy",
          "type": "u64"
        },
        {
          "name": "externalSyConstraint",
          "type": "u64"
        }
      ]
    },
    {
      "name": "wrapperProvideLiquidityClassic",
      "discriminator": [
        37
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "docs": [
            "PT liquidity account"
          ],
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "docs": [
            "SY token account owned by the market"
          ],
          "writable": true
        },
        {
          "name": "tokenLpDst",
          "writable": true
        },
        {
          "name": "mintLp",
          "writable": true
        },
        {
          "name": "tokenSyDepositor",
          "docs": [
            "Token account for SY owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "tokenPtDepositor",
          "docs": [
            "Token account for PT owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "marketAddressLookupTable"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "tokenLpEscrow",
          "writable": true
        },
        {
          "name": "lpPosition",
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amountBase",
          "type": "u64"
        },
        {
          "name": "amountPt",
          "type": "u64"
        },
        {
          "name": "minLpOut",
          "type": "u64"
        },
        {
          "name": "mintSyAccountsUntil",
          "type": "u8"
        }
      ]
    },
    {
      "name": "wrapperSellPt",
      "discriminator": [
        30
      ],
      "accounts": [
        {
          "name": "seller",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenSyTrader",
          "writable": true
        },
        {
          "name": "tokenPtTrader",
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "writable": true
        },
        {
          "name": "addressLookupTable"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "tokenFeeTreasurySy",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amountPt",
          "type": "u64"
        },
        {
          "name": "minBaseAmount",
          "type": "u64"
        },
        {
          "name": "redeemSyRemAccountsUntil",
          "type": "u8"
        }
      ]
    },
    {
      "name": "wrapperSellYt",
      "discriminator": [
        32
      ],
      "accounts": [
        {
          "name": "seller",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenSyTrader",
          "writable": true
        },
        {
          "name": "tokenYtTrader",
          "writable": true
        },
        {
          "name": "tokenPtTrader",
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "writable": true
        },
        {
          "name": "marketAddressLookupTable"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "vaultAuthority",
          "writable": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "tokenSyEscrowVault",
          "writable": true
        },
        {
          "name": "mintYt",
          "writable": true
        },
        {
          "name": "mintPt",
          "writable": true
        },
        {
          "name": "vaultAddressLookupTable"
        },
        {
          "name": "yieldPosition",
          "writable": true
        },
        {
          "name": "tokenFeeTreasurySy",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "ytAmount",
          "type": "u64"
        },
        {
          "name": "minBaseAmount",
          "type": "u64"
        },
        {
          "name": "redeemSyAccountsUntil",
          "type": "u8"
        }
      ]
    },
    {
      "name": "wrapperStrip",
      "discriminator": [
        38
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "tokenSyDepositor",
          "docs": [
            "Token account for SY owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "escrowSy",
          "writable": true
        },
        {
          "name": "tokenYtDepositor",
          "writable": true
        },
        {
          "name": "tokenPtDepositor",
          "writable": true
        },
        {
          "name": "mintYt",
          "writable": true
        },
        {
          "name": "mintPt",
          "writable": true
        },
        {
          "name": "authority",
          "writable": true
        },
        {
          "name": "vaultAddressLookupTable"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "escrowYt",
          "writable": true
        },
        {
          "name": "userYieldPosition",
          "writable": true
        },
        {
          "name": "vaultRobotYieldPosition",
          "writable": true
        },
        {
          "name": "syProgram"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amountBase",
          "type": "u64"
        },
        {
          "name": "mintSyAccountsUntil",
          "type": "u8"
        }
      ]
    },
    {
      "name": "wrapperWithdrawLiquidity",
      "discriminator": [
        34
      ],
      "accounts": [
        {
          "name": "withdrawer",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "docs": [
            "PT liquidity account"
          ],
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "docs": [
            "SY token account owned by the market"
          ],
          "writable": true
        },
        {
          "name": "tokenLpSrc",
          "writable": true
        },
        {
          "name": "mintLp",
          "writable": true
        },
        {
          "name": "tokenSyWithdrawer",
          "docs": [
            "Token account for SY owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "tokenPtWithdrawer",
          "docs": [
            "Token account for PT owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "marketAddressLookupTable"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "tokenLpEscrow",
          "writable": true
        },
        {
          "name": "tokenFeeTreasurySy",
          "writable": true
        },
        {
          "name": "lpPosition",
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amountLp",
          "type": "u64"
        },
        {
          "name": "syConstraint",
          "type": "u64"
        },
        {
          "name": "redeemSyAccountsLength",
          "type": "u8"
        }
      ]
    },
    {
      "name": "wrapperWithdrawLiquidityClassic",
      "discriminator": [
        35
      ],
      "accounts": [
        {
          "name": "withdrawer",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tokenPtEscrow",
          "docs": [
            "PT liquidity account"
          ],
          "writable": true
        },
        {
          "name": "tokenSyEscrow",
          "docs": [
            "SY token account owned by the market"
          ],
          "writable": true
        },
        {
          "name": "tokenLpSrc",
          "writable": true
        },
        {
          "name": "mintLp",
          "writable": true
        },
        {
          "name": "tokenSyWithdrawer",
          "docs": [
            "Token account for SY owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "tokenPtWithdrawer",
          "docs": [
            "Token account for PT owned by the depositor"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "marketAddressLookupTable"
        },
        {
          "name": "syProgram"
        },
        {
          "name": "tokenLpEscrow",
          "writable": true
        },
        {
          "name": "lpPosition",
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amountLp",
          "type": "u64"
        },
        {
          "name": "redeemSyAccountsLength",
          "type": "u8"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "admin",
      "discriminator": [
        244,
        158,
        220,
        65,
        8,
        73,
        4,
        65
      ]
    },
    {
      "name": "lpPosition",
      "discriminator": [
        105,
        241,
        37,
        200,
        224,
        2,
        252,
        90
      ]
    },
    {
      "name": "marketTwo",
      "discriminator": [
        212,
        4,
        132,
        126,
        169,
        121,
        121,
        20
      ]
    },
    {
      "name": "vault",
      "discriminator": [
        211,
        8,
        232,
        43,
        2,
        152,
        117,
        119
      ]
    },
    {
      "name": "yieldTokenPosition",
      "discriminator": [
        227,
        92,
        146,
        49,
        29,
        85,
        71,
        94
      ]
    }
  ],
  "events": [
    {
      "name": "buyPtEvent",
      "discriminator": [
        62,
        132,
        50,
        43,
        219,
        234,
        221,
        167
      ]
    },
    {
      "name": "buyYtEvent",
      "discriminator": [
        172,
        181,
        56,
        183,
        219,
        24,
        130,
        64
      ]
    },
    {
      "name": "claimFarmEmissionsEvent",
      "discriminator": [
        184,
        10,
        159,
        144,
        144,
        194,
        166,
        92
      ]
    },
    {
      "name": "claimFarmEmissionsEventV2",
      "discriminator": [
        179,
        43,
        141,
        15,
        184,
        80,
        2,
        47
      ]
    },
    {
      "name": "collectEmissionEvent",
      "discriminator": [
        220,
        173,
        217,
        52,
        133,
        253,
        5,
        114
      ]
    },
    {
      "name": "collectEmissionEventV2",
      "discriminator": [
        235,
        35,
        233,
        149,
        215,
        221,
        100,
        66
      ]
    },
    {
      "name": "collectInterestEvent",
      "discriminator": [
        95,
        53,
        16,
        82,
        91,
        39,
        176,
        252
      ]
    },
    {
      "name": "collectInterestEventV2",
      "discriminator": [
        208,
        173,
        139,
        10,
        96,
        51,
        184,
        154
      ]
    },
    {
      "name": "depositLiquidityEvent",
      "discriminator": [
        169,
        84,
        67,
        174,
        222,
        138,
        16,
        123
      ]
    },
    {
      "name": "depositLpEvent",
      "discriminator": [
        118,
        20,
        164,
        8,
        112,
        130,
        3,
        62
      ]
    },
    {
      "name": "depositLpEventV2",
      "discriminator": [
        49,
        159,
        183,
        208,
        173,
        243,
        36,
        137
      ]
    },
    {
      "name": "depositYtEvent",
      "discriminator": [
        78,
        226,
        18,
        115,
        161,
        164,
        137,
        112
      ]
    },
    {
      "name": "depositYtEventV2",
      "discriminator": [
        24,
        10,
        201,
        118,
        79,
        178,
        237,
        243
      ]
    },
    {
      "name": "initLpPositionEvent",
      "discriminator": [
        7,
        245,
        31,
        228,
        42,
        49,
        201,
        192
      ]
    },
    {
      "name": "initializeYieldPositionEvent",
      "discriminator": [
        114,
        53,
        131,
        31,
        90,
        57,
        208,
        196
      ]
    },
    {
      "name": "marketCollectEmissionEvent",
      "discriminator": [
        125,
        129,
        15,
        139,
        232,
        244,
        82,
        67
      ]
    },
    {
      "name": "marketCollectEmissionEventV2",
      "discriminator": [
        173,
        158,
        156,
        218,
        11,
        243,
        57,
        54
      ]
    },
    {
      "name": "mergeEvent",
      "discriminator": [
        25,
        30,
        29,
        41,
        108,
        139,
        103,
        4
      ]
    },
    {
      "name": "sellPtEvent",
      "discriminator": [
        103,
        139,
        188,
        216,
        68,
        124,
        110,
        63
      ]
    },
    {
      "name": "sellYtEvent",
      "discriminator": [
        149,
        147,
        29,
        159,
        148,
        240,
        129,
        7
      ]
    },
    {
      "name": "stageYieldEvent",
      "discriminator": [
        248,
        92,
        96,
        80,
        238,
        94,
        91,
        195
      ]
    },
    {
      "name": "stageYieldEventV2",
      "discriminator": [
        23,
        7,
        36,
        198,
        5,
        216,
        217,
        189
      ]
    },
    {
      "name": "stripEvent",
      "discriminator": [
        114,
        189,
        26,
        143,
        143,
        50,
        197,
        89
      ]
    },
    {
      "name": "tradePtEvent",
      "discriminator": [
        159,
        225,
        96,
        81,
        255,
        227,
        233,
        174
      ]
    },
    {
      "name": "withdrawLiquidityEvent",
      "discriminator": [
        214,
        6,
        161,
        45,
        191,
        142,
        124,
        186
      ]
    },
    {
      "name": "withdrawLpEvent",
      "discriminator": [
        200,
        158,
        208,
        54,
        94,
        239,
        92,
        222
      ]
    },
    {
      "name": "withdrawLpEventV2",
      "discriminator": [
        17,
        227,
        69,
        154,
        238,
        182,
        86,
        33
      ]
    },
    {
      "name": "withdrawYtEvent",
      "discriminator": [
        190,
        66,
        234,
        53,
        4,
        207,
        221,
        17
      ]
    },
    {
      "name": "withdrawYtEventV2",
      "discriminator": [
        41,
        253,
        139,
        142,
        167,
        187,
        86,
        118
      ]
    },
    {
      "name": "wrapperBuyYtEvent",
      "discriminator": [
        148,
        95,
        21,
        54,
        99,
        16,
        15,
        40
      ]
    },
    {
      "name": "wrapperCollectInterestEvent",
      "discriminator": [
        177,
        39,
        249,
        210,
        72,
        117,
        16,
        192
      ]
    },
    {
      "name": "wrapperMergeEvent",
      "discriminator": [
        97,
        26,
        173,
        147,
        157,
        218,
        37,
        102
      ]
    },
    {
      "name": "wrapperProvideLiquidityBaseEvent",
      "discriminator": [
        60,
        121,
        164,
        93,
        220,
        13,
        142,
        197
      ]
    },
    {
      "name": "wrapperProvideLiquidityClassicEvent",
      "discriminator": [
        87,
        163,
        150,
        162,
        186,
        147,
        234,
        200
      ]
    },
    {
      "name": "wrapperProvideLiquidityEvent",
      "discriminator": [
        209,
        42,
        227,
        77,
        187,
        216,
        17,
        177
      ]
    },
    {
      "name": "wrapperSellYtEvent",
      "discriminator": [
        252,
        140,
        45,
        253,
        6,
        94,
        95,
        135
      ]
    },
    {
      "name": "wrapperStripEvent",
      "discriminator": [
        70,
        128,
        226,
        182,
        17,
        99,
        235,
        18
      ]
    },
    {
      "name": "wrapperWithdrawLiquidityClassicEvent",
      "discriminator": [
        18,
        154,
        212,
        39,
        36,
        23,
        158,
        124
      ]
    },
    {
      "name": "wrapperWithdrawLiquidityEvent",
      "discriminator": [
        52,
        32,
        180,
        241,
        36,
        221,
        72,
        167
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "invalidProxyAccount",
      "msg": "Invalid Proxy Account"
    },
    {
      "code": 6001,
      "name": "vaultExpired",
      "msg": "Vault is expired"
    },
    {
      "code": 6002,
      "name": "emissionIndexMustBeSequential",
      "msg": "Emission Index must be sequential"
    },
    {
      "code": 6003,
      "name": "amountLargerThanStaged",
      "msg": "Amount larger than staged"
    },
    {
      "code": 6004,
      "name": "mathOverflow",
      "msg": "Math overflow"
    },
    {
      "code": 6005,
      "name": "durationNegative",
      "msg": "Duration is negative"
    },
    {
      "code": 6006,
      "name": "farmDoesNotExist",
      "msg": "Farm does not exist"
    },
    {
      "code": 6007,
      "name": "lpSupplyMaximumExceeded",
      "msg": "Lp supply maximum exceeded"
    },
    {
      "code": 6008,
      "name": "vaultIsNotActive",
      "msg": "Vault has not started yet or has ended"
    },
    {
      "code": 6009,
      "name": "operationAmountTooSmall",
      "msg": "Operation amount too small"
    },
    {
      "code": 6010,
      "name": "strippingDisabled",
      "msg": "Stripping is disabled"
    },
    {
      "code": 6011,
      "name": "mergingDisabled",
      "msg": "Merging is disabled"
    },
    {
      "code": 6012,
      "name": "depositingYtDisabled",
      "msg": "Depositing YT is disabled"
    },
    {
      "code": 6013,
      "name": "withdrawingYtDisabled",
      "msg": "Withdrawing YT is disabled"
    },
    {
      "code": 6014,
      "name": "collectingInterestDisabled",
      "msg": "Collecting interest is disabled"
    },
    {
      "code": 6015,
      "name": "collectingEmissionsDisabled",
      "msg": "Collecting Emissions is disabled"
    },
    {
      "code": 6016,
      "name": "buyingPtDisabled",
      "msg": "Buying PT is disabled"
    },
    {
      "code": 6017,
      "name": "sellingPtDisabled",
      "msg": "Selling PT is disabled"
    },
    {
      "code": 6018,
      "name": "buyingYtDisabled",
      "msg": "Buying YT is disabled"
    },
    {
      "code": 6019,
      "name": "sellingYtDisabled",
      "msg": "Selling YT is disabled"
    },
    {
      "code": 6020,
      "name": "depositingLiquidityDisabled",
      "msg": "Depositing Liquidity is disabled"
    },
    {
      "code": 6021,
      "name": "withdrawingLiquidityDisabled",
      "msg": "Withdrawing Liquidity is disabled"
    },
    {
      "code": 6022,
      "name": "vaultInEmergencyMode",
      "msg": "Vault is in emergency mode"
    },
    {
      "code": 6023,
      "name": "farmAlreadyExists",
      "msg": "Farm already exists"
    },
    {
      "code": 6024,
      "name": "claimLimitExceeded",
      "msg": "Claim limit exceeded"
    },
    {
      "code": 6025,
      "name": "netBalanceChangeExceedsLimit",
      "msg": "Net balance change exceeds limit"
    },
    {
      "code": 6026,
      "name": "minSyOutNotMet",
      "msg": "Min SY out not met"
    },
    {
      "code": 6027,
      "name": "minPtOutNotMet",
      "msg": "Min PT out not met"
    },
    {
      "code": 6028,
      "name": "minLpOutNotMet",
      "msg": "Min LP out not met"
    }
  ],
  "types": [
    {
      "name": "admin",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "uberAdmin",
            "type": "pubkey"
          },
          {
            "name": "proposedUberAdmin",
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "principles",
            "type": {
              "defined": {
                "name": "principles"
              }
            }
          }
        ]
      }
    },
    {
      "name": "adminAction",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "setVaultStatus",
            "fields": [
              "u8"
            ]
          },
          {
            "name": "changeVaultBpsFee",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "changeVaultTreasuryTokenAccount",
            "fields": [
              "pubkey"
            ]
          },
          {
            "name": "changeEmissionTreasuryTokenAccount",
            "fields": [
              {
                "name": "emissionIndex",
                "type": "u16"
              },
              {
                "name": "newTokenAccount",
                "type": "pubkey"
              }
            ]
          },
          {
            "name": "changeMinOperationSize",
            "fields": [
              {
                "name": "isStrip",
                "type": "bool"
              },
              {
                "name": "newSize",
                "type": "u64"
              }
            ]
          },
          {
            "name": "changeEmissionBpsFee",
            "fields": [
              {
                "name": "emissionIndex",
                "type": "u16"
              },
              {
                "name": "newFeeBps",
                "type": "u16"
              }
            ]
          },
          {
            "name": "changeCpiAccounts",
            "fields": [
              {
                "name": "cpiAccounts",
                "type": {
                  "defined": {
                    "name": "cpiAccounts"
                  }
                }
              }
            ]
          },
          {
            "name": "changeClaimLimits",
            "fields": [
              {
                "name": "maxClaimAmountPerWindow",
                "type": "u64"
              },
              {
                "name": "claimWindowDurationSeconds",
                "type": "u32"
              }
            ]
          },
          {
            "name": "changeMaxPySupply",
            "fields": [
              {
                "name": "newMaxPySupply",
                "type": "u64"
              }
            ]
          },
          {
            "name": "changeAddressLookupTable",
            "fields": [
              "pubkey"
            ]
          },
          {
            "name": "removeVaultEmission",
            "fields": [
              "u8"
            ]
          }
        ]
      }
    },
    {
      "name": "amount",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "all"
          },
          {
            "name": "some",
            "fields": [
              "u64"
            ]
          }
        ]
      }
    },
    {
      "name": "buyPtEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "buyer",
            "type": "pubkey"
          },
          {
            "name": "baseAmountIn",
            "type": "u64"
          },
          {
            "name": "ptAmountOut",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "buyYtEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "trader",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "tokenSyTrader",
            "type": "pubkey"
          },
          {
            "name": "tokenYtTrader",
            "type": "pubkey"
          },
          {
            "name": "tokenPtTrader",
            "type": "pubkey"
          },
          {
            "name": "tokenSyEscrow",
            "type": "pubkey"
          },
          {
            "name": "tokenPtEscrow",
            "type": "pubkey"
          },
          {
            "name": "maxSyIn",
            "type": "u64"
          },
          {
            "name": "ytOut",
            "type": "u64"
          },
          {
            "name": "syExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "syToStrip",
            "type": "u64"
          },
          {
            "name": "syBorrowed",
            "type": "u64"
          },
          {
            "name": "ptOut",
            "type": "u64"
          },
          {
            "name": "syRepaid",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "claimFarmEmissionsEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "lpPosition",
            "type": "pubkey"
          },
          {
            "name": "tokenDst",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "tokenFarm",
            "type": "pubkey"
          },
          {
            "name": "farmIndex",
            "type": "u8"
          },
          {
            "name": "amountClaimed",
            "type": "u64"
          },
          {
            "name": "remainingStaged",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "claimFarmEmissionsEventV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "lpPosition",
            "type": "pubkey"
          },
          {
            "name": "tokenDst",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "tokenFarm",
            "type": "pubkey"
          },
          {
            "name": "farmIndex",
            "type": "u8"
          },
          {
            "name": "amountClaimed",
            "type": "u64"
          },
          {
            "name": "remainingStaged",
            "type": "u64"
          },
          {
            "name": "emissions",
            "type": {
              "defined": {
                "name": "personalYieldTrackers"
              }
            }
          },
          {
            "name": "farms",
            "type": {
              "defined": {
                "name": "personalYieldTrackers"
              }
            }
          }
        ]
      }
    },
    {
      "name": "claimLimits",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "claimWindowStartTimestamp",
            "type": "u32"
          },
          {
            "name": "totalClaimAmountInWindow",
            "type": "u64"
          },
          {
            "name": "maxClaimAmountPerWindow",
            "type": "u64"
          },
          {
            "name": "claimWindowDurationSeconds",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "collectEmissionEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "position",
            "type": "pubkey"
          },
          {
            "name": "emissionIndex",
            "type": "u16"
          },
          {
            "name": "amountToUser",
            "type": "u64"
          },
          {
            "name": "amountToTreasury",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "collectEmissionEventV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "position",
            "type": "pubkey"
          },
          {
            "name": "emissionIndex",
            "type": "u16"
          },
          {
            "name": "amountToUser",
            "type": "u64"
          },
          {
            "name": "amountToTreasury",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          },
          {
            "name": "userInterest",
            "type": {
              "defined": {
                "name": "yieldTokenTracker"
              }
            }
          },
          {
            "name": "userEmissions",
            "type": {
              "vec": {
                "defined": {
                  "name": "yieldTokenTracker"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "collectInterestEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "userYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "amountToUser",
            "type": "u64"
          },
          {
            "name": "amountToTreasury",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "collectInterestEventV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "userYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "amountToUser",
            "type": "u64"
          },
          {
            "name": "amountToTreasury",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          },
          {
            "name": "userInterest",
            "type": {
              "defined": {
                "name": "yieldTokenTracker"
              }
            }
          },
          {
            "name": "userEmissions",
            "type": {
              "vec": {
                "defined": {
                  "name": "yieldTokenTracker"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "collectTreasuryEmissionKind",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "yieldPosition"
          },
          {
            "name": "treasuryEmission"
          }
        ]
      }
    },
    {
      "name": "collectTreasuryInterestKind",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "yieldPosition"
          },
          {
            "name": "treasuryInterest"
          }
        ]
      }
    },
    {
      "name": "cpiAccounts",
      "docs": [
        "Account lists for validating CPI calls to the SY program"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "getSyState",
            "docs": [
              "Fetch SY state"
            ],
            "type": {
              "vec": {
                "defined": {
                  "name": "cpiInterfaceContext"
                }
              }
            }
          },
          {
            "name": "depositSy",
            "docs": [
              "Deposit SY into personal account owned by vault"
            ],
            "type": {
              "vec": {
                "defined": {
                  "name": "cpiInterfaceContext"
                }
              }
            }
          },
          {
            "name": "withdrawSy",
            "docs": [
              "Withdraw SY from personal account owned by vault"
            ],
            "type": {
              "vec": {
                "defined": {
                  "name": "cpiInterfaceContext"
                }
              }
            }
          },
          {
            "name": "claimEmission",
            "docs": [
              "Settle rewards for vault to accounts owned by the vault"
            ],
            "type": {
              "vec": {
                "vec": {
                  "defined": {
                    "name": "cpiInterfaceContext"
                  }
                }
              }
            }
          },
          {
            "name": "getPositionState",
            "docs": [
              "Get personal yield position"
            ],
            "type": {
              "vec": {
                "defined": {
                  "name": "cpiInterfaceContext"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "cpiInterfaceContext",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "altIndex",
            "docs": [
              "Address-lookup-table index"
            ],
            "type": "u8"
          },
          {
            "name": "isSigner",
            "type": "bool"
          },
          {
            "name": "isWritable",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "depositLiquidityEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "depositor",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "tokenPtSrc",
            "type": "pubkey"
          },
          {
            "name": "tokenSySrc",
            "type": "pubkey"
          },
          {
            "name": "tokenPtEscrow",
            "type": "pubkey"
          },
          {
            "name": "tokenSyEscrow",
            "type": "pubkey"
          },
          {
            "name": "tokenLpDst",
            "type": "pubkey"
          },
          {
            "name": "mintLp",
            "type": "pubkey"
          },
          {
            "name": "ptIntent",
            "type": "u64"
          },
          {
            "name": "syIntent",
            "type": "u64"
          },
          {
            "name": "ptIn",
            "type": "u64"
          },
          {
            "name": "syIn",
            "type": "u64"
          },
          {
            "name": "lpOut",
            "type": "u64"
          },
          {
            "name": "newLpSupply",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "depositLpEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "lpPosition",
            "type": "pubkey"
          },
          {
            "name": "tokenLpSrc",
            "type": "pubkey"
          },
          {
            "name": "tokenLpEscrow",
            "type": "pubkey"
          },
          {
            "name": "mintLp",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "newLpBalance",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "depositLpEventV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "lpPosition",
            "type": "pubkey"
          },
          {
            "name": "tokenLpSrc",
            "type": "pubkey"
          },
          {
            "name": "tokenLpEscrow",
            "type": "pubkey"
          },
          {
            "name": "mintLp",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "newLpBalance",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          },
          {
            "name": "emissions",
            "type": {
              "defined": {
                "name": "personalYieldTrackers"
              }
            }
          },
          {
            "name": "farms",
            "type": {
              "defined": {
                "name": "personalYieldTrackers"
              }
            }
          }
        ]
      }
    },
    {
      "name": "depositYtEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "depositor",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "userYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "vaultYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "ytSrc",
            "type": "pubkey"
          },
          {
            "name": "escrowYt",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "syExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "userYtBalanceAfter",
            "type": "u64"
          },
          {
            "name": "vaultYtBalanceAfter",
            "type": "u64"
          },
          {
            "name": "userStagedYield",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "depositYtEventV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "depositor",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "userYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "vaultYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "ytSrc",
            "type": "pubkey"
          },
          {
            "name": "escrowYt",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "syExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "userYtBalanceAfter",
            "type": "u64"
          },
          {
            "name": "vaultYtBalanceAfter",
            "type": "u64"
          },
          {
            "name": "userStagedYield",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          },
          {
            "name": "userInterest",
            "type": {
              "defined": {
                "name": "yieldTokenTracker"
              }
            }
          },
          {
            "name": "userEmissions",
            "type": {
              "vec": {
                "defined": {
                  "name": "yieldTokenTracker"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "emissionInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenAccount",
            "docs": [
              "The token account for the emission where the vault authority is the authority"
            ],
            "type": "pubkey"
          },
          {
            "name": "initialIndex",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "lastSeenIndex",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "finalIndex",
            "docs": [
              "The final index is used to track the last claimable index after the vault expires"
            ],
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "treasuryTokenAccount",
            "docs": [
              "The treasury token account for this reward"
            ],
            "type": "pubkey"
          },
          {
            "name": "feeBps",
            "docs": [
              "The fee taken from emission collecting"
            ],
            "type": "u16"
          },
          {
            "name": "treasuryEmission",
            "docs": [
              "The lambo fund"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "farmEmission",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "Mint for the emission token"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenRate",
            "docs": [
              "Rate at which the emission token is emitted per second"
            ],
            "type": "u64"
          },
          {
            "name": "expiryTimestamp",
            "docs": [
              "Expiration timestamp for the emission token"
            ],
            "type": "u32"
          },
          {
            "name": "index",
            "docs": [
              "Index for converting LP shares into earned emissions"
            ],
            "type": {
              "defined": {
                "name": "number"
              }
            }
          }
        ]
      }
    },
    {
      "name": "initLpPositionEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "feePayer",
            "type": "pubkey"
          },
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "lpPosition",
            "type": "pubkey"
          },
          {
            "name": "numEmissionTrackers",
            "type": "u8"
          },
          {
            "name": "numFarmEmissions",
            "type": "u8"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "initializeYieldPositionEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "yieldPosition",
            "type": "pubkey"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "liquidityNetBalanceLimits",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "windowStartTimestamp",
            "type": "u32"
          },
          {
            "name": "windowStartNetBalance",
            "type": "u64"
          },
          {
            "name": "maxNetBalanceChangeNegativePercentage",
            "docs": [
              "Maximum allowed negative change in basis points (10000 = 100%)"
            ],
            "type": "u16"
          },
          {
            "name": "maxNetBalanceChangePositivePercentage",
            "docs": [
              "Maximum allowed positive change in basis points (10000 = 100%)",
              "Using u32 to allow for very large increases (up to ~429,496%)"
            ],
            "type": "u32"
          },
          {
            "name": "windowDurationSeconds",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "lpFarm",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "lastSeenTimestamp",
            "type": "u32"
          },
          {
            "name": "farmEmissions",
            "type": {
              "vec": {
                "defined": {
                  "name": "farmEmission"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "lpPosition",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "docs": [
              "Link to address that owns this position"
            ],
            "type": "pubkey"
          },
          {
            "name": "market",
            "docs": [
              "Link to market that manages the LP"
            ],
            "type": "pubkey"
          },
          {
            "name": "lpBalance",
            "docs": [
              "Track the LP balance of the user here"
            ],
            "type": "u64"
          },
          {
            "name": "emissions",
            "docs": [
              "Tracker for emissions earned (paid in emission tokens)"
            ],
            "type": {
              "defined": {
                "name": "personalYieldTrackers"
              }
            }
          },
          {
            "name": "farms",
            "type": {
              "defined": {
                "name": "personalYieldTrackers"
              }
            }
          }
        ]
      }
    },
    {
      "name": "marketAdminAction",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "setStatus",
            "fields": [
              "u8"
            ]
          },
          {
            "name": "setMaxLpSupply",
            "fields": [
              "u64"
            ]
          },
          {
            "name": "changeTreasuryTradeSyBpsFee",
            "fields": [
              "u16"
            ]
          },
          {
            "name": "changeLnFeeRateRoot",
            "fields": [
              "f64"
            ]
          },
          {
            "name": "changeRateScalarRoot",
            "fields": [
              "f64"
            ]
          },
          {
            "name": "changeCpiAccounts",
            "fields": [
              {
                "name": "cpiAccounts",
                "type": {
                  "defined": {
                    "name": "cpiAccounts"
                  }
                }
              }
            ]
          },
          {
            "name": "changeLiquidityNetBalanceLimits",
            "fields": [
              {
                "name": "maxNetBalanceChangeNegativePercentage",
                "type": "u16"
              },
              {
                "name": "maxNetBalanceChangePositivePercentage",
                "type": "u32"
              },
              {
                "name": "windowDurationSeconds",
                "type": "u32"
              }
            ]
          },
          {
            "name": "changeAddressLookupTable",
            "fields": [
              "pubkey"
            ]
          },
          {
            "name": "removeMarketEmission",
            "fields": [
              "u8"
            ]
          }
        ]
      }
    },
    {
      "name": "marketCollectEmissionEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "lpPosition",
            "type": "pubkey"
          },
          {
            "name": "tokenEmissionEscrow",
            "type": "pubkey"
          },
          {
            "name": "tokenEmissionDst",
            "type": "pubkey"
          },
          {
            "name": "emissionIndex",
            "type": "u16"
          },
          {
            "name": "amountCollected",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "marketCollectEmissionEventV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "lpPosition",
            "type": "pubkey"
          },
          {
            "name": "tokenEmissionEscrow",
            "type": "pubkey"
          },
          {
            "name": "tokenEmissionDst",
            "type": "pubkey"
          },
          {
            "name": "emissionIndex",
            "type": "u16"
          },
          {
            "name": "amountCollected",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          },
          {
            "name": "emissions",
            "type": {
              "defined": {
                "name": "personalYieldTrackers"
              }
            }
          },
          {
            "name": "farms",
            "type": {
              "defined": {
                "name": "personalYieldTrackers"
              }
            }
          }
        ]
      }
    },
    {
      "name": "marketEmission",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenEscrow",
            "docs": [
              "Escrow account that receives the emissions from the SY program",
              "And then passes them through to the user"
            ],
            "type": "pubkey"
          },
          {
            "name": "lpShareIndex",
            "docs": [
              "Index for converting LP shares into earned emissions"
            ],
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "lastSeenStaged",
            "docs": [
              "The difference between the staged amount and collected emission amount"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "marketEmissions",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "trackers",
            "type": {
              "vec": {
                "defined": {
                  "name": "marketEmission"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "marketFinancials",
      "docs": [
        "Financial parameters for the market"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "expirationTs",
            "docs": [
              "Expiration timestamp, which is copied from the vault associated with the PT"
            ],
            "type": "u64"
          },
          {
            "name": "ptBalance",
            "docs": [
              "Balance of PT in the market",
              "This amount is tracked separately to prevent bugs from token transfers directly to the market"
            ],
            "type": "u64"
          },
          {
            "name": "syBalance",
            "docs": [
              "Balance of SY in the market",
              "This amount is tracked separately to prevent bugs from token transfers directly to the market"
            ],
            "type": "u64"
          },
          {
            "name": "lnFeeRateRoot",
            "docs": [
              "Initial log of fee rate, which decreases over time"
            ],
            "type": "f64"
          },
          {
            "name": "lastLnImpliedRate",
            "docs": [
              "Last seen log of implied rate (APY) for PT",
              "Used to maintain continuity of the APY between trades over time"
            ],
            "type": "f64"
          },
          {
            "name": "rateScalarRoot",
            "docs": [
              "Initial rate scalar, which increases over time"
            ],
            "type": "f64"
          }
        ]
      }
    },
    {
      "name": "marketTwo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "addressLookupTable",
            "docs": [
              "Address to ALT"
            ],
            "type": "pubkey"
          },
          {
            "name": "mintPt",
            "docs": [
              "Mint of the vault's PT token"
            ],
            "type": "pubkey"
          },
          {
            "name": "mintSy",
            "docs": [
              "Mint of the SY program's SY token"
            ],
            "type": "pubkey"
          },
          {
            "name": "vault",
            "docs": [
              "Link to yield-stripping vault"
            ],
            "type": "pubkey"
          },
          {
            "name": "mintLp",
            "docs": [
              "Mint for the market's LP tokens"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenLpEscrow",
            "docs": [
              "Holds the LP tokens that are earning emissions",
              "This is where LP holders \"stake\" their LP tokens"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenPtEscrow",
            "docs": [
              "Token account that holds PT liquidity"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenSyEscrow",
            "docs": [
              "Pass-through token account for SY moving from the depositor to the SY program"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenFeeTreasurySy",
            "docs": [
              "Token account that holds SY fees from trade_pt"
            ],
            "type": "pubkey"
          },
          {
            "name": "feeTreasurySyBps",
            "docs": [
              "Fee treasury SY BPS"
            ],
            "type": "u16"
          },
          {
            "name": "selfAddress",
            "docs": [
              "Authority for CPI calls owned by the market struct"
            ],
            "type": "pubkey"
          },
          {
            "name": "signerBump",
            "docs": [
              "Bump for signing the PDA"
            ],
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          },
          {
            "name": "statusFlags",
            "type": "u8"
          },
          {
            "name": "syProgram",
            "docs": [
              "Link to the SY program ID"
            ],
            "type": "pubkey"
          },
          {
            "name": "financials",
            "type": {
              "defined": {
                "name": "marketFinancials"
              }
            }
          },
          {
            "name": "emissions",
            "type": {
              "defined": {
                "name": "marketEmissions"
              }
            }
          },
          {
            "name": "lpFarm",
            "type": {
              "defined": {
                "name": "lpFarm"
              }
            }
          },
          {
            "name": "maxLpSupply",
            "type": "u64"
          },
          {
            "name": "lpEscrowAmount",
            "type": "u64"
          },
          {
            "name": "cpiAccounts",
            "docs": [
              "Record of CPI accounts"
            ],
            "type": {
              "defined": {
                "name": "cpiAccounts"
              }
            }
          },
          {
            "name": "isCurrentFlashSwap",
            "type": "bool"
          },
          {
            "name": "liquidityNetBalanceLimits",
            "type": {
              "defined": {
                "name": "liquidityNetBalanceLimits"
              }
            }
          },
          {
            "name": "seedId",
            "docs": [
              "Unique seed id for the market"
            ],
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          }
        ]
      }
    },
    {
      "name": "mergeEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "syDst",
            "type": "pubkey"
          },
          {
            "name": "escrowSy",
            "type": "pubkey"
          },
          {
            "name": "ytSrc",
            "type": "pubkey"
          },
          {
            "name": "ptSrc",
            "type": "pubkey"
          },
          {
            "name": "mintYt",
            "type": "pubkey"
          },
          {
            "name": "mintPt",
            "type": "pubkey"
          },
          {
            "name": "yieldPosition",
            "type": "pubkey"
          },
          {
            "name": "amountPyIn",
            "type": "u64"
          },
          {
            "name": "amountSyOut",
            "type": "u64"
          },
          {
            "name": "syExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "ptRedemptionRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "totalSyInEscrow",
            "type": "u64"
          },
          {
            "name": "ptSupply",
            "type": "u64"
          },
          {
            "name": "ytBalance",
            "type": "u64"
          },
          {
            "name": "syForPt",
            "type": "u64"
          },
          {
            "name": "isVaultActive",
            "type": "bool"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "number",
      "docs": [
        "High precision number, stored as 4 u64 words in little endian"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "array": [
              "u64",
              4
            ]
          }
        ]
      }
    },
    {
      "name": "personalYieldTracker",
      "docs": [
        "Generic tracker for interest and emissions earned by deposits"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "lastSeenIndex",
            "docs": [
              "The index is the per-share value of the SY token",
              "Note that the YT balance must be converted to the equivalent SY balance"
            ],
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "staged",
            "docs": [
              "Staged tokens that may be withdrawn"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "personalYieldTrackers",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "trackers",
            "type": {
              "vec": {
                "defined": {
                  "name": "personalYieldTracker"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "principleDetails",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "administrators",
            "type": {
              "vec": "pubkey"
            }
          }
        ]
      }
    },
    {
      "name": "principles",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "marginfiStandard",
            "type": {
              "defined": {
                "name": "principleDetails"
              }
            }
          },
          {
            "name": "collectTreasury",
            "type": {
              "defined": {
                "name": "principleDetails"
              }
            }
          },
          {
            "name": "kaminoLendStandard",
            "type": {
              "defined": {
                "name": "principleDetails"
              }
            }
          },
          {
            "name": "exponentCore",
            "type": {
              "defined": {
                "name": "principleDetails"
              }
            }
          },
          {
            "name": "changeStatusFlags",
            "type": {
              "defined": {
                "name": "principleDetails"
              }
            }
          },
          {
            "name": "jitoRestaking",
            "type": {
              "defined": {
                "name": "principleDetails"
              }
            }
          }
        ]
      }
    },
    {
      "name": "sellPtEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "seller",
            "type": "pubkey"
          },
          {
            "name": "ptAmountIn",
            "type": "u64"
          },
          {
            "name": "baseAmountOut",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "sellYtEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "trader",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "tokenYtTrader",
            "type": "pubkey"
          },
          {
            "name": "tokenPtTrader",
            "type": "pubkey"
          },
          {
            "name": "tokenSyTrader",
            "type": "pubkey"
          },
          {
            "name": "tokenSyEscrow",
            "type": "pubkey"
          },
          {
            "name": "tokenPtEscrow",
            "type": "pubkey"
          },
          {
            "name": "amountYtIn",
            "type": "u64"
          },
          {
            "name": "amountSyReceivedFromMerge",
            "type": "u64"
          },
          {
            "name": "amountSySpentBuyingPt",
            "type": "u64"
          },
          {
            "name": "amountSyOut",
            "type": "u64"
          },
          {
            "name": "ptBorrowedAndRepaid",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "stageYieldEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "payer",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "userYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "vaultYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "syExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "userYtBalance",
            "type": "u64"
          },
          {
            "name": "userStagedYield",
            "type": "u64"
          },
          {
            "name": "userStagedEmissions",
            "type": {
              "vec": "u64"
            }
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "stageYieldEventV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "payer",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "userYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "vaultYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "syExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "userYtBalance",
            "type": "u64"
          },
          {
            "name": "userStagedYield",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          },
          {
            "name": "userInterest",
            "type": {
              "defined": {
                "name": "yieldTokenTracker"
              }
            }
          },
          {
            "name": "userEmissions",
            "type": {
              "vec": {
                "defined": {
                  "name": "yieldTokenTracker"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "stripEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "depositor",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "sySrc",
            "type": "pubkey"
          },
          {
            "name": "escrowSy",
            "type": "pubkey"
          },
          {
            "name": "ytDst",
            "type": "pubkey"
          },
          {
            "name": "ptDst",
            "type": "pubkey"
          },
          {
            "name": "mintYt",
            "type": "pubkey"
          },
          {
            "name": "mintPt",
            "type": "pubkey"
          },
          {
            "name": "yieldPosition",
            "type": "pubkey"
          },
          {
            "name": "amountSyIn",
            "type": "u64"
          },
          {
            "name": "amountPyOut",
            "type": "u64"
          },
          {
            "name": "syExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "totalSyInEscrow",
            "type": "u64"
          },
          {
            "name": "ptSupply",
            "type": "u64"
          },
          {
            "name": "ytBalance",
            "type": "u64"
          },
          {
            "name": "allTimeHighSyExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "syForPt",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "tradePtEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "trader",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "tokenSyTrader",
            "type": "pubkey"
          },
          {
            "name": "tokenPtTrader",
            "type": "pubkey"
          },
          {
            "name": "tokenSyEscrow",
            "type": "pubkey"
          },
          {
            "name": "tokenPtEscrow",
            "type": "pubkey"
          },
          {
            "name": "netTraderPt",
            "type": "i64"
          },
          {
            "name": "netTraderSy",
            "type": "i64"
          },
          {
            "name": "feeSy",
            "type": "u64"
          },
          {
            "name": "syExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "vault",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "syProgram",
            "docs": [
              "Link to SY program"
            ],
            "type": "pubkey"
          },
          {
            "name": "mintSy",
            "docs": [
              "Mint for SY token"
            ],
            "type": "pubkey"
          },
          {
            "name": "mintYt",
            "docs": [
              "Mint for the vault-specific YT token"
            ],
            "type": "pubkey"
          },
          {
            "name": "mintPt",
            "docs": [
              "Mint for the vault-specific PT token"
            ],
            "type": "pubkey"
          },
          {
            "name": "escrowYt",
            "docs": [
              "Escrow account for holding deposited YT"
            ],
            "type": "pubkey"
          },
          {
            "name": "escrowSy",
            "docs": [
              "Escrow account that holds temporary SY tokens",
              "As an interchange between users and the SY program"
            ],
            "type": "pubkey"
          },
          {
            "name": "yieldPosition",
            "docs": [
              "Link to a vault-owned YT position",
              "This account collects yield from all \"unstaked\" YT"
            ],
            "type": "pubkey"
          },
          {
            "name": "addressLookupTable",
            "docs": [
              "Address lookup table key for vault"
            ],
            "type": "pubkey"
          },
          {
            "name": "startTs",
            "docs": [
              "start timestamp"
            ],
            "type": "u32"
          },
          {
            "name": "duration",
            "docs": [
              "seconds duration"
            ],
            "type": "u32"
          },
          {
            "name": "signerSeed",
            "docs": [
              "Seed for CPI signing"
            ],
            "type": "pubkey"
          },
          {
            "name": "authority",
            "docs": [
              "Authority for CPI signing"
            ],
            "type": "pubkey"
          },
          {
            "name": "signerBump",
            "docs": [
              "bump for signer authority PDA"
            ],
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          },
          {
            "name": "lastSeenSyExchangeRate",
            "docs": [
              "Last seen SY exchange rate",
              "This continues to be updated even after vault maturity to track SY appreciation for treasury collection"
            ],
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "allTimeHighSyExchangeRate",
            "docs": [
              "This is the all time high exchange rate for SY"
            ],
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "finalSyExchangeRate",
            "docs": [
              "This is the exchange rate for SY when the vault expires"
            ],
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "totalSyInEscrow",
            "docs": [
              "How much SY is held in escrow"
            ],
            "type": "u64"
          },
          {
            "name": "syForPt",
            "docs": [
              "The total SY set aside to back the PT holders",
              "This value is updated on every operation that touches the PT supply or the last seen exchange rate"
            ],
            "type": "u64"
          },
          {
            "name": "ptSupply",
            "docs": [
              "Total supply of PT"
            ],
            "type": "u64"
          },
          {
            "name": "treasurySy",
            "docs": [
              "Amount of SY staged for the treasury"
            ],
            "type": "u64"
          },
          {
            "name": "uncollectedSy",
            "docs": [
              "SY that has been earned by YT, but not yet collected"
            ],
            "type": "u64"
          },
          {
            "name": "treasurySyTokenAccount",
            "type": "pubkey"
          },
          {
            "name": "interestBpsFee",
            "type": "u16"
          },
          {
            "name": "minOpSizeStrip",
            "type": "u64"
          },
          {
            "name": "minOpSizeMerge",
            "type": "u64"
          },
          {
            "name": "status",
            "type": "u8"
          },
          {
            "name": "emissions",
            "type": {
              "vec": {
                "defined": {
                  "name": "emissionInfo"
                }
              }
            }
          },
          {
            "name": "cpiAccounts",
            "type": {
              "defined": {
                "name": "cpiAccounts"
              }
            }
          },
          {
            "name": "claimLimits",
            "type": {
              "defined": {
                "name": "claimLimits"
              }
            }
          },
          {
            "name": "maxPySupply",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "withdrawLiquidityEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "withdrawer",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "tokenPtDst",
            "type": "pubkey"
          },
          {
            "name": "tokenSyDst",
            "type": "pubkey"
          },
          {
            "name": "tokenPtEscrow",
            "type": "pubkey"
          },
          {
            "name": "tokenSyEscrow",
            "type": "pubkey"
          },
          {
            "name": "tokenLpSrc",
            "type": "pubkey"
          },
          {
            "name": "mintLp",
            "type": "pubkey"
          },
          {
            "name": "lpIn",
            "type": "u64"
          },
          {
            "name": "ptOut",
            "type": "u64"
          },
          {
            "name": "syOut",
            "type": "u64"
          },
          {
            "name": "newLpSupply",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "withdrawLpEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "lpPosition",
            "type": "pubkey"
          },
          {
            "name": "mintLp",
            "type": "pubkey"
          },
          {
            "name": "tokenLpDst",
            "type": "pubkey"
          },
          {
            "name": "tokenLpEscrow",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "newLpBalance",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "withdrawLpEventV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "lpPosition",
            "type": "pubkey"
          },
          {
            "name": "mintLp",
            "type": "pubkey"
          },
          {
            "name": "tokenLpDst",
            "type": "pubkey"
          },
          {
            "name": "tokenLpEscrow",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "newLpBalance",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          },
          {
            "name": "emissions",
            "type": {
              "defined": {
                "name": "personalYieldTrackers"
              }
            }
          },
          {
            "name": "farms",
            "type": {
              "defined": {
                "name": "personalYieldTrackers"
              }
            }
          }
        ]
      }
    },
    {
      "name": "withdrawYtEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "userYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "vaultYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "ytDst",
            "type": "pubkey"
          },
          {
            "name": "escrowYt",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "syExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "userYtBalanceAfter",
            "type": "u64"
          },
          {
            "name": "vaultYtBalanceAfter",
            "type": "u64"
          },
          {
            "name": "userStagedYield",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "withdrawYtEventV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "userYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "vaultYieldPosition",
            "type": "pubkey"
          },
          {
            "name": "ytDst",
            "type": "pubkey"
          },
          {
            "name": "escrowYt",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "syExchangeRate",
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "userYtBalanceAfter",
            "type": "u64"
          },
          {
            "name": "vaultYtBalanceAfter",
            "type": "u64"
          },
          {
            "name": "userStagedYield",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          },
          {
            "name": "userInterest",
            "type": {
              "defined": {
                "name": "yieldTokenTracker"
              }
            }
          },
          {
            "name": "userEmissions",
            "type": {
              "vec": {
                "defined": {
                  "name": "yieldTokenTracker"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "wrapperBuyYtEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "buyer",
            "type": "pubkey"
          },
          {
            "name": "ytOutAmount",
            "type": "u64"
          },
          {
            "name": "baseInAmount",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "wrapperCollectInterestEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "depositor",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "amountBaseCollected",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "wrapperMergeEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "userAddress",
            "type": "pubkey"
          },
          {
            "name": "vaultAddress",
            "type": "pubkey"
          },
          {
            "name": "amountPyIn",
            "type": "u64"
          },
          {
            "name": "amountSyRedeemed",
            "type": "u64"
          },
          {
            "name": "amountBaseOut",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "wrapperProvideLiquidityBaseEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "userAddress",
            "type": "pubkey"
          },
          {
            "name": "marketAddress",
            "type": "pubkey"
          },
          {
            "name": "amountBaseIn",
            "type": "u64"
          },
          {
            "name": "tradeAmountPtOut",
            "type": "u64"
          },
          {
            "name": "tradeAmountSyIn",
            "type": "u64"
          },
          {
            "name": "amountLpOut",
            "type": "u64"
          },
          {
            "name": "lpPrice",
            "type": "f64"
          }
        ]
      }
    },
    {
      "name": "wrapperProvideLiquidityClassicEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "userAddress",
            "type": "pubkey"
          },
          {
            "name": "marketAddress",
            "type": "pubkey"
          },
          {
            "name": "amountBaseIn",
            "type": "u64"
          },
          {
            "name": "amountPtIn",
            "type": "u64"
          },
          {
            "name": "amountLpOut",
            "type": "u64"
          },
          {
            "name": "lpPrice",
            "type": "f64"
          }
        ]
      }
    },
    {
      "name": "wrapperProvideLiquidityEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "userAddress",
            "type": "pubkey"
          },
          {
            "name": "marketAddress",
            "type": "pubkey"
          },
          {
            "name": "amountBaseIn",
            "type": "u64"
          },
          {
            "name": "amountLpOut",
            "type": "u64"
          },
          {
            "name": "amountYtOut",
            "type": "u64"
          },
          {
            "name": "lpPrice",
            "type": "f64"
          }
        ]
      }
    },
    {
      "name": "wrapperSellYtEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "seller",
            "type": "pubkey"
          },
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "ytInAmount",
            "type": "u64"
          },
          {
            "name": "baseOutAmount",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "wrapperStripEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "userAddress",
            "type": "pubkey"
          },
          {
            "name": "vaultAddress",
            "type": "pubkey"
          },
          {
            "name": "amountBaseIn",
            "type": "u64"
          },
          {
            "name": "amountSyStripped",
            "type": "u64"
          },
          {
            "name": "amountPyOut",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "wrapperWithdrawLiquidityClassicEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "userAddress",
            "docs": [
              "User who withdrew liquidity"
            ],
            "type": "pubkey"
          },
          {
            "name": "marketAddress",
            "docs": [
              "Market that the user withdrew liquidity from"
            ],
            "type": "pubkey"
          },
          {
            "name": "amountBaseOut",
            "docs": [
              "Amount of base token out"
            ],
            "type": "u64"
          },
          {
            "name": "amountLpIn",
            "docs": [
              "Amount of LP token in"
            ],
            "type": "u64"
          },
          {
            "name": "amountPtOut",
            "docs": [
              "Amount of PT token out"
            ],
            "type": "u64"
          },
          {
            "name": "lpPrice",
            "docs": [
              "LP price in asset"
            ],
            "type": "f64"
          }
        ]
      }
    },
    {
      "name": "wrapperWithdrawLiquidityEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "userAddress",
            "type": "pubkey"
          },
          {
            "name": "marketAddress",
            "type": "pubkey"
          },
          {
            "name": "amountBaseOut",
            "type": "u64"
          },
          {
            "name": "amountLpIn",
            "type": "u64"
          },
          {
            "name": "lpPrice",
            "type": "f64"
          }
        ]
      }
    },
    {
      "name": "yieldTokenPosition",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "docs": [
              "Link to address that owns this position"
            ],
            "type": "pubkey"
          },
          {
            "name": "vault",
            "docs": [
              "Link to vault that manages the YT"
            ],
            "type": "pubkey"
          },
          {
            "name": "ytBalance",
            "docs": [
              "Track the YT balance of the user here"
            ],
            "type": "u64"
          },
          {
            "name": "interest",
            "docs": [
              "Tracker for interest earned (paid in SY)"
            ],
            "type": {
              "defined": {
                "name": "yieldTokenTracker"
              }
            }
          },
          {
            "name": "emissions",
            "docs": [
              "Tracker for emissions earned (paid in emission tokens)"
            ],
            "type": {
              "vec": {
                "defined": {
                  "name": "yieldTokenTracker"
                }
              }
            }
          }
        ]
      }
    },
    {
      "name": "yieldTokenTracker",
      "docs": [
        "Generic tracker for interest and emissions earned by YT deposits"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "lastSeenIndex",
            "docs": [
              "The index is the per-share value of the SY token",
              "Note that the YT balance must be converted to the equivalent SY balance"
            ],
            "type": {
              "defined": {
                "name": "number"
              }
            }
          },
          {
            "name": "staged",
            "docs": [
              "Staged tokens that may be withdrawn"
            ],
            "type": "u64"
          }
        ]
      }
    }
  ]
};
