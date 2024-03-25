use std::{collections::HashMap, str::FromStr};

use anchor_lang::AccountDeserialize;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};
use marginfi::state::marginfi_group::Bank;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Mint;

use crate::{
    db::{establish_connection, models::*, schema::*},
    error::{FetchEntityError, IndexingError},
};

pub struct EntityStore {
    pub rpc_client: RpcClient,
    pub db_connection: PgConnection,
    mint_cache: HashMap<String, MintData>,
    bank_cache: HashMap<String, BankData>,
    account_cache: HashMap<String, AccountData>,
    authority_cache: HashMap<String, UserData>,
}

impl EntityStore {
    pub fn new(rpc_endpoint: String, db_connection_url: String) -> Self {
        let rpc_client = RpcClient::new_with_commitment(
            rpc_endpoint,
            CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            },
        );

        let db_connection = establish_connection(db_connection_url);

        EntityStore {
            rpc_client,
            db_connection,
            mint_cache: HashMap::new(),
            bank_cache: HashMap::new(),
            account_cache: HashMap::new(),
            authority_cache: HashMap::new(),
        }
    }

    pub fn get_or_fetch_mint(&mut self, address: &str) -> Result<MintData, IndexingError> {
        let maybe_mint = self.mint_cache.get(&address.to_string());

        if let Some(mint) = maybe_mint {
            Ok(mint.clone())
        } else {
            let mint = MintData::fetch(&self.rpc_client, &mut self.db_connection, address)?;
            Ok(mint)
        }
    }

    pub fn get_or_fetch_bank(&mut self, address: &str) -> Result<BankData, IndexingError> {
        let maybe_bank = self.bank_cache.get(&address.to_string());

        if let Some(bank) = maybe_bank {
            Ok(bank.clone())
        } else {
            let bank = BankData::fetch(&self.rpc_client, &mut self.db_connection, address)?;
            Ok(bank)
        }
    }
}

#[derive(Debug, Clone)]
pub struct MintData {
    pub id: Option<i32>,
    pub address: String,
    pub symbol: String,
    pub decimals: i16,
}

impl MintData {
    pub fn fetch(
        rpc_client: &RpcClient,
        db_connection: &mut PgConnection,
        address: &str,
    ) -> Result<MintData, IndexingError> {
        let db_record = MintData::fetch_from_db(db_connection, address)?;

        if let Some(db_record) = db_record {
            return Ok(db_record);
        }

        let mint = MintData::fetch_from_rpc(rpc_client, address)?;

        Ok(mint)
    }

    fn fetch_from_db(
        db_connection: &mut PgConnection,
        address: &str,
    ) -> Result<Option<MintData>, IndexingError> {
        let db_records = mints::dsl::mints
            .filter(mints::address.eq(address.to_string()))
            .select(Mints::as_select())
            .limit(1)
            .load(db_connection)
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "mint".to_string(),
                    e.to_string(),
                ))
            })?;

        if db_records.is_empty() {
            return Ok(None);
        }

        let db_record = db_records.get(0).unwrap();

        Ok(Some(MintData {
            id: Some(db_record.id),
            address: db_record.address.clone(),
            symbol: db_record.symbol.clone(),
            decimals: db_record.decimals,
        }))
    }

    fn fetch_from_rpc(rpc_client: &RpcClient, address: &str) -> Result<MintData, IndexingError> {
        let mint_data = rpc_client
            .get_account_data(&Pubkey::from_str(address).unwrap())
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "mint".to_string(),
                    e.to_string(),
                ))
            })?;

        let mint = Mint::unpack_from_slice(&mint_data).map_err(|e| {
            IndexingError::FailedToFetchEntity(FetchEntityError::UnpackError(
                "mint".to_string(),
                e.to_string(),
            ))
        })?;

        Ok(MintData {
            id: None,
            address: address.to_string(),
            symbol: "".to_string(),
            decimals: mint.decimals as i16,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BankData {
    pub id: Option<i32>,
    pub address: String,
    pub mint: MintData,
}

impl BankData {
    pub fn fetch(
        rpc_client: &RpcClient,
        db_connection: &mut PgConnection,
        address: &str,
    ) -> Result<BankData, IndexingError> {
        let db_record = BankData::fetch_from_db(db_connection, address)?;

        if let Some(db_record) = db_record {
            return Ok(db_record);
        }

        let mint = BankData::fetch_from_rpc(rpc_client, db_connection, address)?;

        Ok(mint)
    }

    fn fetch_from_db(
        db_connection: &mut PgConnection,
        address: &str,
    ) -> Result<Option<BankData>, IndexingError> {
        let db_records = banks::dsl::banks
            .filter(banks::address.eq(address.to_string()))
            .select(Banks::as_select())
            .limit(1)
            .load(db_connection)
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "bank".to_string(),
                    e.to_string(),
                ))
            })?;

        if db_records.is_empty() {
            return Ok(None);
        }

        let db_record = db_records.get(0).unwrap();

        let mint_data = MintData::fetch_from_db(db_connection, &db_record.mint_id.to_string())
            .unwrap()
            .unwrap();

        Ok(Some(BankData {
            id: Some(db_record.id),
            address: db_record.address.clone(),
            mint: mint_data,
        }))
    }

    fn fetch_from_rpc(
        rpc_client: &RpcClient,
        db_connection: &mut PgConnection,
        address: &str,
    ) -> Result<BankData, IndexingError> {
        let mint_data = rpc_client
            .get_account_data(&Pubkey::from_str(address).unwrap())
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "bank".to_string(),
                    e.to_string(),
                ))
            })?;

        let bank = Bank::try_deserialize(&mut mint_data.as_slice()).map_err(|e| {
            IndexingError::FailedToFetchEntity(FetchEntityError::UnpackError(
                "bank".to_string(),
                e.to_string(),
            ))
        })?;

        let mint_data = MintData::fetch(rpc_client, db_connection, &bank.mint.to_string())?;

        Ok(BankData {
            id: None,
            address: address.to_string(),
            mint: mint_data,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AccountData {
    pub id: Option<i32>,
    pub address: String,
    pub user_id: i32,
}

impl AccountData {
    fn get_cache_or_fetch(&mut self, address: &str) -> AccountData {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct UserData {
    pub id: Option<i32>,
    pub address: String,
}

impl UserData {
    fn get_cache_or_fetch(&mut self, address: &str) -> UserData {
        todo!()
    }
}
