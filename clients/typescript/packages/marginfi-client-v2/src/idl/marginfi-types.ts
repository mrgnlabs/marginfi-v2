export type Marginfi = {
  version: "0.1.0";
  name: "marginfi";
  constants: [
    {
      name: "LIQUIDITY_VAULT_AUTHORITY_SEED";
      type: "bytes";
      value: "[108, 105, 113, 117, 105, 100, 105, 116, 121, 95, 118, 97, 117, 108, 116, 95, 97, 117, 116, 104]";
    },
    {
      name: "INSURANCE_VAULT_AUTHORITY_SEED";
      type: "bytes";
      value: "[105, 110, 115, 117, 114, 97, 110, 99, 101, 95, 118, 97, 117, 108, 116, 95, 97, 117, 116, 104]";
    },
    {
      name: "FEE_VAULT_AUTHORITY_SEED";
      type: "bytes";
      value: "[102, 101, 101, 95, 118, 97, 117, 108, 116, 95, 97, 117, 116, 104]";
    },
    {
      name: "LIQUIDITY_VAULT_SEED";
      type: "bytes";
      value: "[108, 105, 113, 117, 105, 100, 105, 116, 121, 95, 118, 97, 117, 108, 116]";
    },
    {
      name: "INSURANCE_VAULT_SEED";
      type: "bytes";
      value: "[105, 110, 115, 117, 114, 97, 110, 99, 101, 95, 118, 97, 117, 108, 116]";
    },
    {
      name: "FEE_VAULT_SEED";
      type: "bytes";
      value: "[102, 101, 101, 95, 118, 97, 117, 108, 116]";
    },
    {
      name: "LENDING_POOL_BANK_SEED";
      type: "bytes";
      value: "[108, 101, 110, 100, 105, 110, 103, 95, 112, 111, 111, 108, 95, 98, 97, 110, 107]";
    }
  ];
  instructions: [
    {
      name: "initializeMarginfiGroup";
      docs: ["Initialize a new Marginfi Group with initial config"];
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
      name: "initializeMarginfiAccount";
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
      accounts: [
        {
          name: "marginfiGroup";
          isMut: true;
          isSigner: false;
        },
        {
          name: "marginfiAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "signer";
          isMut: true;
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
          docs: ["Token mint is checked at transfer"];
        },
        {
          name: "bankLiquidityVault";
          isMut: true;
          isSigner: false;
          docs: ["TODO: Store bump on-chain"];
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
      accounts: [
        {
          name: "marginfiGroup";
          isMut: true;
          isSigner: false;
        },
        {
          name: "marginfiAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "signer";
          isMut: true;
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
      accounts: [
        {
          name: "marginfiGroup";
          isMut: true;
          isSigner: false;
        },
        {
          name: "assetBank";
          isMut: true;
          isSigner: false;
        },
        {
          name: "assetPriceFeed";
          isMut: true;
          isSigner: false;
        },
        {
          name: "liabBank";
          isMut: true;
          isSigner: false;
        },
        {
          name: "liabPriceFeed";
          isMut: true;
          isSigner: false;
        },
        {
          name: "liquidatorMarginfiAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "signer";
          isMut: true;
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
          isMut: true;
          isSigner: false;
        },
        {
          name: "bank";
          isMut: true;
          isSigner: false;
          docs: [
            "PDA / seeds check ensures that provided account is legit, and use of the",
            "marginfi group + underlying mint guarantees unicity of bank per mint within a group"
          ];
        },
        {
          name: "liquidityVaultAuthority";
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
            name: "owner";
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
            name: "mintPk";
            type: "publicKey";
          },
          {
            name: "group";
            type: "publicKey";
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
            name: "config";
            type: {
              defined: "BankConfig";
            };
          },
          {
            name: "totalBorrowShares";
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
                  option: {
                    defined: "Balance";
                  };
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
            name: "bankPk";
            type: "publicKey";
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
    }
  ];
};

export const IDL: Marginfi = {
  version: "0.1.0",
  name: "marginfi",
  constants: [
    {
      name: "LIQUIDITY_VAULT_AUTHORITY_SEED",
      type: "bytes",
      value:
        "[108, 105, 113, 117, 105, 100, 105, 116, 121, 95, 118, 97, 117, 108, 116, 95, 97, 117, 116, 104]",
    },
    {
      name: "INSURANCE_VAULT_AUTHORITY_SEED",
      type: "bytes",
      value:
        "[105, 110, 115, 117, 114, 97, 110, 99, 101, 95, 118, 97, 117, 108, 116, 95, 97, 117, 116, 104]",
    },
    {
      name: "FEE_VAULT_AUTHORITY_SEED",
      type: "bytes",
      value:
        "[102, 101, 101, 95, 118, 97, 117, 108, 116, 95, 97, 117, 116, 104]",
    },
    {
      name: "LIQUIDITY_VAULT_SEED",
      type: "bytes",
      value:
        "[108, 105, 113, 117, 105, 100, 105, 116, 121, 95, 118, 97, 117, 108, 116]",
    },
    {
      name: "INSURANCE_VAULT_SEED",
      type: "bytes",
      value:
        "[105, 110, 115, 117, 114, 97, 110, 99, 101, 95, 118, 97, 117, 108, 116]",
    },
    {
      name: "FEE_VAULT_SEED",
      type: "bytes",
      value: "[102, 101, 101, 95, 118, 97, 117, 108, 116]",
    },
    {
      name: "LENDING_POOL_BANK_SEED",
      type: "bytes",
      value:
        "[108, 101, 110, 100, 105, 110, 103, 95, 112, 111, 111, 108, 95, 98, 97, 110, 107]",
    },
  ],
  instructions: [
    {
      name: "initializeMarginfiGroup",
      docs: ["Initialize a new Marginfi Group with initial config"],
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
      name: "initializeMarginfiAccount",
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
      accounts: [
        {
          name: "marginfiGroup",
          isMut: true,
          isSigner: false,
        },
        {
          name: "marginfiAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "signer",
          isMut: true,
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
          docs: ["Token mint is checked at transfer"],
        },
        {
          name: "bankLiquidityVault",
          isMut: true,
          isSigner: false,
          docs: ["TODO: Store bump on-chain"],
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
      accounts: [
        {
          name: "marginfiGroup",
          isMut: true,
          isSigner: false,
        },
        {
          name: "marginfiAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "signer",
          isMut: true,
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
      accounts: [
        {
          name: "marginfiGroup",
          isMut: true,
          isSigner: false,
        },
        {
          name: "assetBank",
          isMut: true,
          isSigner: false,
        },
        {
          name: "assetPriceFeed",
          isMut: true,
          isSigner: false,
        },
        {
          name: "liabBank",
          isMut: true,
          isSigner: false,
        },
        {
          name: "liabPriceFeed",
          isMut: true,
          isSigner: false,
        },
        {
          name: "liquidatorMarginfiAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "signer",
          isMut: true,
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
          isMut: true,
          isSigner: false,
        },
        {
          name: "bank",
          isMut: true,
          isSigner: false,
          docs: [
            "PDA / seeds check ensures that provided account is legit, and use of the",
            "marginfi group + underlying mint guarantees unicity of bank per mint within a group",
          ],
        },
        {
          name: "liquidityVaultAuthority",
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
            name: "owner",
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
            name: "mintPk",
            type: "publicKey",
          },
          {
            name: "group",
            type: "publicKey",
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
            name: "config",
            type: {
              defined: "BankConfig",
            },
          },
          {
            name: "totalBorrowShares",
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
                  option: {
                    defined: "Balance",
                  },
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
            name: "bankPk",
            type: "publicKey",
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
  ],
};
