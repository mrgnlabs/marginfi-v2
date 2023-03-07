pub mod solana {
    pub mod storage {
        pub mod confirmed_block {
            tonic::include_proto!("solana.storage.confirmed_block");
        }
    }
}

pub mod geyser {
    tonic::include_proto!("geyser");
}

pub mod gcp_pubsub {
    tonic::include_proto!("gcp_pubsub");
}

pub use geyser::*;
pub use solana::storage::confirmed_block::*;

mod conversion {
    use crate::utils::errors::GeyserServiceError;
    use itertools::Itertools;
    use solana_account_decoder::parse_token::UiTokenAmount;
    use solana_sdk::account::Account;
    use solana_sdk::{
        hash::Hash,
        instruction::CompiledInstruction,
        message::{
            legacy,
            v0::{self, LoadedAddresses, MessageAddressTableLookup},
            MessageHeader, VersionedMessage,
        },
        pubkey::Pubkey,
        signature::Signature,
        transaction::{TransactionError, VersionedTransaction},
    };
    use solana_transaction_status::{
        InnerInstructions, Reward, RewardType, TransactionStatusMeta, TransactionTokenBalance,
        VersionedTransactionWithStatusMeta,
    };

    impl TryFrom<super::SubscribeUpdateAccountInfo> for Account {
        type Error = GeyserServiceError;

        fn try_from(
            account_data_proto: super::SubscribeUpdateAccountInfo,
        ) -> Result<Self, Self::Error> {
            Ok(Self {
                data: account_data_proto.data,
                owner: Pubkey::try_from(account_data_proto.owner.as_slice())
                    .map_err(|_| GeyserServiceError::ProtoMessageConversionFailed)?,
                lamports: account_data_proto.lamports,
                executable: account_data_proto.executable,
                rent_epoch: account_data_proto.rent_epoch,
            })
        }
    }

    impl From<super::CompiledInstruction> for CompiledInstruction {
        fn from(instruction_proto: super::CompiledInstruction) -> Self {
            Self {
                program_id_index: instruction_proto.program_id_index as u8,
                accounts: instruction_proto.accounts,
                data: instruction_proto.data,
            }
        }
    }

    impl From<super::MessageHeader> for MessageHeader {
        fn from(header_proto: super::MessageHeader) -> Self {
            Self {
                num_required_signatures: header_proto.num_required_signatures as u8,
                num_readonly_signed_accounts: header_proto.num_readonly_signed_accounts as u8,
                num_readonly_unsigned_accounts: header_proto.num_readonly_unsigned_accounts as u8,
            }
        }
    }

    impl TryFrom<super::MessageAddressTableLookup> for MessageAddressTableLookup {
        type Error = GeyserServiceError;

        fn try_from(lut_proto: super::MessageAddressTableLookup) -> Result<Self, Self::Error> {
            Ok(Self {
                account_key: Pubkey::try_from(lut_proto.account_key.as_slice())
                    .map_err(|_| GeyserServiceError::ProtoMessageConversionFailed)?,
                writable_indexes: lut_proto.writable_indexes,
                readonly_indexes: lut_proto.readonly_indexes,
            })
        }
    }

    impl TryFrom<super::Message> for legacy::Message {
        type Error = GeyserServiceError;

        fn try_from(message_proto: super::Message) -> Result<Self, Self::Error> {
            let message_header_proto = message_proto.header.expect("missing message header");
            Ok(Self {
                account_keys: message_proto
                    .account_keys
                    .iter()
                    .map(|address_bytes| Pubkey::try_from(address_bytes.as_slice()))
                    .into_iter()
                    .collect::<Result<_, _>>()
                    .map_err(|_| GeyserServiceError::ProtoMessageConversionFailed)?,
                header: message_header_proto.into(),
                recent_blockhash: Hash::new(&message_proto.recent_blockhash),
                instructions: message_proto.instructions.into_iter().map_into().collect(),
            })
        }
    }

    impl TryFrom<super::Message> for v0::Message {
        type Error = GeyserServiceError;

        fn try_from(message_proto: super::Message) -> Result<Self, Self::Error> {
            let message_header_proto = message_proto.header.expect("missing message header");

            Ok(Self {
                header: message_header_proto.into(),
                account_keys: message_proto
                    .account_keys
                    .iter()
                    .map(|address_bytes| Pubkey::try_from(address_bytes.as_slice()))
                    .into_iter()
                    .collect::<Result<_, _>>()
                    .map_err(|_| GeyserServiceError::ProtoMessageConversionFailed)?,
                recent_blockhash: Hash::new(&message_proto.recent_blockhash),
                instructions: message_proto.instructions.into_iter().map_into().collect(),
                address_table_lookups: message_proto
                    .address_table_lookups
                    .into_iter()
                    .map(TryFrom::try_from)
                    .into_iter()
                    .collect::<Result<_, _>>()?,
            })
        }
    }

    impl TryFrom<super::Message> for VersionedMessage {
        type Error = GeyserServiceError;

        fn try_from(message_proto: super::Message) -> Result<Self, Self::Error> {
            Ok(match message_proto.versioned {
                false => VersionedMessage::Legacy(message_proto.try_into()?),
                true => VersionedMessage::V0(message_proto.try_into()?),
            })
        }
    }

