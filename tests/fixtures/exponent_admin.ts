/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/exponent_admin.json`.
 */
export type ExponentAdmin = {
  "address": "3D6ojc8vBfDteLBDTTRznZbZRh7bkEGQaYqNkudoTCBQ",
  "metadata": {
    "name": "exponentAdmin",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "acceptInvitation",
      "discriminator": [
        114,
        70,
        62,
        248,
        204,
        49,
        98,
        239
      ],
      "accounts": [
        {
          "name": "adminAccount",
          "writable": true
        },
        {
          "name": "newUberAdmin",
          "signer": true
        }
      ],
      "args": []
    },
    {
      "name": "addPrincipleAdmin",
      "discriminator": [
        176,
        92,
        90,
        244,
        94,
        195,
        223,
        17
      ],
      "accounts": [
        {
          "name": "adminAccount",
          "writable": true
        },
        {
          "name": "newAdmin"
        },
        {
          "name": "feePayer",
          "writable": true,
          "signer": true
        },
        {
          "name": "uberAdmin",
          "signer": true
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": [
        {
          "name": "principle",
          "type": {
            "defined": {
              "name": "principle"
            }
          }
        }
      ]
    },
    {
      "name": "initializeAdmin",
      "discriminator": [
        35,
        176,
        8,
        143,
        42,
        160,
        61,
        158
      ],
      "accounts": [
        {
          "name": "adminAccount",
          "writable": true
        },
        {
          "name": "feePayer",
          "writable": true,
          "signer": true
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": []
    },
    {
      "name": "inviteAdmin",
      "discriminator": [
        41,
        60,
        9,
        111,
        149,
        16,
        239,
        88
      ],
      "accounts": [
        {
          "name": "adminAccount",
          "writable": true
        },
        {
          "name": "uberAdmin",
          "signer": true
        },
        {
          "name": "proposedAdmin"
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": []
    },
    {
      "name": "reallocAdmin",
      "discriminator": [
        161,
        113,
        63,
        40,
        129,
        42,
        14,
        84
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "adminAccount",
          "writable": true
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": [
        {
          "name": "newLen",
          "type": "u16"
        }
      ]
    },
    {
      "name": "removePrincipleAdmin",
      "discriminator": [
        38,
        60,
        155,
        176,
        227,
        130,
        205,
        154
      ],
      "accounts": [
        {
          "name": "adminAccount",
          "writable": true
        },
        {
          "name": "adminToRemove"
        },
        {
          "name": "uberAdmin",
          "writable": true,
          "signer": true
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": [
        {
          "name": "principle",
          "type": {
            "defined": {
              "name": "principle"
            }
          }
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
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "unauthorized",
      "msg": "unauthorized"
    },
    {
      "code": 6001,
      "name": "noProposedAdmin",
      "msg": "There is no proposed admin"
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
      "name": "principle",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "marginfiStandard"
          },
          {
            "name": "collectTreasury"
          },
          {
            "name": "kaminoLendStandard"
          },
          {
            "name": "exponentCore"
          },
          {
            "name": "changeStatusFlags"
          },
          {
            "name": "jitoRestaking"
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
    }
  ]
};
