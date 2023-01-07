export type Marginfi = {
  version: "0.1.0";
  name: "marginfi";
  instructions: [
    {
      name: "initializeMarginfiGroup";
      docs: ["Initialize a new Marginfi Group with initial config"];
      accounts: [
        {
          name: "marginfiGroup";
          isMut: true;
          isSigner: true;
        },
        {
          name: "admin";
          isMut: true;
          isSigner: true;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [];
    },
    {
      name: "configureMarginfiGroup";
      docs: ["Configure a Marginfi Group"];
      accounts: [
        {
          name: "marginfiGroup";
          isMut: true;
          isSigner: false;
        },
        {
          name: "admin";
          isMut: false;
          isSigner: true;
        }
      ];
      args: [
        {
          name: "config";
          type: {
            defined: "GroupConfig";
          };
        }
      ];
    },
    {
      name: "lendingPoolAddBank";
      docs: ["Add a new bank to the Marginfi Group"];
      accounts: [
        {
          name: "marginfiGroup";
          isMut: false;
          isSigner: false;
        },
        {
          name: "admin";
          isMut: true;
          isSigner: true;
        },
        {
          name: "bankMint";
          isMut: false;
          isSigner: false;
        },
        {
          name: "bank";
          isMut: true;
          isSigner: true;
        },
        {
          name: "liquidityVaultAuthority";
          isMut: false;
          isSigner: false;
        },
        {
          name: "liquidityVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "insuranceVaultAuthority";
          isMut: false;
          isSigner: false;
        },
        {
          name: "insuranceVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "feeVaultAuthority";
          isMut: false;
          isSigner: false;
        },
        {
          name: "feeVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "pythOracle";
          isMut: false;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "bankConfig";
          type: {
            defined: "BankConfig";
          };
        }
      ];
    },
    {
      name: "lendingPoolConfigureBank";
      docs: ["Configure a bank in the Marginfi Group"];
      accounts: [
        {
          name: "marginfiGroup";
          isMut: false;
          isSigner: false;
        },
        {
          name: "admin";
          isMut: false;
          isSigner: true;
        },
        {
          name: "bank";
          isMut: true;
          isSigner: false;
        },
        {
          name: "pythOracle";
          isMut: false;
          isSigner: false;
          docs: [
            "Set only if pyth oracle is being changed otherwise can be a random account."
          ];
        }
      ];
      args: [
        {
          name: "bankConfigOpt";
          type: {
            defined: "BankConfigOpt";
          };
        }
      ];
    },
    {
      name: "lendingPoolHandleBankruptcy";
      docs: [
        "Handle bad debt of a bankrupt marginfi account for a given bank."
      ];
      accounts: [
        {
          name: "marginfiGroup";
          isMut: false;
          isSigner: false;
        },
        {
          name: "admin";
          isMut: false;
          isSigner: true;
        },
        {
          name: "bank";
          isMut: true;
          isSigner: false;
        },
        {
          name: "marginfiAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "liquidityVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "insuranceVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "insuranceVaultAuthority";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [];
    },
    {
      name: "initializeMarginfiAccount";
      docs: ["Initialize a marginfi account for a given group"];
      accounts: [
        {
          name: "marginfiGroup";
          isMut: false;
          isSigner: false;
        },
        {
          name: "marginfiAccount";
          isMut: true;
          isSigner: true;
        },
        {
          name: "signer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [];
    },
    {
      name: "bankDeposit";
      docs: [
        "Deposit assets into a lending account",
        "Repay borrowed assets, if any exist."
      ];
      accounts: [
        {
          name: "marginfiGroup";
          isMut: false;
          isSigner: false;
        },
        {
          name: "marginfiAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "signer";
          isMut: false;
          isSigner: true;
        },
        {
          name: "bank";
          isMut: true;
          isSigner: false;
        },
        {
          name: "signerTokenAccount";
          isMut: true;
          isSigner: false;
          docs: ["Token mint/authority are checked at transfer"];
        },
        {
          name: "bankLiquidityVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        }
      ];
    },
    {
      name: "bankWithdraw";
      docs: [
        "Withdraw assets from a lending account",
        "Withdraw deposited assets, if any exist, otherwise borrow assets.",
        "Account health checked."
      ];
      accounts: [
        {
          name: "marginfiGroup";
          isMut: false;
          isSigner: false;
        },
        {
          name: "marginfiAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "signer";
          isMut: false;
          isSigner: true;
        },
        {
          name: "bank";
          isMut: true;
          isSigner: false;
        },
        {
          name: "destinationTokenAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "bankLiquidityVaultAuthority";
          isMut: true;
          isSigner: false;
        },
        {
          name: "bankLiquidityVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        }
      ];
    },
    {
      name: "lendingAccountLiquidate";
      docs: [
        "Liquidate a lending account balance of an unhealthy marginfi account"
      ];
      accounts: [
        {
          name: "marginfiGroup";
          isMut: false;
          isSigner: false;
        },
        {
          name: "assetBank";
          isMut: true;
          isSigner: false;
        },
        {
          name: "assetPriceFeed";
          isMut: false;
          isSigner: false;
        },
        {
          name: "liabBank";
          isMut: true;
          isSigner: false;
        },
        {
          name: "liabPriceFeed";
          isMut: false;
          isSigner: false;
        },
        {
          name: "liquidatorMarginfiAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "signer";
          isMut: false;
          isSigner: true;
        },
        {
          name: "liquidateeMarginfiAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "bankLiquidityVaultAuthority";
          isMut: true;
          isSigner: false;
        },
        {
          name: "bankLiquidityVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "bankInsuranceVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "assetAmount";
          type: "u64";
        }
      ];
    },
    {
      name: "bankAccrueInterest";
      accounts: [
        {
          name: "marginfiGroup";
          isMut: false;
          isSigner: false;
        },
        {
          name: "bank";
          isMut: true;
          isSigner: false;
        },
        {
          name: "liquidityVaultAuthority";
          isMut: false;
          isSigner: false;
        },
        {
          name: "liquidityVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "insuranceVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "feeVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [];
    }
  ];
  accounts: [
    {
      name: "marginfiAccount";
      type: {
        kind: "struct";
        fields: [
          {
            name: "group";
            type: "publicKey";
          },
          {
            name: "authority";
            type: "publicKey";
          },
          {
            name: "lendingAccount";
            type: {
              defined: "LendingAccount";
            };
          }
        ];
      };
    },
    {
      name: "marginfiGroup";
      type: {
        kind: "struct";
        fields: [
          {
            name: "admin";
            type: "publicKey";
          }
        ];
      };
    },
    {
      name: "bank";
      type: {
        kind: "struct";
        fields: [
          {
            name: "mint";
            type: "publicKey";
          },
          {
            name: "mintDecimals";
            type: "u8";
          },
          {
            name: "group";
            type: "publicKey";
          },
          {
            name: "ignore1";
            type: {
              array: ["u8", 7];
            };
          },
          {
            name: "depositShareValue";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "liabilityShareValue";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "liquidityVault";
            type: "publicKey";
          },
          {
            name: "liquidityVaultBump";
            type: "u8";
          },
          {
            name: "liquidityVaultAuthorityBump";
            type: "u8";
          },
          {
            name: "insuranceVault";
            type: "publicKey";
          },
          {
            name: "insuranceVaultBump";
            type: "u8";
          },
          {
            name: "insuranceVaultAuthorityBump";
            type: "u8";
          },
          {
            name: "feeVault";
            type: "publicKey";
          },
          {
            name: "feeVaultBump";
            type: "u8";
          },
          {
            name: "feeVaultAuthorityBump";
            type: "u8";
          },
          {
            name: "ignore2";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "config";
            type: {
              defined: "BankConfig";
            };
          },
          {
            name: "totalLiabilityShares";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "totalDepositShares";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "lastUpdate";
            type: "i64";
          }
        ];
      };
    }
  ];
  types: [
    {
      name: "LendingAccount";
      type: {
        kind: "struct";
        fields: [
          {
            name: "balances";
            type: {
              array: [
                {
                  defined: "Balance";
                },
                16
              ];
            };
          }
        ];
      };
    },
    {
      name: "Balance";
      type: {
        kind: "struct";
        fields: [
          {
            name: "active";
            type: "bool";
          },
          {
            name: "bankPk";
            type: "publicKey";
          },
          {
            name: "ignore";
            type: {
              array: ["u8", 7];
            };
          },
          {
            name: "depositShares";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "liabilityShares";
            type: {
              defined: "WrappedI80F48";
            };
          }
        ];
      };
    },
    {
      name: "GroupConfig";
      type: {
        kind: "struct";
        fields: [
          {
            name: "admin";
            type: {
              option: "publicKey";
            };
          }
        ];
      };
    },
    {
      name: "InterestRateConfig";
      type: {
        kind: "struct";
        fields: [
          {
            name: "optimalUtilizationRate";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "plateauInterestRate";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "maxInterestRate";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "insuranceFeeFixedApr";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "insuranceIrFee";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "protocolFixedFeeApr";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "protocolIrFee";
            type: {
              defined: "WrappedI80F48";
            };
          }
        ];
      };
    },
    {
      name: "BankConfig";
      docs: [
        "TODO: Convert weights to (u64, u64) to avoid precision loss (maybe?)"
      ];
      type: {
        kind: "struct";
        fields: [
          {
            name: "depositWeightInit";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "depositWeightMaint";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "liabilityWeightInit";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "liabilityWeightMaint";
            type: {
              defined: "WrappedI80F48";
            };
          },
          {
            name: "maxCapacity";
            type: "u64";
          },
          {
            name: "pythOracle";
            type: "publicKey";
          },
          {
            name: "interestRateConfig";
            type: {
              defined: "InterestRateConfig";
            };
          }
        ];
      };
    },
    {
      name: "WrappedI80F48";
      type: {
        kind: "struct";
        fields: [
          {
            name: "value";
            type: "i128";
          }
        ];
      };
    },
    {
      name: "BankConfigOpt";
      type: {
        kind: "struct";
        fields: [
          {
            name: "depositWeightInit";
            type: {
              option: {
                defined: "WrappedI80F48";
              };
            };
          },
          {
            name: "depositWeightMaint";
            type: {
              option: {
                defined: "WrappedI80F48";
              };
            };
          },
          {
            name: "liabilityWeightInit";
            type: {
              option: {
                defined: "WrappedI80F48";
              };
            };
          },
          {
            name: "liabilityWeightMaint";
            type: {
              option: {
                defined: "WrappedI80F48";
              };
            };
          },
          {
            name: "maxCapacity";
            type: {
              option: "u64";
            };
          },
          {
            name: "pythOracle";
            type: {
              option: "publicKey";
            };
          }
        ];
      };
    },
    {
      name: "WeightType";
      type: {
        kind: "enum";
        variants: [
          {
            name: "Initial";
          },
          {
            name: "Maintenance";
          }
        ];
      };
    },
    {
      name: "RiskRequirementType";
      type: {
        kind: "enum";
        variants: [
          {
            name: "Initial";
          },
          {
            name: "Maintenance";
          }
        ];
      };
    },
    {
      name: "BankVaultType";
      type: {
        kind: "enum";
        variants: [
          {
            name: "Liquidity";
          },
          {
            name: "Insurance";
          },
          {
            name: "Fee";
          }
        ];
      };
    }
  ];
  errors: [
    {
      code: 6000;
      name: "MathError";
      msg: "Math error";
    },
    {
      code: 6001;
      name: "BankNotFound";
      msg: "Invalid bank index";
    },
    {
      code: 6002;
      name: "LendingAccountBalanceNotFound";
      msg: "Lending account balance not found";
    },
    {
      code: 6003;
      name: "BankDepositCapacityExceeded";
      msg: "Bank deposit capacity exceeded";
    },
    {
      code: 6004;
      name: "InvalidTransfer";
      msg: "Invalid transfer";
    },
    {
      code: 6005;
      name: "MissingPythOrBankAccount";
      msg: "Missing Pyth or Bank account";
    },
    {
      code: 6006;
      name: "MissingPythAccount";
      msg: "Missing Pyth account";
    },
    {
      code: 6007;
      name: "InvalidPythAccount";
      msg: "Invalid Pyth account";
    },
    {
      code: 6008;
      name: "MissingBankAccount";
      msg: "Missing Bank account";
    },
    {
      code: 6009;
      name: "InvalidBankAccount";
      msg: "Invalid Bank account";
    },
    {
      code: 6010;
      name: "BadAccountHealth";
      msg: "Bad account health";
    },
    {
      code: 6011;
      name: "LendingAccountBalanceSlotsFull";
      msg: "Lending account balance slots are full";
    },
    {
      code: 6012;
      name: "BankAlreadyExists";
      msg: "Bank already exists";
    },
    {
      code: 6013;
      name: "BorrowingNotAllowed";
      msg: "Borrowing not allowed";
    },
    {
      code: 6014;
      name: "AccountIllegalPostLiquidationState";
      msg: "Illegal post liquidation state, account is either not unhealthy or liquidation was too big";
    },
    {
      code: 6015;
      name: "AccountNotBankrupt";
      msg: "Account is not bankrupt";
    },
    {
      code: 6016;
      name: "BalanceNotBadDebt";
      msg: "Account balance is not bad debt";
    },
    {
      code: 6017;
      name: "InvalidConfig";
      msg: "Invalid group config";
    },
    {
      code: 6018;
      name: "StaleOracle";
      msg: "Stale oracle data";
    }
  ];
};

export const IDL: Marginfi = {
  version: "0.1.0",
  name: "marginfi",
  instructions: [
    {
      name: "initializeMarginfiGroup",
      docs: ["Initialize a new Marginfi Group with initial config"],
      accounts: [
        {
          name: "marginfiGroup",
          isMut: true,
          isSigner: true,
        },
        {
          name: "admin",
          isMut: true,
          isSigner: true,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "configureMarginfiGroup",
      docs: ["Configure a Marginfi Group"],
      accounts: [
        {
          name: "marginfiGroup",
          isMut: true,
          isSigner: false,
        },
        {
          name: "admin",
          isMut: false,
          isSigner: true,
        },
      ],
      args: [
        {
          name: "config",
          type: {
            defined: "GroupConfig",
          },
        },
      ],
    },
    {
      name: "lendingPoolAddBank",
      docs: ["Add a new bank to the Marginfi Group"],
      accounts: [
        {
          name: "marginfiGroup",
          isMut: false,
          isSigner: false,
        },
        {
          name: "admin",
          isMut: true,
          isSigner: true,
        },
        {
          name: "bankMint",
          isMut: false,
          isSigner: false,
        },
        {
          name: "bank",
          isMut: true,
          isSigner: true,
        },
        {
          name: "liquidityVaultAuthority",
          isMut: false,
          isSigner: false,
        },
        {
          name: "liquidityVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "insuranceVaultAuthority",
          isMut: false,
          isSigner: false,
        },
        {
          name: "insuranceVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "feeVaultAuthority",
          isMut: false,
          isSigner: false,
        },
        {
          name: "feeVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "pythOracle",
          isMut: false,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "bankConfig",
          type: {
            defined: "BankConfig",
          },
        },
      ],
    },
    {
      name: "lendingPoolConfigureBank",
      docs: ["Configure a bank in the Marginfi Group"],
      accounts: [
        {
          name: "marginfiGroup",
          isMut: false,
          isSigner: false,
        },
        {
          name: "admin",
          isMut: false,
          isSigner: true,
        },
        {
          name: "bank",
          isMut: true,
          isSigner: false,
        },
        {
          name: "pythOracle",
          isMut: false,
          isSigner: false,
          docs: [
            "Set only if pyth oracle is being changed otherwise can be a random account.",
          ],
        },
      ],
      args: [
        {
          name: "bankConfigOpt",
          type: {
            defined: "BankConfigOpt",
          },
        },
      ],
    },
    {
      name: "lendingPoolHandleBankruptcy",
      docs: [
        "Handle bad debt of a bankrupt marginfi account for a given bank.",
      ],
      accounts: [
        {
          name: "marginfiGroup",
          isMut: false,
          isSigner: false,
        },
        {
          name: "admin",
          isMut: false,
          isSigner: true,
        },
        {
          name: "bank",
          isMut: true,
          isSigner: false,
        },
        {
          name: "marginfiAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "liquidityVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "insuranceVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "insuranceVaultAuthority",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "initializeMarginfiAccount",
      docs: ["Initialize a marginfi account for a given group"],
      accounts: [
        {
          name: "marginfiGroup",
          isMut: false,
          isSigner: false,
        },
        {
          name: "marginfiAccount",
          isMut: true,
          isSigner: true,
        },
        {
          name: "signer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "bankDeposit",
      docs: [
        "Deposit assets into a lending account",
        "Repay borrowed assets, if any exist.",
      ],
      accounts: [
        {
          name: "marginfiGroup",
          isMut: false,
          isSigner: false,
        },
        {
          name: "marginfiAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "signer",
          isMut: false,
          isSigner: true,
        },
        {
          name: "bank",
          isMut: true,
          isSigner: false,
        },
        {
          name: "signerTokenAccount",
          isMut: true,
          isSigner: false,
          docs: ["Token mint/authority are checked at transfer"],
        },
        {
          name: "bankLiquidityVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "amount",
          type: "u64",
        },
      ],
    },
    {
      name: "bankWithdraw",
      docs: [
        "Withdraw assets from a lending account",
        "Withdraw deposited assets, if any exist, otherwise borrow assets.",
        "Account health checked.",
      ],
      accounts: [
        {
          name: "marginfiGroup",
          isMut: false,
          isSigner: false,
        },
        {
          name: "marginfiAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "signer",
          isMut: false,
          isSigner: true,
        },
        {
          name: "bank",
          isMut: true,
          isSigner: false,
        },
        {
          name: "destinationTokenAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "bankLiquidityVaultAuthority",
          isMut: true,
          isSigner: false,
        },
        {
          name: "bankLiquidityVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "amount",
          type: "u64",
        },
      ],
    },
    {
      name: "lendingAccountLiquidate",
      docs: [
        "Liquidate a lending account balance of an unhealthy marginfi account",
      ],
      accounts: [
        {
          name: "marginfiGroup",
          isMut: false,
          isSigner: false,
        },
        {
          name: "assetBank",
          isMut: true,
          isSigner: false,
        },
        {
          name: "assetPriceFeed",
          isMut: false,
          isSigner: false,
        },
        {
          name: "liabBank",
          isMut: true,
          isSigner: false,
        },
        {
          name: "liabPriceFeed",
          isMut: false,
          isSigner: false,
        },
        {
          name: "liquidatorMarginfiAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "signer",
          isMut: false,
          isSigner: true,
        },
        {
          name: "liquidateeMarginfiAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "bankLiquidityVaultAuthority",
          isMut: true,
          isSigner: false,
        },
        {
          name: "bankLiquidityVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "bankInsuranceVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "assetAmount",
          type: "u64",
        },
      ],
    },
    {
      name: "bankAccrueInterest",
      accounts: [
        {
          name: "marginfiGroup",
          isMut: false,
          isSigner: false,
        },
        {
          name: "bank",
          isMut: true,
          isSigner: false,
        },
        {
          name: "liquidityVaultAuthority",
          isMut: false,
          isSigner: false,
        },
        {
          name: "liquidityVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "insuranceVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "feeVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
  ],
  accounts: [
    {
      name: "marginfiAccount",
      type: {
        kind: "struct",
        fields: [
          {
            name: "group",
            type: "publicKey",
          },
          {
            name: "authority",
            type: "publicKey",
          },
          {
            name: "lendingAccount",
            type: {
              defined: "LendingAccount",
            },
          },
        ],
      },
    },
    {
      name: "marginfiGroup",
      type: {
        kind: "struct",
        fields: [
          {
            name: "admin",
            type: "publicKey",
          },
        ],
      },
    },
    {
      name: "bank",
      type: {
        kind: "struct",
        fields: [
          {
            name: "mint",
            type: "publicKey",
          },
          {
            name: "mintDecimals",
            type: "u8",
          },
          {
            name: "group",
            type: "publicKey",
          },
          {
            name: "ignore1",
            type: {
              array: ["u8", 7],
            },
          },
          {
            name: "depositShareValue",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "liabilityShareValue",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "liquidityVault",
            type: "publicKey",
          },
          {
            name: "liquidityVaultBump",
            type: "u8",
          },
          {
            name: "liquidityVaultAuthorityBump",
            type: "u8",
          },
          {
            name: "insuranceVault",
            type: "publicKey",
          },
          {
            name: "insuranceVaultBump",
            type: "u8",
          },
          {
            name: "insuranceVaultAuthorityBump",
            type: "u8",
          },
          {
            name: "feeVault",
            type: "publicKey",
          },
          {
            name: "feeVaultBump",
            type: "u8",
          },
          {
            name: "feeVaultAuthorityBump",
            type: "u8",
          },
          {
            name: "ignore2",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "config",
            type: {
              defined: "BankConfig",
            },
          },
          {
            name: "totalLiabilityShares",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "totalDepositShares",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "lastUpdate",
            type: "i64",
          },
        ],
      },
    },
  ],
  types: [
    {
      name: "LendingAccount",
      type: {
        kind: "struct",
        fields: [
          {
            name: "balances",
            type: {
              array: [
                {
                  defined: "Balance",
                },
                16,
              ],
            },
          },
        ],
      },
    },
    {
      name: "Balance",
      type: {
        kind: "struct",
        fields: [
          {
            name: "active",
            type: "bool",
          },
          {
            name: "bankPk",
            type: "publicKey",
          },
          {
            name: "ignore",
            type: {
              array: ["u8", 7],
            },
          },
          {
            name: "depositShares",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "liabilityShares",
            type: {
              defined: "WrappedI80F48",
            },
          },
        ],
      },
    },
    {
      name: "GroupConfig",
      type: {
        kind: "struct",
        fields: [
          {
            name: "admin",
            type: {
              option: "publicKey",
            },
          },
        ],
      },
    },
    {
      name: "InterestRateConfig",
      type: {
        kind: "struct",
        fields: [
          {
            name: "optimalUtilizationRate",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "plateauInterestRate",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "maxInterestRate",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "insuranceFeeFixedApr",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "insuranceIrFee",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "protocolFixedFeeApr",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "protocolIrFee",
            type: {
              defined: "WrappedI80F48",
            },
          },
        ],
      },
    },
    {
      name: "BankConfig",
      docs: [
        "TODO: Convert weights to (u64, u64) to avoid precision loss (maybe?)",
      ],
      type: {
        kind: "struct",
        fields: [
          {
            name: "depositWeightInit",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "depositWeightMaint",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "liabilityWeightInit",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "liabilityWeightMaint",
            type: {
              defined: "WrappedI80F48",
            },
          },
          {
            name: "maxCapacity",
            type: "u64",
          },
          {
            name: "pythOracle",
            type: "publicKey",
          },
          {
            name: "interestRateConfig",
            type: {
              defined: "InterestRateConfig",
            },
          },
        ],
      },
    },
    {
      name: "WrappedI80F48",
      type: {
        kind: "struct",
        fields: [
          {
            name: "value",
            type: "i128",
          },
        ],
      },
    },
    {
      name: "BankConfigOpt",
      type: {
        kind: "struct",
        fields: [
          {
            name: "depositWeightInit",
            type: {
              option: {
                defined: "WrappedI80F48",
              },
            },
          },
          {
            name: "depositWeightMaint",
            type: {
              option: {
                defined: "WrappedI80F48",
              },
            },
          },
          {
            name: "liabilityWeightInit",
            type: {
              option: {
                defined: "WrappedI80F48",
              },
            },
          },
          {
            name: "liabilityWeightMaint",
            type: {
              option: {
                defined: "WrappedI80F48",
              },
            },
          },
          {
            name: "maxCapacity",
            type: {
              option: "u64",
            },
          },
          {
            name: "pythOracle",
            type: {
              option: "publicKey",
            },
          },
        ],
      },
    },
    {
      name: "WeightType",
      type: {
        kind: "enum",
        variants: [
          {
            name: "Initial",
          },
          {
            name: "Maintenance",
          },
        ],
      },
    },
    {
      name: "RiskRequirementType",
      type: {
        kind: "enum",
        variants: [
          {
            name: "Initial",
          },
          {
            name: "Maintenance",
          },
        ],
      },
    },
    {
      name: "BankVaultType",
      type: {
        kind: "enum",
        variants: [
          {
            name: "Liquidity",
          },
          {
            name: "Insurance",
          },
          {
            name: "Fee",
          },
        ],
      },
    },
  ],
  errors: [
    {
      code: 6000,
      name: "MathError",
      msg: "Math error",
    },
    {
      code: 6001,
      name: "BankNotFound",
      msg: "Invalid bank index",
    },
    {
      code: 6002,
      name: "LendingAccountBalanceNotFound",
      msg: "Lending account balance not found",
    },
    {
      code: 6003,
      name: "BankDepositCapacityExceeded",
      msg: "Bank deposit capacity exceeded",
    },
    {
      code: 6004,
      name: "InvalidTransfer",
      msg: "Invalid transfer",
    },
    {
      code: 6005,
      name: "MissingPythOrBankAccount",
      msg: "Missing Pyth or Bank account",
    },
    {
      code: 6006,
      name: "MissingPythAccount",
      msg: "Missing Pyth account",
    },
    {
      code: 6007,
      name: "InvalidPythAccount",
      msg: "Invalid Pyth account",
    },
    {
      code: 6008,
      name: "MissingBankAccount",
      msg: "Missing Bank account",
    },
    {
      code: 6009,
      name: "InvalidBankAccount",
      msg: "Invalid Bank account",
    },
    {
      code: 6010,
      name: "BadAccountHealth",
      msg: "Bad account health",
    },
    {
      code: 6011,
      name: "LendingAccountBalanceSlotsFull",
      msg: "Lending account balance slots are full",
    },
    {
      code: 6012,
      name: "BankAlreadyExists",
      msg: "Bank already exists",
    },
    {
      code: 6013,
      name: "BorrowingNotAllowed",
      msg: "Borrowing not allowed",
    },
    {
      code: 6014,
      name: "AccountIllegalPostLiquidationState",
      msg: "Illegal post liquidation state, account is either not unhealthy or liquidation was too big",
    },
    {
      code: 6015,
      name: "AccountNotBankrupt",
      msg: "Account is not bankrupt",
    },
    {
      code: 6016,
      name: "BalanceNotBadDebt",
      msg: "Account balance is not bad debt",
    },
    {
      code: 6017,
      name: "InvalidConfig",
      msg: "Invalid group config",
    },
    {
      code: 6018,
      name: "StaleOracle",
      msg: "Stale oracle data",
    },
  ],
};
