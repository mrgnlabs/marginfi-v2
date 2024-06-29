use std::{collections::HashMap, str::FromStr};

use anchor_lang::AccountDeserialize;
use diesel::{
    prelude::*, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper,
};
use marginfi::state::{
    marginfi_account::MarginfiAccount,
    marginfi_group::{Bank, MarginfiGroup},
};
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
    group_cache: HashMap<String, GroupData>,
    bank_cache: HashMap<String, BankData>,
    account_cache: HashMap<String, AccountData>,
    user_cache: HashMap<String, UserData>,
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
            group_cache: HashMap::new(),
            bank_cache: HashMap::new(),
            account_cache: HashMap::new(),
            user_cache: HashMap::new(),
        }
    }

    pub fn get_or_fetch_mint(&mut self, address: &str) -> Result<MintData, IndexingError> {
        let maybe_mint = self.mint_cache.get(&address.to_string());

        if let Some(mint) = maybe_mint {
            Ok(mint.clone())
        } else {
            let mint = MintData::fetch(self, address)?;

            if mint.id.is_some() {
                self.mint_cache.insert(address.to_string(), mint.clone());
            }

            Ok(mint)
        }
    }

    pub fn get_or_fetch_group(&mut self, address: &str) -> Result<GroupData, IndexingError> {
        let maybe_group = self.group_cache.get(&address.to_string());

        if let Some(group) = maybe_group {
            Ok(group.clone())
        } else {
            let group = GroupData::fetch(self, address)?;

            if group.id.is_some() {
                self.group_cache.insert(address.to_string(), group.clone());
            }

            Ok(group)
        }
    }

    pub fn get_or_fetch_group_no_rpc(
        &mut self,
        address: &str,
    ) -> Result<Option<GroupData>, IndexingError> {
        let maybe_group = self.group_cache.get(&address.to_string());

        if let Some(group) = maybe_group {
            Ok(Some(group.clone()))
        } else {
            let maybe_group = GroupData::fetch_from_db(self, address)?;

            if let Some(group) = &maybe_group {
                self.group_cache.insert(address.to_string(), group.clone());
            }

            Ok(maybe_group)
        }
    }

    pub fn get_or_fetch_bank(&mut self, address: &str) -> Result<BankData, IndexingError> {
        let maybe_bank = self.bank_cache.get(&address.to_string());

        if let Some(bank) = maybe_bank {
            Ok(bank.clone())
        } else {
            let bank = BankData::fetch(self, address)?;

            if bank.id.is_some() {
                self.bank_cache.insert(address.to_string(), bank.clone());
            }

            Ok(bank)
        }
    }

    pub fn get_or_fetch_bank_no_rpc(
        &mut self,
        address: &str,
    ) -> Result<Option<BankData>, IndexingError> {
        let maybe_bank = self.bank_cache.get(&address.to_string());

        if let Some(bank) = maybe_bank {
            Ok(Some(bank.clone()))
        } else {
            let maybe_bank = BankData::fetch_from_db(self, address)?;

            if let Some(bank) = &maybe_bank {
                self.bank_cache.insert(address.to_string(), bank.clone());
            }

            Ok(maybe_bank)
        }
    }

    pub fn get_or_fetch_account(&mut self, address: &str) -> Result<AccountData, IndexingError> {
        let maybe_account = { self.account_cache.get(&address.to_string()).cloned() };

        if let Some(account) = maybe_account {
            Ok(account.clone())
        } else {
            let account = AccountData::fetch(self, address)?;

            if account.id.is_some() {
                self.account_cache
                    .insert(address.to_string(), account.clone());
            }

            Ok(account)
        }
    }

    pub fn get_or_fetch_account_no_rpc(
        &mut self,
        address: &str,
    ) -> Result<Option<AccountData>, IndexingError> {
        let maybe_account = { self.account_cache.get(&address.to_string()).cloned() };

        if let Some(account) = maybe_account {
            Ok(Some(account))
        } else {
            let maybe_account = AccountData::fetch_from_db(self, address)?;

            if let Some(account) = &maybe_account {
                self.account_cache
                    .insert(address.to_string(), account.clone());
            }

            Ok(maybe_account)
        }
    }

    pub fn get_or_fetch_user(&mut self, address: &str) -> Result<UserData, IndexingError> {
        let maybe_user = self.user_cache.get(&address.to_string());

        if let Some(user) = maybe_user {
            Ok(user.clone())
        } else {
            let user = UserData::fetch(&mut self.db_connection, address)?;

            if user.id.is_some() {
                self.user_cache.insert(address.to_string(), user.clone());
            }

            Ok(user)
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
    pub fn fetch(entity_store: &mut EntityStore, address: &str) -> Result<Self, IndexingError> {
        let db_record = Self::fetch_from_db(entity_store, address)?;

        if let Some(db_record) = db_record {
            return Ok(db_record);
        }

        let mint = Self::fetch_from_rpc(&entity_store.rpc_client, address)?;

        Ok(mint)
    }

    fn fetch_from_db(
        entity_store: &mut EntityStore,
        address: &str,
    ) -> Result<Option<Self>, IndexingError> {
        let db_records = mints::dsl::mints
            .filter(mints::address.eq(address.to_string()))
            .select(Mints::as_select())
            .limit(1)
            .load(&mut entity_store.db_connection)
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

        Ok(Some(Self {
            id: Some(db_record.id),
            address: db_record.address.clone(),
            symbol: db_record.symbol.clone(),
            decimals: db_record.decimals,
        }))
    }

    fn fetch_from_rpc(rpc_client: &RpcClient, address: &str) -> Result<Self, IndexingError> {
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

        Ok(Self {
            id: None,
            address: address.to_string(),
            symbol: "".to_string(),
            decimals: mint.decimals as i16,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GroupData {
    pub id: Option<i32>,
    pub address: String,
    pub admin: String,
}

impl GroupData {
    pub fn fetch(entity_store: &mut EntityStore, address: &str) -> Result<Self, IndexingError> {
        let db_record = Self::fetch_from_db(entity_store, address)?;

        if let Some(db_record) = db_record {
            return Ok(db_record);
        }

        let group = Self::fetch_from_rpc(&entity_store.rpc_client, address)?;

        Ok(group)
    }

    fn fetch_from_db(
        entity_store: &mut EntityStore,
        address: &str,
    ) -> Result<Option<Self>, IndexingError> {
        let db_records = groups::dsl::groups
            .filter(groups::address.eq(address.to_string()))
            .select(Groups::as_select())
            .limit(1)
            .load(&mut entity_store.db_connection)
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "group".to_string(),
                    e.to_string(),
                ))
            })?;

        if db_records.is_empty() {
            return Ok(None);
        }

        let db_record = db_records.get(0).unwrap();

        Ok(Some(Self {
            id: Some(db_record.id),
            address: db_record.address.clone(),
            admin: db_record.admin.clone(),
        }))
    }

    fn fetch_from_rpc(rpc_client: &RpcClient, address: &str) -> Result<Self, IndexingError> {
        let group_data = rpc_client
            .get_account_data(&Pubkey::from_str(address).unwrap())
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "group".to_string(),
                    e.to_string(),
                ))
            })?;

        let group = MarginfiGroup::try_deserialize(&mut group_data.as_slice()).map_err(|e| {
            IndexingError::FailedToFetchEntity(FetchEntityError::UnpackError(
                "group".to_string(),
                e.to_string(),
            ))
        })?;

        Ok(Self {
            id: None,
            address: address.to_string(),
            admin: group.admin.to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct BankData {
    pub id: Option<i32>,
    pub address: String,
    pub mint: MintData,
    pub group: GroupData,
}

impl BankData {
    pub fn fetch(entity_store: &mut EntityStore, address: &str) -> Result<Self, IndexingError> {
        let db_record = Self::fetch_from_db(entity_store, address)?;

        if let Some(db_record) = db_record {
            return Ok(db_record);
        }

        let mint = Self::fetch_from_rpc(entity_store, address)?;

        Ok(mint)
    }

    fn fetch_from_db(
        entity_store: &mut EntityStore,
        address: &str,
    ) -> Result<Option<Self>, IndexingError> {
        let db_records = banks::dsl::banks
            .filter(banks::address.eq(address.to_string()))
            .select(Banks::as_select())
            .limit(1)
            .load(&mut entity_store.db_connection)
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

        let mint = mints::dsl::mints
            .find(&db_record.mint_id)
            .select(Mints::as_select())
            .first(&mut entity_store.db_connection)
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "mint".to_string(),
                    e.to_string(),
                ))
            })?;

        let mint_data = if let Some(mint_data) = entity_store.mint_cache.get(&mint.address) {
            mint_data.clone()
        } else {
            MintData::fetch_from_db(entity_store, &mint.address)?
                .expect(&format!("Mint {:?} should be in DB.", mint.address))
        };

        let group = groups::dsl::groups
            .find(&db_record.group_id)
            .select(Groups::as_select())
            .first(&mut entity_store.db_connection)
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "group".to_string(),
                    e.to_string(),
                ))
            })?;

        let group_data = if let Some(group_data) = entity_store.group_cache.get(&group.address) {
            group_data.clone()
        } else {
            GroupData::fetch_from_db(entity_store, &group.address)?
                .expect(&format!("Group {:?} should be in DB.", group.address))
        };

        Ok(Some(Self {
            id: Some(db_record.id),
            address: db_record.address.clone(),
            mint: MintData {
                id: mint_data.id,
                address: mint_data.address,
                symbol: mint_data.symbol,
                decimals: mint_data.decimals,
            },
            group: group_data,
        }))
    }

    fn fetch_from_rpc(
        entity_store: &mut EntityStore,
        address: &str,
    ) -> Result<Self, IndexingError> {
        let data = entity_store
            .rpc_client
            .get_account_data(&Pubkey::from_str(address).unwrap())
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "bank".to_string(),
                    e.to_string(),
                ))
            })?;

        let bank = Bank::try_deserialize(&mut data.as_slice()).map_err(|e| {
            IndexingError::FailedToFetchEntity(FetchEntityError::UnpackError(
                "bank".to_string(),
                e.to_string(),
            ))
        })?;

        let mint_data = MintData::fetch(entity_store, &bank.mint.to_string())?;

        let group_data = GroupData::fetch(entity_store, &bank.group.to_string())?;

        Ok(Self {
            id: None,
            address: address.to_string(),
            mint: mint_data,
            group: group_data,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AccountData {
    pub id: Option<i32>,
    pub address: String,
    pub authority: UserData,
    pub group: GroupData,
}

impl AccountData {
    pub fn fetch(entity_store: &mut EntityStore, address: &str) -> Result<Self, IndexingError> {
        let db_record = Self::fetch_from_db(entity_store, address)?;

        if let Some(db_record) = db_record {
            return Ok(db_record);
        }

        let account = Self::fetch_from_rpc(entity_store, address)?;

        Ok(account)
    }

    fn fetch_from_db(
        entity_store: &mut EntityStore,
        address: &str,
    ) -> Result<Option<Self>, IndexingError> {
        let maybe_db_record = accounts::dsl::accounts
            .filter(accounts::address.eq(address.to_string()))
            .select(Accounts::as_select())
            .get_result(&mut entity_store.db_connection)
            .optional()
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "account".to_string(),
                    e.to_string(),
                ))
            })?;

        if maybe_db_record.is_none() {
            return Ok(None);
        }

        let db_record = maybe_db_record.unwrap();

        let authority = users::dsl::users
            .find(&db_record.user_id)
            .select(Users::as_select())
            .first(&mut entity_store.db_connection)
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "account".to_string(),
                    e.to_string(),
                ))
            })?;

        let user_data = if let Some(user_data) = entity_store.user_cache.get(&authority.address) {
            user_data.clone()
        } else {
            UserData::fetch_from_db(&mut entity_store.db_connection, &authority.address)?
                .expect(&format!("User {:?} should be in DB.", authority.address))
        };

        let group = groups::dsl::groups
            .find(&db_record.group_id)
            .select(Groups::as_select())
            .first(&mut entity_store.db_connection)
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "group".to_string(),
                    e.to_string(),
                ))
            })?;

        let group_data = if let Some(group_data) = entity_store.group_cache.get(&group.address) {
            group_data.clone()
        } else {
            GroupData::fetch_from_db(entity_store, &group.address)?
                .expect(&format!("Group {:?} should be in DB.", group.address))
        };

        Ok(Some(Self {
            id: Some(db_record.id),
            address: db_record.address.clone(),
            authority: user_data,
            group: group_data,
        }))
    }

    fn fetch_from_rpc(
        entity_store: &mut EntityStore,
        address: &str,
    ) -> Result<Self, IndexingError> {
        let data = entity_store
            .rpc_client
            .get_account_data(&Pubkey::from_str(address).unwrap())
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "account".to_string(),
                    e.to_string(),
                ))
            })?;

        let account = MarginfiAccount::try_deserialize(&mut data.as_slice()).map_err(|e| {
            IndexingError::FailedToFetchEntity(FetchEntityError::UnpackError(
                "account".to_string(),
                e.to_string(),
            ))
        })?;

        let user_address = account.authority.to_string();
        let user_data = if let Some(user_data) = entity_store.user_cache.get(&user_address) {
            user_data.clone()
        } else {
            UserData::fetch(&mut entity_store.db_connection, &user_address)?
        };

        let group_address = account.group.to_string();
        let group_data = if let Some(group_data) = entity_store.group_cache.get(&group_address) {
            group_data.clone()
        } else {
            GroupData::fetch(entity_store, &group_address)?
        };

        Ok(Self {
            id: None,
            address: address.to_string(),
            authority: user_data,
            group: group_data,
        })
    }
}

#[derive(Debug, Clone)]
pub struct UserData {
    pub id: Option<i32>,
    pub address: String,
}

impl UserData {
    pub fn fetch(db_connection: &mut PgConnection, address: &str) -> Result<Self, IndexingError> {
        let db_record = Self::fetch_from_db(db_connection, address)?;

        if let Some(db_record) = db_record {
            return Ok(db_record);
        }

        Ok(Self {
            id: None,
            address: address.to_string(),
        })
    }

    fn fetch_from_db(
        db_connection: &mut PgConnection,
        address: &str,
    ) -> Result<Option<Self>, IndexingError> {
        let maybe_db_record = users::dsl::users
            .filter(users::address.eq(address.to_string()))
            .select(Users::as_select())
            .get_result(db_connection)
            .optional()
            .map_err(|e| {
                IndexingError::FailedToFetchEntity(FetchEntityError::FetchError(
                    "user".to_string(),
                    e.to_string(),
                ))
            })?;

        if maybe_db_record.is_none() {
            return Ok(None);
        }

        let db_record = maybe_db_record.unwrap();

        Ok(Some(Self {
            id: Some(db_record.id),
            address: db_record.address.clone(),
        }))
    }
}
