/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/farms.json`.
 */
export type Farms = {
  "address": "FarmsPZpWu9i7Kky8tPN37rs2TpmMrAZrC7S7vJa91Hr",
  "metadata": {
    "name": "farms",
    "version": "0.1.0",
    "spec": "0.1.0"
  },
  "instructions": [
    {
      "name": "initializeGlobalConfig",
      "discriminator": [
        113,
        216,
        122,
        131,
        225,
        209,
        22,
        55
      ],
      "accounts": [
        {
          "name": "globalAdmin",
          "writable": true,
          "signer": true
        },
        {
          "name": "globalConfig",
          "writable": true
        },
        {
          "name": "treasuryVaultsAuthority"
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": []
    },
    {
      "name": "updateGlobalConfig",
      "discriminator": [
        164,
        84,
        130,
        189,
        111,
        58,
        250,
        200
      ],
      "accounts": [
        {
          "name": "globalAdmin",
          "signer": true
        },
        {
          "name": "globalConfig",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "mode",
          "type": "u8"
        },
        {
          "name": "value",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    },
    {
      "name": "initializeFarm",
      "discriminator": [
        252,
        28,
        185,
        172,
        244,
        74,
        117,
        165
      ],
      "accounts": [
        {
          "name": "farmAdmin",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "globalConfig"
        },
        {
          "name": "farmVault",
          "writable": true
        },
        {
          "name": "farmVaultsAuthority"
        },
        {
          "name": "tokenMint"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "rent"
        }
      ],
      "args": []
    },
    {
      "name": "initializeFarmDelegated",
      "discriminator": [
        250,
        84,
        101,
        25,
        51,
        77,
        204,
        91
      ],
      "accounts": [
        {
          "name": "farmAdmin",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmDelegate",
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "globalConfig"
        },
        {
          "name": "farmVaultsAuthority"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "rent"
        }
      ],
      "args": []
    },
    {
      "name": "initializeReward",
      "discriminator": [
        95,
        135,
        192,
        196,
        242,
        129,
        230,
        68
      ],
      "accounts": [
        {
          "name": "farmAdmin",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "globalConfig"
        },
        {
          "name": "rewardMint"
        },
        {
          "name": "rewardVault",
          "writable": true
        },
        {
          "name": "rewardTreasuryVault",
          "writable": true
        },
        {
          "name": "farmVaultsAuthority"
        },
        {
          "name": "treasuryVaultsAuthority"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "rent"
        }
      ],
      "args": []
    },
    {
      "name": "addRewards",
      "discriminator": [
        88,
        186,
        25,
        227,
        38,
        137,
        81,
        23
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "rewardMint"
        },
        {
          "name": "rewardVault",
          "writable": true
        },
        {
          "name": "farmVaultsAuthority"
        },
        {
          "name": "payerRewardTokenAta",
          "writable": true
        },
        {
          "name": "scopePrices",
          "optional": true
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "rewardIndex",
          "type": "u64"
        }
      ]
    },
    {
      "name": "updateFarmConfig",
      "discriminator": [
        214,
        176,
        188,
        244,
        203,
        59,
        230,
        207
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "scopePrices",
          "optional": true
        }
      ],
      "args": [
        {
          "name": "mode",
          "type": "u16"
        },
        {
          "name": "data",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "initializeUser",
      "discriminator": [
        111,
        17,
        185,
        250,
        60,
        122,
        38,
        254
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "owner"
        },
        {
          "name": "delegatee"
        },
        {
          "name": "userState",
          "writable": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "rent"
        }
      ],
      "args": []
    },
    {
      "name": "transferOwnership",
      "discriminator": [
        65,
        177,
        215,
        73,
        53,
        45,
        99,
        47
      ],
      "accounts": [
        {
          "name": "owner",
          "signer": true
        },
        {
          "name": "userState",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "newOwner",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "rewardUserOnce",
      "discriminator": [
        219,
        137,
        57,
        22,
        94,
        186,
        96,
        114
      ],
      "accounts": [
        {
          "name": "farmAdmin",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "userState",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "rewardIndex",
          "type": "u64"
        },
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "refreshFarm",
      "discriminator": [
        214,
        131,
        138,
        183,
        144,
        194,
        172,
        42
      ],
      "accounts": [
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "scopePrices",
          "optional": true
        }
      ],
      "args": []
    },
    {
      "name": "stake",
      "discriminator": [
        206,
        176,
        202,
        18,
        200,
        209,
        179,
        108
      ],
      "accounts": [
        {
          "name": "owner",
          "signer": true
        },
        {
          "name": "userState",
          "writable": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "farmVault",
          "writable": true
        },
        {
          "name": "userAta",
          "writable": true
        },
        {
          "name": "tokenMint"
        },
        {
          "name": "scopePrices",
          "optional": true
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "setStakeDelegated",
      "discriminator": [
        73,
        171,
        184,
        75,
        30,
        56,
        198,
        223
      ],
      "accounts": [
        {
          "name": "delegateAuthority",
          "signer": true
        },
        {
          "name": "userState",
          "writable": true
        },
        {
          "name": "farmState",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "newAmount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "harvestReward",
      "discriminator": [
        68,
        200,
        228,
        233,
        184,
        32,
        226,
        188
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "userState",
          "writable": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "globalConfig"
        },
        {
          "name": "rewardMint"
        },
        {
          "name": "userRewardAta",
          "writable": true
        },
        {
          "name": "rewardsVault",
          "writable": true
        },
        {
          "name": "rewardsTreasuryVault",
          "writable": true
        },
        {
          "name": "farmVaultsAuthority"
        },
        {
          "name": "scopePrices",
          "optional": true
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": [
        {
          "name": "rewardIndex",
          "type": "u64"
        }
      ]
    },
    {
      "name": "unstake",
      "discriminator": [
        90,
        95,
        107,
        42,
        205,
        124,
        50,
        225
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "userState",
          "writable": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "scopePrices",
          "optional": true
        }
      ],
      "args": [
        {
          "name": "stakeSharesScaled",
          "type": "u128"
        }
      ]
    },
    {
      "name": "refreshUserState",
      "discriminator": [
        1,
        135,
        12,
        62,
        243,
        140,
        77,
        108
      ],
      "accounts": [
        {
          "name": "userState",
          "writable": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "scopePrices",
          "optional": true
        }
      ],
      "args": []
    },
    {
      "name": "withdrawUnstakedDeposits",
      "discriminator": [
        36,
        102,
        187,
        49,
        220,
        36,
        132,
        67
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "userState",
          "writable": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "userAta",
          "writable": true
        },
        {
          "name": "farmVault",
          "writable": true
        },
        {
          "name": "farmVaultsAuthority"
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": []
    },
    {
      "name": "withdrawTreasury",
      "discriminator": [
        40,
        63,
        122,
        158,
        144,
        216,
        83,
        96
      ],
      "accounts": [
        {
          "name": "globalAdmin",
          "writable": true,
          "signer": true
        },
        {
          "name": "globalConfig"
        },
        {
          "name": "rewardMint"
        },
        {
          "name": "rewardTreasuryVault",
          "writable": true
        },
        {
          "name": "treasuryVaultAuthority"
        },
        {
          "name": "withdrawDestinationTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "depositToFarmVault",
      "discriminator": [
        131,
        166,
        64,
        94,
        108,
        213,
        114,
        183
      ],
      "accounts": [
        {
          "name": "depositor",
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "farmVault",
          "writable": true
        },
        {
          "name": "depositorAta",
          "writable": true
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "withdrawFromFarmVault",
      "discriminator": [
        22,
        82,
        128,
        250,
        86,
        79,
        124,
        78
      ],
      "accounts": [
        {
          "name": "withdrawAuthority",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "withdrawerTokenAccount",
          "writable": true
        },
        {
          "name": "farmVault",
          "writable": true
        },
        {
          "name": "farmVaultsAuthority"
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "withdrawSlashedAmount",
      "discriminator": [
        202,
        217,
        67,
        74,
        172,
        22,
        140,
        216
      ],
      "accounts": [
        {
          "name": "crank",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "slashedAmountSpillAddress",
          "writable": true
        },
        {
          "name": "farmVault",
          "writable": true
        },
        {
          "name": "farmVaultsAuthority"
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": []
    },
    {
      "name": "updateFarmAdmin",
      "discriminator": [
        20,
        37,
        136,
        19,
        122,
        239,
        36,
        130
      ],
      "accounts": [
        {
          "name": "pendingFarmAdmin",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "updateGlobalConfigAdmin",
      "discriminator": [
        184,
        87,
        23,
        193,
        156,
        238,
        175,
        119
      ],
      "accounts": [
        {
          "name": "pendingGlobalAdmin",
          "signer": true
        },
        {
          "name": "globalConfig",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "withdrawReward",
      "discriminator": [
        191,
        187,
        176,
        137,
        9,
        25,
        187,
        244
      ],
      "accounts": [
        {
          "name": "farmAdmin",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "rewardMint"
        },
        {
          "name": "rewardVault",
          "writable": true
        },
        {
          "name": "farmVaultsAuthority"
        },
        {
          "name": "adminRewardTokenAta",
          "writable": true
        },
        {
          "name": "scopePrices",
          "optional": true
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "rewardIndex",
          "type": "u64"
        }
      ]
    },
    {
      "name": "updateSecondDelegatedAuthority",
      "discriminator": [
        127,
        26,
        6,
        181,
        203,
        248,
        117,
        64
      ],
      "accounts": [
        {
          "name": "globalAdmin",
          "writable": true,
          "signer": true
        },
        {
          "name": "farmState",
          "writable": true
        },
        {
          "name": "globalConfig"
        },
        {
          "name": "newSecondDelegatedAuthority"
        }
      ],
      "args": []
    },
    {
      "name": "idlMissingTypes",
      "discriminator": [
        130,
        80,
        38,
        153,
        80,
        212,
        182,
        253
      ],
      "accounts": [
        {
          "name": "globalAdmin",
          "signer": true
        },
        {
          "name": "globalConfig",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "globalConfigOptionKind",
          "type": {
            "defined": {
              "name": "globalConfigOption"
            }
          }
        },
        {
          "name": "farmConfigOptionKind",
          "type": {
            "defined": {
              "name": "farmConfigOption"
            }
          }
        },
        {
          "name": "timeUnit",
          "type": {
            "defined": {
              "name": "timeUnit"
            }
          }
        },
        {
          "name": "lockingMode",
          "type": {
            "defined": {
              "name": "lockingMode"
            }
          }
        },
        {
          "name": "rewardType",
          "type": {
            "defined": {
              "name": "rewardType"
            }
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "farmState",
      "discriminator": [
        198,
        102,
        216,
        74,
        63,
        66,
        163,
        190
      ]
    },
    {
      "name": "globalConfig",
      "discriminator": [
        149,
        8,
        156,
        202,
        160,
        252,
        176,
        217
      ]
    },
    {
      "name": "userState",
      "discriminator": [
        72,
        177,
        85,
        249,
        76,
        167,
        186,
        126
      ]
    },
    {
      "name": "oraclePrices",
      "discriminator": [
        89,
        128,
        118,
        221,
        6,
        72,
        180,
        146
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "stakeZero",
      "msg": "Cannot stake 0 amount"
    },
    {
      "code": 6001,
      "name": "unstakeZero",
      "msg": "Cannot unstake 0 amount"
    },
    {
      "code": 6002,
      "name": "nothingToUnstake",
      "msg": "Nothing to unstake"
    },
    {
      "code": 6003,
      "name": "noRewardToHarvest",
      "msg": "No reward to harvest"
    },
    {
      "code": 6004,
      "name": "noRewardInList",
      "msg": "Reward not present in reward list"
    },
    {
      "code": 6005,
      "name": "rewardAlreadyInitialized",
      "msg": "Reward already initialized"
    },
    {
      "code": 6006,
      "name": "maxRewardNumberReached",
      "msg": "Max number of reward tokens reached"
    },
    {
      "code": 6007,
      "name": "rewardDoesNotExist",
      "msg": "Reward does not exist"
    },
    {
      "code": 6008,
      "name": "wrongRewardVaultAccount",
      "msg": "Reward vault exists but the account is wrong"
    },
    {
      "code": 6009,
      "name": "rewardVaultMismatch",
      "msg": "Reward vault pubkey does not match staking pool vault"
    },
    {
      "code": 6010,
      "name": "rewardVaultAuthorityMismatch",
      "msg": "Reward vault authority pubkey does not match staking pool vault"
    },
    {
      "code": 6011,
      "name": "nothingStaked",
      "msg": "Nothing staked, cannot collect any rewards"
    },
    {
      "code": 6012,
      "name": "integerOverflow",
      "msg": "Integer overflow"
    },
    {
      "code": 6013,
      "name": "conversionFailure",
      "msg": "Conversion failure"
    },
    {
      "code": 6014,
      "name": "unexpectedAccount",
      "msg": "Unexpected account in instruction"
    },
    {
      "code": 6015,
      "name": "operationForbidden",
      "msg": "Operation forbidden"
    },
    {
      "code": 6016,
      "name": "mathOverflow",
      "msg": "Mathematical operation with overflow"
    },
    {
      "code": 6017,
      "name": "minClaimDurationNotReached",
      "msg": "Minimum claim duration has not been reached"
    },
    {
      "code": 6018,
      "name": "rewardsVaultHasDelegate",
      "msg": "Reward vault has a delegate"
    },
    {
      "code": 6019,
      "name": "rewardsVaultHasCloseAuthority",
      "msg": "Reward vault has a close authority"
    },
    {
      "code": 6020,
      "name": "farmVaultHasDelegate",
      "msg": "Farm vault has a delegate"
    },
    {
      "code": 6021,
      "name": "farmVaultHasCloseAuthority",
      "msg": "Farm vault has a close authority"
    },
    {
      "code": 6022,
      "name": "rewardsTreasuryVaultHasDelegate",
      "msg": "Reward vault has a delegate"
    },
    {
      "code": 6023,
      "name": "rewardsTreasuryVaultHasCloseAuthority",
      "msg": "Reward vault has a close authority"
    },
    {
      "code": 6024,
      "name": "userAtaRewardVaultMintMissmatch",
      "msg": "User ata and reward vault have different mints"
    },
    {
      "code": 6025,
      "name": "userAtaFarmTokenMintMissmatch",
      "msg": "User ata and farm token have different mints"
    },
    {
      "code": 6026,
      "name": "tokenFarmTokenMintMissmatch",
      "msg": "Token mint and farm token have different mints"
    },
    {
      "code": 6027,
      "name": "rewardAtaRewardMintMissmatch",
      "msg": "Reward ata mint is different than reward mint"
    },
    {
      "code": 6028,
      "name": "rewardAtaOwnerNotPayer",
      "msg": "Reward ata owner is different than payer"
    },
    {
      "code": 6029,
      "name": "invalidGlobalConfigMode",
      "msg": "Mode to update global_config is invalid"
    },
    {
      "code": 6030,
      "name": "rewardIndexOutOfRange",
      "msg": "Reward Index is higher than number of rewards"
    },
    {
      "code": 6031,
      "name": "nothingToWithdraw",
      "msg": "No tokens available to withdraw"
    },
    {
      "code": 6032,
      "name": "userDelegatedFarmNonDelegatedMissmatch",
      "msg": "user, user_ref, authority and payer must match for non-delegated farm"
    },
    {
      "code": 6033,
      "name": "authorityFarmDelegateMissmatch",
      "msg": "Authority must match farm delegate authority"
    },
    {
      "code": 6034,
      "name": "farmNotDelegated",
      "msg": "Farm not delegated, can not set stake"
    },
    {
      "code": 6035,
      "name": "farmDelegated",
      "msg": "Operation not allowed for delegated farm"
    },
    {
      "code": 6036,
      "name": "unstakeNotElapsed",
      "msg": "Unstake lockup period is not elapsed. Deposit is locked until end of unstake period"
    },
    {
      "code": 6037,
      "name": "pendingWithdrawalNotWithdrawnYet",
      "msg": "Pending withdrawal already exist and not withdrawn yet"
    },
    {
      "code": 6038,
      "name": "depositZero",
      "msg": "Cannot deposit zero amount directly to farm vault"
    },
    {
      "code": 6039,
      "name": "invalidConfigValue",
      "msg": "Invalid config value"
    },
    {
      "code": 6040,
      "name": "invalidPenaltyPercentage",
      "msg": "Invalid penalty percentage"
    },
    {
      "code": 6041,
      "name": "earlyWithdrawalNotAllowed",
      "msg": "Early withdrawal not allowed"
    },
    {
      "code": 6042,
      "name": "invalidLockingTimestamps",
      "msg": "Invalid locking timestamps"
    },
    {
      "code": 6043,
      "name": "invalidRpsCurvePoint",
      "msg": "Invalid reward rate curve point"
    },
    {
      "code": 6044,
      "name": "invalidTimestamp",
      "msg": "Invalid timestamp"
    },
    {
      "code": 6045,
      "name": "depositCapReached",
      "msg": "Deposit cap reached"
    },
    {
      "code": 6046,
      "name": "missingScopePrices",
      "msg": "Missing Scope Prices"
    },
    {
      "code": 6047,
      "name": "scopeOraclePriceTooOld",
      "msg": "Scope Oracle Price Too Old"
    },
    {
      "code": 6048,
      "name": "invalidOracleConfig",
      "msg": "Invalid Oracle Config"
    },
    {
      "code": 6049,
      "name": "couldNotDeserializeScope",
      "msg": "Could not deserialize scope"
    },
    {
      "code": 6050,
      "name": "rewardAtaOwnerNotAdmin",
      "msg": "Reward ata owner is different than farm admin"
    },
    {
      "code": 6051,
      "name": "withdrawRewardZeroAvailable",
      "msg": "Cannot withdraw reward as available amount is zero"
    },
    {
      "code": 6052,
      "name": "rewardScheduleCurveSet",
      "msg": "Cannot withdraw reward as reward schedule is set"
    },
    {
      "code": 6053,
      "name": "unsupportedTokenExtension",
      "msg": "Cannot initialize farm while having a mint with token22 and requested extensions"
    },
    {
      "code": 6054,
      "name": "invalidFarmConfigUpdateAuthority",
      "msg": "Invalid authority for updating farm config"
    }
  ],
  "types": [
    {
      "name": "farmConfigOption",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "updateRewardRps"
          },
          {
            "name": "updateRewardMinClaimDuration"
          },
          {
            "name": "withdrawAuthority"
          },
          {
            "name": "depositWarmupPeriod"
          },
          {
            "name": "withdrawCooldownPeriod"
          },
          {
            "name": "rewardType"
          },
          {
            "name": "rpsDecimals"
          },
          {
            "name": "lockingMode"
          },
          {
            "name": "lockingStartTimestamp"
          },
          {
            "name": "lockingDuration"
          },
          {
            "name": "lockingEarlyWithdrawalPenaltyBps"
          },
          {
            "name": "depositCapAmount"
          },
          {
            "name": "slashedAmountSpillAddress"
          },
          {
            "name": "scopePricesAccount"
          },
          {
            "name": "scopeOraclePriceId"
          },
          {
            "name": "scopeOracleMaxAge"
          },
          {
            "name": "updateRewardScheduleCurvePoints"
          },
          {
            "name": "updatePendingFarmAdmin"
          },
          {
            "name": "updateStrategyId"
          },
          {
            "name": "updateDelegatedRpsAdmin"
          },
          {
            "name": "updateVaultId"
          },
          {
            "name": "updateExtraDelegatedAuthority"
          }
        ]
      }
    },
    {
      "name": "globalConfigOption",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "setPendingGlobalAdmin"
          },
          {
            "name": "setTreasuryFeeBps"
          }
        ]
      }
    },
    {
      "name": "lockingMode",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "none"
          },
          {
            "name": "continuous"
          },
          {
            "name": "withExpiry"
          }
        ]
      }
    },
    {
      "name": "rewardInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token",
            "type": {
              "defined": {
                "name": "tokenInfo"
              }
            }
          },
          {
            "name": "rewardsVault",
            "type": "pubkey"
          },
          {
            "name": "rewardsAvailable",
            "type": "u64"
          },
          {
            "name": "rewardScheduleCurve",
            "type": {
              "defined": {
                "name": "rewardScheduleCurve"
              }
            }
          },
          {
            "name": "minClaimDurationSeconds",
            "type": "u64"
          },
          {
            "name": "lastIssuanceTs",
            "type": "u64"
          },
          {
            "name": "rewardsIssuedUnclaimed",
            "type": "u64"
          },
          {
            "name": "rewardsIssuedCumulative",
            "type": "u64"
          },
          {
            "name": "rewardPerShareScaled",
            "type": "u128"
          },
          {
            "name": "placeholder0",
            "type": "u64"
          },
          {
            "name": "rewardType",
            "type": "u8"
          },
          {
            "name": "rewardsPerSecondDecimals",
            "type": "u8"
          },
          {
            "name": "padding0",
            "type": {
              "array": [
                "u8",
                6
              ]
            }
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u64",
                20
              ]
            }
          }
        ]
      }
    },
    {
      "name": "rewardPerTimeUnitPoint",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tsStart",
            "type": "u64"
          },
          {
            "name": "rewardPerTimeUnit",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "rewardScheduleCurve",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "points",
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "rewardPerTimeUnitPoint"
                  }
                },
                20
              ]
            }
          }
        ]
      }
    },
    {
      "name": "rewardType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "proportional"
          },
          {
            "name": "constant"
          }
        ]
      }
    },
    {
      "name": "timeUnit",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "seconds"
          },
          {
            "name": "slots"
          }
        ]
      }
    },
    {
      "name": "tokenInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "decimals",
            "type": "u64"
          },
          {
            "name": "tokenProgram",
            "type": "pubkey"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u64",
                6
              ]
            }
          }
        ]
      }
    },
    {
      "name": "datedPrice",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "price",
            "type": {
              "defined": {
                "name": "price"
              }
            }
          },
          {
            "name": "lastUpdatedSlot",
            "type": "u64"
          },
          {
            "name": "unixTimestamp",
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u64",
                2
              ]
            }
          },
          {
            "name": "reserved2",
            "type": {
              "array": [
                "u16",
                3
              ]
            }
          },
          {
            "name": "index",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "price",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "value",
            "type": "u64"
          },
          {
            "name": "exp",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "farmState",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "farmAdmin",
            "type": "pubkey"
          },
          {
            "name": "globalConfig",
            "type": "pubkey"
          },
          {
            "name": "token",
            "type": {
              "defined": {
                "name": "tokenInfo"
              }
            }
          },
          {
            "name": "rewardInfos",
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "rewardInfo"
                  }
                },
                10
              ]
            }
          },
          {
            "name": "numRewardTokens",
            "type": "u64"
          },
          {
            "name": "numUsers",
            "type": "u64"
          },
          {
            "name": "totalStakedAmount",
            "type": "u64"
          },
          {
            "name": "farmVault",
            "type": "pubkey"
          },
          {
            "name": "farmVaultsAuthority",
            "type": "pubkey"
          },
          {
            "name": "farmVaultsAuthorityBump",
            "type": "u64"
          },
          {
            "name": "delegateAuthority",
            "type": "pubkey"
          },
          {
            "name": "timeUnit",
            "type": "u8"
          },
          {
            "name": "isFarmFrozen",
            "type": "u8"
          },
          {
            "name": "isFarmDelegated",
            "type": "u8"
          },
          {
            "name": "padding0",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "withdrawAuthority",
            "type": "pubkey"
          },
          {
            "name": "depositWarmupPeriod",
            "type": "u32"
          },
          {
            "name": "withdrawalCooldownPeriod",
            "type": "u32"
          },
          {
            "name": "totalActiveStakeScaled",
            "type": "u128"
          },
          {
            "name": "totalPendingStakeScaled",
            "type": "u128"
          },
          {
            "name": "totalPendingAmount",
            "type": "u64"
          },
          {
            "name": "slashedAmountCurrent",
            "type": "u64"
          },
          {
            "name": "slashedAmountCumulative",
            "type": "u64"
          },
          {
            "name": "slashedAmountSpillAddress",
            "type": "pubkey"
          },
          {
            "name": "lockingMode",
            "type": "u64"
          },
          {
            "name": "lockingStartTimestamp",
            "type": "u64"
          },
          {
            "name": "lockingDuration",
            "type": "u64"
          },
          {
            "name": "lockingEarlyWithdrawalPenaltyBps",
            "type": "u64"
          },
          {
            "name": "depositCapAmount",
            "type": "u64"
          },
          {
            "name": "scopePrices",
            "type": "pubkey"
          },
          {
            "name": "scopeOraclePriceId",
            "type": "u64"
          },
          {
            "name": "scopeOracleMaxAge",
            "type": "u64"
          },
          {
            "name": "pendingFarmAdmin",
            "type": "pubkey"
          },
          {
            "name": "strategyId",
            "type": "pubkey"
          },
          {
            "name": "delegatedRpsAdmin",
            "type": "pubkey"
          },
          {
            "name": "vaultId",
            "type": "pubkey"
          },
          {
            "name": "secondDelegatedAuthority",
            "type": "pubkey"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u64",
                74
              ]
            }
          }
        ]
      }
    },
    {
      "name": "globalConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "globalAdmin",
            "type": "pubkey"
          },
          {
            "name": "treasuryFeeBps",
            "type": "u64"
          },
          {
            "name": "treasuryVaultsAuthority",
            "type": "pubkey"
          },
          {
            "name": "treasuryVaultsAuthorityBump",
            "type": "u64"
          },
          {
            "name": "pendingGlobalAdmin",
            "type": "pubkey"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u128",
                126
              ]
            }
          }
        ]
      }
    },
    {
      "name": "userState",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "userId",
            "type": "u64"
          },
          {
            "name": "farmState",
            "type": "pubkey"
          },
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "isFarmDelegated",
            "type": "u8"
          },
          {
            "name": "padding0",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "rewardsTallyScaled",
            "type": {
              "array": [
                "u128",
                10
              ]
            }
          },
          {
            "name": "rewardsIssuedUnclaimed",
            "type": {
              "array": [
                "u64",
                10
              ]
            }
          },
          {
            "name": "lastClaimTs",
            "type": {
              "array": [
                "u64",
                10
              ]
            }
          },
          {
            "name": "activeStakeScaled",
            "type": "u128"
          },
          {
            "name": "pendingDepositStakeScaled",
            "type": "u128"
          },
          {
            "name": "pendingDepositStakeTs",
            "type": "u64"
          },
          {
            "name": "pendingWithdrawalUnstakeScaled",
            "type": "u128"
          },
          {
            "name": "pendingWithdrawalUnstakeTs",
            "type": "u64"
          },
          {
            "name": "bump",
            "type": "u64"
          },
          {
            "name": "delegatee",
            "type": "pubkey"
          },
          {
            "name": "lastStakeTs",
            "type": "u64"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u64",
                50
              ]
            }
          }
        ]
      }
    },
    {
      "name": "oraclePrices",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "oracleMappings",
            "type": "pubkey"
          },
          {
            "name": "prices",
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "datedPrice"
                  }
                },
                512
              ]
            }
          }
        ]
      }
    }
  ]
};