    impl TryFrom<super::Transaction> for VersionedTransaction {
        type Error = GeyserServiceError;

        fn try_from(transaction_proto: super::Transaction) -> Result<Self, Self::Error> {
            let message_proto = transaction_proto.message.expect("missing message");

            Ok(Self {
                signatures: transaction_proto
                    .signatures
                    .iter()
                    .map(|sig_bytes| Signature::new(sig_bytes))
                    .collect_vec(),
                message: message_proto.try_into()?,
            })
        }
    }

    impl From<super::Reward> for Reward {
        fn from(reward_proto: super::Reward) -> Self {
            Self {
                pubkey: reward_proto.pubkey,
                lamports: reward_proto.lamports,
                post_balance: reward_proto.post_balance,
                reward_type: match reward_proto.reward_type {
                    0 => None,
                    1 => Some(RewardType::Fee),
                    2 => Some(RewardType::Rent),
                    3 => Some(RewardType::Staking),
                    4 => Some(RewardType::Voting),
                    _ => panic!("unknown reward type {}", reward_proto.reward_type),
                },
                commission: reward_proto.commission.parse::<u8>().ok(),
            }
        }
    }

    impl From<super::InnerInstructions> for InnerInstructions {
        fn from(inner_instructions_proto: super::InnerInstructions) -> Self {
            Self {
                index: inner_instructions_proto.index as u8,
                instructions: inner_instructions_proto
                    .instructions
                    .into_iter()
                    .map_into()
                    .collect(),
            }
        }
    }

    impl From<super::UiTokenAmount> for UiTokenAmount {
        fn from(amount_proto: super::UiTokenAmount) -> Self {
            Self {
                ui_amount: Some(amount_proto.ui_amount),
                decimals: amount_proto.decimals as u8,
                amount: amount_proto.amount,
                ui_amount_string: amount_proto.ui_amount_string,
            }
        }
    }

    impl From<super::TokenBalance> for TransactionTokenBalance {
        fn from(token_balance_proto: super::TokenBalance) -> Self {
            Self {
                account_index: token_balance_proto.account_index as u8,
                mint: token_balance_proto.mint,
                ui_token_amount: token_balance_proto.ui_token_amount.unwrap().into(), // Why `Option`? proto field is not optional
                owner: token_balance_proto.owner,
                program_id: token_balance_proto.program_id,
            }
        }
    }

    impl TryFrom<super::TransactionStatusMeta> for TransactionStatusMeta {
        type Error = GeyserServiceError;

        fn try_from(meta_proto: super::TransactionStatusMeta) -> Result<Self, Self::Error> {
            let loaded_writable_addresses: Vec<Pubkey> = meta_proto
                .loaded_writable_addresses
                .iter()
                .map(|address_bytes| Pubkey::try_from(address_bytes.as_slice()))
                .into_iter()
                .collect::<Result<_, _>>()
                .map_err(|_| GeyserServiceError::ProtoMessageConversionFailed)?;
            let loaded_readable_addresses = meta_proto
                .loaded_readonly_addresses
                .iter()
                .map(|address_bytes| Pubkey::try_from(address_bytes.as_slice()))
                .into_iter()
                .collect::<Result<_, _>>()
                .map_err(|_| GeyserServiceError::ProtoMessageConversionFailed)?;

            Ok(Self {
                status: match meta_proto.err {
                    None => Ok(()),
                    Some(err) => Err(bincode::deserialize::<TransactionError>(&err.err).unwrap()),
                },
                fee: meta_proto.fee,
                pre_balances: meta_proto.pre_balances,
                post_balances: meta_proto.post_balances,
                inner_instructions: Some(
                    meta_proto
                        .inner_instructions
                        .into_iter()
                        .map_into()
                        .collect(),
                ),
                log_messages: (!meta_proto.log_messages_none).then_some(meta_proto.log_messages),
                pre_token_balances: Some(
                    meta_proto
                        .pre_token_balances
                        .into_iter()
                        .map_into()
                        .collect(),
                ),
                post_token_balances: Some(
                    meta_proto
                        .post_token_balances
                        .into_iter()
                        .map_into()
                        .collect(),
                ),
                rewards: Some(meta_proto.rewards.into_iter().map_into().collect()),
                loaded_addresses: LoadedAddresses {
                    writable: loaded_writable_addresses,
                    readonly: loaded_readable_addresses,
                },
                return_data: None,
                compute_units_consumed: None,
            })
        }
    }

    impl TryFrom<super::SubscribeUpdateTransactionInfo> for VersionedTransactionWithStatusMeta {
        type Error = GeyserServiceError;

        fn try_from(
            transaction_info: super::SubscribeUpdateTransactionInfo,
        ) -> Result<Self, Self::Error> {
            Ok(Self {
                transaction: transaction_info
                    .transaction
                    .expect("missing transaction")
                    .try_into()?,
                meta: transaction_info
                    .meta
                    .expect("missing transaction meta")
                    .try_into()?,
            })
        }
    }
}
