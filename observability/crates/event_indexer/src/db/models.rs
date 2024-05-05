use crate::db::schema::*;
use diesel::{prelude::*, sql_types::*};
use rust_decimal::Decimal;

#[derive(QueryableByName)]
struct IdResult {
    #[sql_type = "Integer"]
    pub id: i32,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = mints)]
pub struct Mints {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub address: String,
    pub symbol: String,
    pub decimals: i16,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(belongs_to(Mints, foreign_key = mint_id))]
#[diesel(table_name = banks)]
pub struct Banks {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub address: String,
    pub mint_id: i32,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = users)]
pub struct Users {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub address: String,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable, Associations)]
#[diesel(table_name = accounts)]
#[diesel(belongs_to(Users, foreign_key = user_id))]
pub struct Accounts {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub address: String,
    pub user_id: i32,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = unknown_events)]
pub struct UnknownEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,
}

#[derive(Default, Debug, Queryable, Selectable, Insertable, Associations)]
#[diesel(table_name = create_account_events)]
#[diesel(belongs_to(Accounts, foreign_key = account_id), belongs_to(Users, foreign_key = authority_id))]
pub struct CreateAccountEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
}

impl CreateAccountEvents {
    pub fn insert(
        db_connection: &mut PgConnection,
        account_id: i32,
        authority_id: i32,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: String,
        tx_sig: String,
    ) -> QueryResult<Option<i32>> {
        let create_account_event = CreateAccountEvents {
            timestamp,
            slot,
            tx_sig,
            call_stack,
            in_flashloan,
            account_id,
            authority_id,
            ..Default::default()
        };

        diesel::insert_into(create_account_events::table)
            .values(&create_account_event)
            .on_conflict_do_nothing()
            .returning(create_account_events::id)
            .get_result(db_connection)
            .optional()
    }

    pub fn insert_with_dependents(
        connection: &mut PgConnection,
        account: &str,
        authority: &str,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: &str,
        tx_sig: &str,
    ) -> QueryResult<i32> {
        let sql = r#"
        WITH upsert_authority AS (
            INSERT INTO users (address)
            VALUES ($1)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_authority AS (
            SELECT id FROM users WHERE address = $1
        ), combined_authority AS (
            SELECT id FROM upsert_authority
            UNION ALL
            SELECT id FROM existing_authority
            LIMIT 1
        ),
        upsert_account AS (
            INSERT INTO accounts (address, user_id)
            VALUES ($2, (SELECT id FROM combined_authority))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_account AS (
            SELECT id FROM accounts WHERE address = $2
        ), combined_account AS (
            SELECT id FROM upsert_account
            UNION ALL
            SELECT id FROM existing_account
            LIMIT 1
        )
        INSERT INTO create_account_events (timestamp, slot, tx_sig, in_flashloan, call_stack, account_id, authority_id)
        VALUES ($3, $4, $5, $6, $7, (SELECT id FROM combined_account), (SELECT id FROM combined_authority))
        RETURNING id;
        "#;

        let id = diesel::sql_query(sql)
            .bind::<Text, _>(authority)
            .bind::<Text, _>(account)
            .bind::<Timestamp, _>(timestamp)
            .bind::<Numeric, _>(slot)
            .bind::<Text, _>(tx_sig)
            .bind::<Bool, _>(in_flashloan)
            .bind::<Text, _>(call_stack)
            .load::<IdResult>(connection)?
            .pop()
            .expect("Expected at least one id")
            .id;

        Ok(id)
    }
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = transfer_account_authority_events)]
pub struct TransferAccountAuthorityEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub old_authority_id: i32,
    pub new_authority_id: i32,
}

impl TransferAccountAuthorityEvents {
    pub fn insert(
        db_connection: &mut PgConnection,
        account_id: i32,
        old_authority_id: i32,
        new_authority_id: i32,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: String,
        tx_sig: String,
    ) -> QueryResult<Option<i32>> {
        let transfer_account_authority_event = TransferAccountAuthorityEvents {
            timestamp,
            slot,
            tx_sig,
            call_stack,
            in_flashloan,
            account_id,
            old_authority_id,
            new_authority_id,
            ..Default::default()
        };

        diesel::insert_into(transfer_account_authority_events::table)
            .values(&transfer_account_authority_event)
            .on_conflict_do_nothing()
            .returning(transfer_account_authority_events::id)
            .get_result(db_connection)
            .optional()
    }

    pub fn insert_with_dependents(
        connection: &mut PgConnection,
        account: &str,
        old_authority: &str,
        new_authority: &str,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: &str,
        tx_sig: &str,
    ) -> QueryResult<i32> {
        let sql = r#"
        WITH upsert_old_authority AS (
            INSERT INTO users (address)
            VALUES ($1)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_old_authority AS (
            SELECT id FROM users WHERE address = $1
        ), combined_old_authority AS (
            SELECT id FROM upsert_old_authority
            UNION ALL
            SELECT id FROM existing_old_authority
            LIMIT 1
        ),
        upsert_new_authority AS (
            INSERT INTO users (address)
            VALUES ($2)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_new_authority AS (
            SELECT id FROM users WHERE address = $2
        ), combined_new_authority AS (
            SELECT id FROM upsert_new_authority
            UNION ALL
            SELECT id FROM existing_new_authority
            LIMIT 1
        ),
        upsert_account AS (
            INSERT INTO accounts (address, user_id)
            VALUES ($3, (SELECT id FROM combined_new_authority))
        ), existing_old_authority AS (
            SELECT id FROM users WHERE address = $1
        ), combined_old_authority AS (
            SELECT id FROM upsert_old_authority
            UNION ALL
            SELECT id FROM existing_old_authority
            LIMIT 1
        ),
        INSERT INTO transfer_account_authority_events (timestamp, slot, tx_sig, in_flashloan, call_stack, account_id, old_authority_id, new_authority_id)
        VALUES ($4, $5, $6, $7, $8, (SELECT id FROM combined_account), (SELECT id FROM combined_old_authority), (SELECT id FROM combined_new_authority))
        RETURNING id;
        "#;

        let id = diesel::sql_query(sql)
            .bind::<Text, _>(old_authority)
            .bind::<Text, _>(new_authority)
            .bind::<Text, _>(account)
            .bind::<Timestamp, _>(timestamp)
            .bind::<Numeric, _>(slot)
            .bind::<Text, _>(tx_sig)
            .bind::<Bool, _>(in_flashloan)
            .bind::<Text, _>(call_stack)
            .load::<IdResult>(connection)?
            .pop()
            .expect("Expected at least one id")
            .id;

        Ok(id)
    }
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = deposit_events)]
pub struct DepositEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub amount: Decimal,
}

impl DepositEvents {
    pub fn insert(
        db_connection: &mut PgConnection,
        account_id: i32,
        authority_id: i32,
        bank_id: i32,
        amount: Decimal,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: String,
        tx_sig: String,
    ) -> QueryResult<Option<i32>> {
        let deposit_event = DepositEvents {
            timestamp,
            slot,
            tx_sig,
            call_stack,
            in_flashloan,
            account_id,
            authority_id,
            bank_id,
            amount,
            ..Default::default()
        };

        diesel::insert_into(deposit_events::table)
            .values(&deposit_event)
            .on_conflict_do_nothing()
            .returning(deposit_events::id)
            .get_result(db_connection)
            .optional()
    }

    pub fn insert_with_dependents(
        connection: &mut PgConnection,
        authority: &str,
        account: &str,
        bank_mint_address: &str,
        bank_mint_symbol: &str,
        bank_mint_decimals: i16,
        bank: &str,
        amount: Decimal,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: &str,
        tx_sig: &str,
    ) -> QueryResult<i32> {
        let sql = r#"
        WITH upsert_authority AS (
            INSERT INTO users (address)
            VALUES ($1)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_authority AS (
            SELECT id FROM users WHERE address = $1
        ), combined_authority AS (
            SELECT id FROM upsert_authority
            UNION ALL
            SELECT id FROM existing_authority
            LIMIT 1
        ),
        upsert_account AS (
            INSERT INTO accounts (address, user_id)
            VALUES ($2, (SELECT id FROM combined_authority))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_account AS (
            SELECT id FROM accounts WHERE address = $2
        ), combined_account AS (
            SELECT id FROM upsert_account
            UNION ALL
            SELECT id FROM existing_account
            LIMIT 1
        ),
        upsert_bank_mint AS (
            INSERT INTO mints (address, symbol, decimals)
            VALUES ($3, $4, $5)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank_mint AS (
            SELECT id FROM
        ), combined_bank_mint AS (
            SELECT id FROM upsert_bank_mint
            UNION ALL
            SELECT id FROM existing_bank_mint
            LIMIT 1
        )
        upsert_bank AS (
            INSERT INTO banks (address, mint_id)
            VALUES ($6, (SELECT id FROM combined_bank_mint))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank AS (
            SELECT id FROM
        ), combined_bank AS (
            SELECT id FROM upsert_bank
            UNION ALL
            SELECT id FROM existing_bank
            LIMIT 1
        )
        INSERT INTO deposit_events (timestamp, slot, tx_sig, in_flashloan, call_stack, account_id, authority_id, bank_id, amount)
        VALUES ($7, $8, $9, $10, $11, (SELECT id FROM combined_account), (SELECT id FROM combined_authority), (SELECT id FROM combined_bank), $12)
        RETURNING id;
        "#;

        let id = diesel::sql_query(sql)
            .bind::<Text, _>(authority)
            .bind::<Text, _>(account)
            .bind::<Text, _>(bank_mint_address)
            .bind::<Text, _>(bank_mint_symbol)
            .bind::<SmallInt, _>(bank_mint_decimals)
            .bind::<Text, _>(bank)
            .bind::<Timestamp, _>(timestamp)
            .bind::<Numeric, _>(slot)
            .bind::<Text, _>(tx_sig)
            .bind::<Bool, _>(in_flashloan)
            .bind::<Text, _>(call_stack)
            .bind::<Numeric, _>(amount)
            .load::<IdResult>(connection)?
            .pop()
            .expect("Expected at least one id")
            .id;

        Ok(id)
    }
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = borrow_events)]
pub struct BorrowEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub amount: Decimal,
}

impl BorrowEvents {
    pub fn insert(
        db_connection: &mut PgConnection,
        account_id: i32,
        authority_id: i32,
        bank_id: i32,
        amount: Decimal,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: String,
        tx_sig: String,
    ) -> QueryResult<Option<i32>> {
        let borrow_event = BorrowEvents {
            timestamp,
            slot,
            tx_sig,
            call_stack,
            in_flashloan,
            account_id,
            authority_id,
            bank_id,
            amount,
            ..Default::default()
        };

        diesel::insert_into(borrow_events::table)
            .values(&borrow_event)
            .on_conflict_do_nothing()
            .returning(borrow_events::id)
            .get_result(db_connection)
            .optional()
    }

    pub fn insert_with_dependents(
        connection: &mut PgConnection,
        authority: &str,
        account: &str,
        bank_mint_address: &str,
        bank_mint_symbol: &str,
        bank_mint_decimals: i16,
        bank: &str,
        amount: Decimal,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: &str,
        tx_sig: &str,
    ) -> QueryResult<i32> {
        let sql = r#"
        WITH upsert_authority AS (
            INSERT INTO users (address)
            VALUES ($1)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_authority AS (
            SELECT id FROM users WHERE address = $1
        ), combined_authority AS (
            SELECT id FROM upsert_authority
            UNION ALL
            SELECT id FROM existing_authority
            LIMIT 1
        ),
        upsert_account AS (
            INSERT INTO accounts (address, user_id)
            VALUES ($2, (SELECT id FROM combined_authority))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_account AS (
            SELECT id FROM accounts WHERE address = $2
        ), combined_account AS (
            SELECT id FROM upsert_account
            UNION ALL
            SELECT id FROM existing_account
            LIMIT 1
        ),
        upsert_bank_mint AS (
            INSERT INTO mints (address
        ), existing_bank_mint AS (
            SELECT id FROM
        ), combined_bank_mint AS (
            SELECT id FROM upsert_bank_mint
            UNION ALL
            SELECT id FROM existing_bank_mint
            LIMIT 1
        )
        upsert_bank AS (
            INSERT INTO banks (address, mint_id)
            VALUES ($6, (SELECT id FROM combined_bank_mint))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank AS (
            SELECT id FROM
        ), combined_bank AS (
            SELECT id FROM upsert_bank
            UNION ALL
            SELECT id FROM existing_bank
            LIMIT 1
        )
        INSERT INTO borrow_events (timestamp, slot, tx_sig, in_flashloan, call_stack, account_id, authority_id, bank_id, amount)
        VALUES ($7, $8, $9, $10, $11, (SELECT id FROM combined_account), (SELECT id FROM combined_authority), (SELECT id FROM combined_bank), $12)
        RETURNING id;
        "#;

        let id = diesel::sql_query(sql)
            .bind::<Text, _>(authority)
            .bind::<Text, _>(account)
            .bind::<Text, _>(bank_mint_address)
            .bind::<Text, _>(bank_mint_symbol)
            .bind::<SmallInt, _>(bank_mint_decimals)
            .bind::<Text, _>(bank)
            .bind::<Timestamp, _>(timestamp)
            .bind::<Numeric, _>(slot)
            .bind::<Text, _>(tx_sig)
            .bind::<Bool, _>(in_flashloan)
            .bind::<Text, _>(call_stack)
            .bind::<Numeric, _>(amount)
            .load::<IdResult>(connection)?
            .pop()
            .expect("Expected at least one id")
            .id;

        Ok(id)
    }
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = repay_events)]
pub struct RepayEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub amount: Decimal,
    pub all: bool,
}

impl RepayEvents {
    pub fn insert(
        db_connection: &mut PgConnection,
        account_id: i32,
        authority_id: i32,
        bank_id: i32,
        amount: Decimal,
        all: bool,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: String,
        tx_sig: String,
    ) -> QueryResult<Option<i32>> {
        let repay_event = RepayEvents {
            timestamp,
            slot,
            tx_sig,
            call_stack,
            in_flashloan,
            account_id,
            authority_id,
            bank_id,
            amount,
            all,
            ..Default::default()
        };

        diesel::insert_into(repay_events::table)
            .values(&repay_event)
            .on_conflict_do_nothing()
            .returning(repay_events::id)
            .get_result(db_connection)
            .optional()
    }

    pub fn insert_with_dependents(
        connection: &mut PgConnection,
        authority: &str,
        account: &str,
        bank_mint_address: &str,
        bank_mint_symbol: &str,
        bank_mint_decimals: i16,
        bank: &str,
        amount: Decimal,
        all: bool,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: &str,
        tx_sig: &str,
    ) -> QueryResult<i32> {
        let sql = r#"
        WITH upsert_authority AS (
            INSERT INTO users (address)
            VALUES ($1)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_authority AS (
            SELECT id FROM users WHERE address = $1
        ), combined_authority AS (
            SELECT id FROM upsert_authority
            UNION ALL
            SELECT id FROM existing_authority
            LIMIT 1
        ),
        upsert_account AS (
            INSERT INTO accounts (address, user_id)
            VALUES ($2, (SELECT id FROM combined_authority))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_account AS (
            SELECT id FROM accounts WHERE address = $2
        ), combined_account AS (
            SELECT id FROM upsert_account
            UNION ALL
            SELECT id FROM existing_account
            LIMIT 1
        ),
        upsert_bank_mint AS (
            INSERT INTO mints (address, symbol, decimals)
            VALUES ($3, $4, $5)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank_mint AS (
            SELECT id FROM mints WHERE address = $3
        ), combined_bank_mint AS (
            SELECT id FROM upsert_bank_mint
            UNION ALL
            SELECT id FROM existing_bank_mint
            LIMIT 1
        ),
        upsert_bank AS (
            INSERT INTO banks (address, mint_id)
            VALUES ($6, (SELECT id FROM combined_bank_mint))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank AS (
            SELECT id FROM banks WHERE address = $6
        ), combined_bank AS (
            SELECT id FROM upsert_bank
            UNION ALL
            SELECT id FROM existing_bank
            LIMIT 1
        )
        INSERT INTO repay_events (timestamp, slot, tx_sig, in_flashloan, call_stack, account_id, authority_id, bank_id, amount, all)
        VALUES ($7, $8, $9, $10, $11, (SELECT id FROM combined_account), (SELECT id FROM combined_authority), (SELECT id FROM combined_bank), $12, $13)
        RETURNING id;
        "#;

        let id = diesel::sql_query(sql)
            .bind::<Text, _>(authority)
            .bind::<Text, _>(account)
            .bind::<Text, _>(bank_mint_address)
            .bind::<Text, _>(bank_mint_symbol)
            .bind::<SmallInt, _>(bank_mint_decimals)
            .bind::<Text, _>(bank)
            .bind::<Timestamp, _>(timestamp)
            .bind::<Numeric, _>(slot)
            .bind::<Text, _>(tx_sig)
            .bind::<Bool, _>(in_flashloan)
            .bind::<Text, _>(call_stack)
            .bind::<Numeric, _>(amount)
            .bind::<Bool, _>(all)
            .load::<IdResult>(connection)?
            .pop()
            .expect("Expected at least one id")
            .id;

        Ok(id)
    }
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = withdraw_events)]
pub struct WithdrawEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub amount: Decimal,
    pub all: bool,
}

impl WithdrawEvents {
    pub fn insert(
        db_connection: &mut PgConnection,
        account_id: i32,
        authority_id: i32,
        bank_id: i32,
        amount: Decimal,
        all: bool,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: String,
        tx_sig: String,
    ) -> QueryResult<Option<i32>> {
        let withdraw_event = WithdrawEvents {
            timestamp,
            slot,
            tx_sig,
            call_stack,
            in_flashloan,
            account_id,
            authority_id,
            bank_id,
            amount,
            all,
            ..Default::default()
        };

        diesel::insert_into(withdraw_events::table)
            .values(&withdraw_event)
            .on_conflict_do_nothing()
            .returning(withdraw_events::id)
            .get_result(db_connection)
            .optional()
    }

    pub fn insert_with_dependents(
        connection: &mut PgConnection,
        authority: &str,
        account: &str,
        bank_mint_address: &str,
        bank_mint_symbol: &str,
        bank_mint_decimals: i16,
        bank: &str,
        amount: Decimal,
        all: bool,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: &str,
        tx_sig: &str,
    ) -> QueryResult<i32> {
        let sql = r#"
        WITH upsert_authority AS (
            INSERT INTO users (address)
            VALUES ($1)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_authority AS (
            SELECT id FROM users WHERE address = $1
        ), combined_authority AS (
            SELECT id FROM upsert_authority
            UNION ALL
            SELECT id FROM existing_authority
            LIMIT 1
        ),
        upsert_account AS (
            INSERT INTO accounts (address, user_id)
            VALUES ($2, (SELECT id FROM combined_authority))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_account AS (
            SELECT id FROM accounts WHERE address = $2
        ), combined_account AS (
            SELECT id FROM upsert_account
            UNION ALL
            SELECT id FROM existing_account
            LIMIT 1
        ),
        upsert_bank_mint AS (
            INSERT INTO mints (address, symbol, decimals)
            VALUES ($3, $4, $5)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank_mint AS (
            SELECT id FROM mints WHERE address = $3
        ), combined_bank_mint AS (
            SELECT id FROM upsert_bank_mint
            UNION ALL
            SELECT id FROM existing_bank_mint
            LIMIT 1
        ),
        upsert_bank AS (
            INSERT INTO banks (address, mint_id)
            VALUES ($6, (SELECT id FROM combined_bank_mint))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank AS (
            SELECT id FROM banks WHERE address = $6
        ), combined_bank AS (
            SELECT id FROM upsert_bank
            UNION ALL
            SELECT id FROM existing_bank
            LIMIT 1
        )
        INSERT INTO withdraw_events (timestamp, slot, tx_sig, in_flashloan, call_stack, account_id, authority_id, bank_id, amount, all)
        VALUES ($7, $8, $9, $10, $11, (SELECT id FROM combined_account), (SELECT id FROM combined_authority), (SELECT id FROM combined_bank), $12, $13)
        RETURNING id;
        "#;

        let id = diesel::sql_query(sql)
            .bind::<Text, _>(authority)
            .bind::<Text, _>(account)
            .bind::<Text, _>(bank_mint_address)
            .bind::<Text, _>(bank_mint_symbol)
            .bind::<SmallInt, _>(bank_mint_decimals)
            .bind::<Text, _>(bank)
            .bind::<Timestamp, _>(timestamp)
            .bind::<Numeric, _>(slot)
            .bind::<Text, _>(tx_sig)
            .bind::<Bool, _>(in_flashloan)
            .bind::<Text, _>(call_stack)
            .bind::<Numeric, _>(amount)
            .bind::<Bool, _>(all)
            .load::<IdResult>(connection)?
            .pop()
            .expect("Expected at least one id")
            .id;

        Ok(id)
    }
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = withdraw_emissions_events)]
pub struct WithdrawEmissionsEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub account_id: i32,
    pub authority_id: i32,
    pub bank_id: i32,
    pub emission_mint_id: i32,
    pub amount: Decimal,
}

impl WithdrawEmissionsEvents {
    pub fn insert(
        db_connection: &mut PgConnection,
        account_id: i32,
        authority_id: i32,
        bank_id: i32,
        emission_mint_id: i32,
        amount: Decimal,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: String,
        tx_sig: String,
    ) -> QueryResult<Option<i32>> {
        let withdraw_emissions_event = WithdrawEmissionsEvents {
            timestamp,
            slot,
            tx_sig,
            call_stack,
            in_flashloan,
            account_id,
            authority_id,
            bank_id,
            emission_mint_id,
            amount,
            ..Default::default()
        };

        diesel::insert_into(withdraw_emissions_events::table)
            .values(&withdraw_emissions_event)
            .on_conflict_do_nothing()
            .returning(withdraw_emissions_events::id)
            .get_result(db_connection)
            .optional()
    }

    pub fn insert_with_dependents(
        connection: &mut PgConnection,
        authority: &str,
        account: &str,
        bank_mint_address: &str,
        bank_mint_symbol: &str,
        bank_mint_decimals: i16,
        bank: &str,
        emission_mint_address: &str,
        emission_mint_symbol: &str,
        emission_mint_decimals: i16,
        amount: Decimal,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: &str,
        tx_sig: &str,
    ) -> QueryResult<i32> {
        let sql = r#"
        WITH upsert_authority AS (
            INSERT INTO users (address)
            VALUES ($1)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_authority AS (
            SELECT id FROM users WHERE address = $1
        ), combined_authority AS (
            SELECT id FROM upsert_authority
            UNION ALL
            SELECT id FROM existing_authority
            LIMIT 1
        ),
        upsert_account AS (
            INSERT INTO accounts (address, user_id)
            VALUES ($2, (SELECT id FROM combined_authority))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_account AS (
            SELECT id
        ), combined_account AS (
            SELECT id
        ),
        upsert_bank_mint AS (
            INSERT INTO mints (address, symbol, decimals)
            VALUES ($3, $4, $5)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank_mint AS (
            SELECT id
        ), combined_bank_mint AS (
            SELECT id
        ),
        upsert_bank AS (
            INSERT INTO banks (address, mint_id)
            VALUES ($6, (SELECT id FROM combined_bank_mint))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank AS (
            SELECT id
        ), combined_bank AS (
            SELECT id
        ),
        upsert_emission_mint AS (
            INSERT INTO mints (address, symbol, decimals)
            VALUES ($7, $8, $9)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_emission_mint AS (
            SELECT id
        ), combined_emission_mint AS (
            SELECT id
        )
        INSERT INTO withdraw_emissions_events (timestamp, slot, tx_sig, in_flashloan, call_stack, account_id, authority_id, bank_id, emission_mint_id, amount)
        VALUES ($10, $11, $12, $13, $14, (SELECT id FROM combined_account), (SELECT id FROM combined_authority), (SELECT id FROM combined_bank), (SELECT id FROM combined_emission_mint), $15)
        RETURNING id;
        "#;

        let id = diesel::sql_query(sql)
            .bind::<Text, _>(authority)
            .bind::<Text, _>(account)
            .bind::<Text, _>(bank_mint_address)
            .bind::<Text, _>(bank_mint_symbol)
            .bind::<SmallInt, _>(bank_mint_decimals)
            .bind::<Text, _>(bank)
            .bind::<Text, _>(emission_mint_address)
            .bind::<Text, _>(emission_mint_symbol)
            .bind::<SmallInt, _>(emission_mint_decimals)
            .bind::<Timestamp, _>(timestamp)
            .bind::<Numeric, _>(slot)
            .bind::<Text, _>(tx_sig)
            .bind::<Bool, _>(in_flashloan)
            .bind::<Text, _>(call_stack)
            .bind::<Numeric, _>(amount)
            .load::<IdResult>(connection)?
            .pop()
            .expect("Expected at least one id")
            .id;

        Ok(id)
    }
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = liquidate_events)]
pub struct LiquidateEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub liquidator_account_id: i32,
    pub liquidatee_account_id: i32,
    pub liquidator_user_id: i32,
    pub asset_bank_id: i32,
    pub liability_bank_id: i32,
    pub asset_amount: Decimal,
}

impl LiquidateEvents {
    pub fn insert(
        db_connection: &mut PgConnection,
        liquidator_account_id: i32,
        liquidatee_account_id: i32,
        liquidator_user_id: i32,
        asset_bank_id: i32,
        liability_bank_id: i32,
        asset_amount: Decimal,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: String,
        tx_sig: String,
    ) -> QueryResult<Option<i32>> {
        let liquidate_event = LiquidateEvents {
            timestamp,
            slot,
            tx_sig,
            call_stack,
            in_flashloan,
            liquidator_account_id,
            liquidatee_account_id,
            liquidator_user_id,
            asset_bank_id,
            liability_bank_id,
            asset_amount,
            ..Default::default()
        };

        diesel::insert_into(liquidate_events::table)
            .values(&liquidate_event)
            .on_conflict_do_nothing()
            .returning(liquidate_events::id)
            .get_result(db_connection)
            .optional()
    }

    pub fn insert_with_dependents(
        connection: &mut PgConnection,
        liquidator_user: &str,
        liquidatee_user: &str,
        liquidator_account: &str,
        liquidatee_account: &str,
        asset_mint_address: &str,
        asset_mint_symbol: &str,
        asset_mint_decimals: i16,
        liability_mint_address: &str,
        liability_mint_symbol: &str,
        liability_mint_decimals: i16,
        asset_bank: &str,
        liability_bank: &str,
        asset_amount: Decimal,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: &str,
        tx_sig: &str,
    ) -> QueryResult<i32> {
        let sql = r#"
        WITH upsert_user_liquidator AS (
            INSERT INTO users (address)
            VALUES ($1)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_user_liquidator AS (
            SELECT id FROM users WHERE address = $1
        ), combined_user_liquidator AS (
            SELECT id FROM upsert_user_liquidator
            UNION ALL
            SELECT id FROM existing_user_liquidator
            LIMIT 1
        ),
        upsert_user_liquidatee AS (
            INSERT INTO users (address)
            VALUES ($2)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_user_liquidatee AS (
            SELECT id FROM users WHERE address = $2
        ), combined_user_liquidatee AS (
            SELECT id FROM upsert_user_liquidatee
            UNION ALL
            SELECT id FROM existing_user_liquidatee
            LIMIT 1
        ),
        upsert_account_liquidator AS (
            INSERT INTO accounts (address, user_id)
            VALUES ($3, (SELECT id FROM combined_user_liquidator))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_account_liquidator AS (
            SELECT id FROM accounts WHERE address = $3
        ), combined_account_liquidator AS (
            SELECT id FROM upsert_account_liquidator
            UNION ALL
            SELECT id FROM existing_account_liquidator
            LIMIT 1
        ),
        upsert_account_liquidatee AS (
            INSERT INTO accounts (address, user_id)
            VALUES ($4, (SELECT id FROM combined_user_liquidatee))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_account_liquidatee AS (
            SELECT id FROM accounts WHERE address = $4
        ), combined_account_liquidatee AS (
            SELECT id FROM upsert_account_liquidatee
            UNION ALL
            SELECT id FROM existing_account_liquidatee
            LIMIT 1
        ),
        upsert_mint_asset AS (
            INSERT INTO mints (address, symbol, decimals)
            VALUES ($5, $6, $7)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_mint_asset AS (
            SELECT id FROM mints WHERE address = $5
        ), combined_mint_asset AS (
            SELECT id FROM upsert_mint_asset
            UNION ALL
            SELECT id FROM existing_mint_asset
            LIMIT 1
        ),
        upsert_mint_liability AS (
            INSERT INTO mints (address, symbol, decimals)
            VALUES ($8, $9, $10)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_mint_liability AS (
            SELECT id FROM mints WHERE address = $8
        ), combined_mint_liability AS (
            SELECT id FROM upsert_mint_liability
            UNION ALL
            SELECT id FROM existing_mint_liability
            LIMIT 1
        ),
        upsert_bank_asset AS (
            INSERT INTO banks (address, mint_id)
            VALUES ($11, (SELECT id FROM combined_mint_asset))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank_asset AS (
            SELECT id FROM banks WHERE address = $11
        ), combined_bank_asset AS (
            SELECT id FROM upsert_bank_asset
            UNION ALL
            SELECT id FROM existing_bank_asset
            LIMIT 1
        ),
        upsert_bank_liability AS (
            INSERT INTO banks (address, mint_id)
            VALUES ($12, (SELECT id FROM combined_mint_liability))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank_liability AS (
            SELECT id FROM banks WHERE address = $12
        ), combined_bank_liability AS (
            SELECT id FROM upsert_bank_liability
            UNION ALL
            SELECT id FROM existing_bank_liability
            LIMIT 1
        )
        INSERT INTO liquidate_events (timestamp, slot, tx_sig, in_flashloan, call_stack, liquidator_account_id, liquidatee_account_id, liquidator_user_id, asset_bank_id, liability_bank_id, asset_amount)
        VALUES ($13, $14, $15, $16, $17, (SELECT id FROM combined_account_liquidator), (SELECT id FROM combined_account_liquidatee), (SELECT id FROM combined_user_liquidator), (SELECT id FROM combined_bank_asset), (SELECT id FROM combined_bank_liability), $18)
        RETURNING id;
        "#;

        let id = diesel::sql_query(sql)
            .bind::<Text, _>(liquidator_user)
            .bind::<Text, _>(liquidatee_user)
            .bind::<Text, _>(liquidator_account)
            .bind::<Text, _>(liquidatee_account)
            .bind::<Text, _>(asset_mint_address)
            .bind::<Text, _>(asset_mint_symbol)
            .bind::<SmallInt, _>(asset_mint_decimals)
            .bind::<Text, _>(liability_mint_address)
            .bind::<Text, _>(liability_mint_symbol)
            .bind::<SmallInt, _>(liability_mint_decimals)
            .bind::<Text, _>(asset_bank)
            .bind::<Text, _>(liability_bank)
            .bind::<Timestamp, _>(timestamp)
            .bind::<Numeric, _>(slot)
            .bind::<Text, _>(tx_sig)
            .bind::<Bool, _>(in_flashloan)
            .bind::<Text, _>(call_stack)
            .bind::<Numeric, _>(asset_amount)
            .load::<IdResult>(connection)?
            .pop()
            .expect("Expected at least one id")
            .id;

        Ok(id)
    }
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = create_bank_events)]
pub struct CreateBankEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub bank_id: i32,
    pub asset_weight_init: Decimal,
    pub asset_weight_maint: Decimal,
    pub liability_weight_init: Decimal,
    pub liability_weight_maint: Decimal,
    pub deposit_limit: Decimal,
    pub optimal_utilization_rate: Decimal,
    pub plateau_interest_rate: Decimal,
    pub max_interest_rate: Decimal,
    pub insurance_fee_fixed_apr: Decimal,
    pub insurance_ir_fee: Decimal,
    pub protocol_fixed_fee_apr: Decimal,
    pub protocol_ir_fee: Decimal,
    pub operational_state_id: i32,
    pub oracle_setup_id: i32,
    pub oracle_keys: String,
    pub borrow_limit: Decimal,
    pub risk_tier_id: i32,
    pub total_asset_value_init_limit: Decimal,
    pub oracle_max_age: i32,
}

impl CreateBankEvents {
    pub fn insert(
        db_connection: &mut PgConnection,
        bank_id: i32,
        asset_weight_init: Decimal,
        asset_weight_maint: Decimal,
        liability_weight_init: Decimal,
        liability_weight_maint: Decimal,
        deposit_limit: Decimal,
        optimal_utilization_rate: Decimal,
        plateau_interest_rate: Decimal,
        max_interest_rate: Decimal,
        insurance_fee_fixed_apr: Decimal,
        insurance_ir_fee: Decimal,
        protocol_fixed_fee_apr: Decimal,
        protocol_ir_fee: Decimal,
        operational_state_id: i32,
        oracle_setup_id: i32,
        oracle_keys: String,
        borrow_limit: Decimal,
        risk_tier_id: i32,
        total_asset_value_init_limit: Decimal,
        oracle_max_age: i32,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: String,
        tx_sig: String,
    ) -> QueryResult<Option<i32>> {
        let create_bank_event = CreateBankEvents {
            timestamp,
            slot,
            tx_sig,
            call_stack,
            in_flashloan,
            bank_id,
            asset_weight_init,
            asset_weight_maint,
            liability_weight_init,
            liability_weight_maint,
            deposit_limit,
            optimal_utilization_rate,
            plateau_interest_rate,
            max_interest_rate,
            insurance_fee_fixed_apr,
            insurance_ir_fee,
            protocol_fixed_fee_apr,
            protocol_ir_fee,
            operational_state_id,
            oracle_setup_id,
            oracle_keys,
            borrow_limit,
            risk_tier_id,
            total_asset_value_init_limit,
            oracle_max_age,
            ..Default::default()
        };

        diesel::insert_into(create_bank_events::table)
            .values(&create_bank_event)
            .on_conflict_do_nothing()
            .returning(create_bank_events::id)
            .get_result(db_connection)
            .optional()
    }

    pub fn insert_with_dependents(
        connection: &mut PgConnection,
        bank_address: &str,
        bank_mint_address: &str,
        bank_mint_symbol: &str,
        bank_mint_decimals: i16,
        asset_weight_init: Decimal,
        asset_weight_maint: Decimal,
        liability_weight_init: Decimal,
        liability_weight_maint: Decimal,
        deposit_limit: Decimal,
        optimal_utilization_rate: Decimal,
        plateau_interest_rate: Decimal,
        max_interest_rate: Decimal,
        insurance_fee_fixed_apr: Decimal,
        insurance_ir_fee: Decimal,
        protocol_fixed_fee_apr: Decimal,
        protocol_ir_fee: Decimal,
        operational_state_id: i32,
        oracle_setup_id: i32,
        oracle_keys: &str,
        borrow_limit: Decimal,
        risk_tier_id: i32,
        total_asset_value_init_limit: Decimal,
        oracle_max_age: i32,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: &str,
        tx_sig: &str,
    ) -> QueryResult<i32> {
        let sql = r#"
        WITH upsert_bank_mint AS (
            INSERT INTO mints (address, symbol, decimals)
            VALUES ($2, $3, $4)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank_mint AS (
            SELECT id FROM mints WHERE address = $2
        ), combined_bank_mint AS (
            SELECT id FROM upsert_bank_mint
            UNION ALL
            SELECT id FROM existing_bank_mint
            LIMIT 1
        ),
        upsert_bank AS (
            INSERT INTO banks (address, mint_id)
            VALUES ($1, (SELECT id FROM combined_bank_mint))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank AS (
            SELECT id FROM banks WHERE address = $1
        ), combined_bank AS (
            SELECT id FROM upsert_bank
            UNION ALL
            SELECT id FROM existing_bank
            LIMIT 1
        )

        INSERT INTO create_bank_events (timestamp, slot, tx_sig, in_flashloan, call_stack, bank_id, asset_weight_init, asset_weight_maint, liability_weight_init, liability_weight_maint, deposit_limit, optimal_utilization_rate, plateau_interest_rate, max_interest_rate, insurance_fee_fixed_apr, insurance_ir_fee, protocol_fixed_fee_apr, protocol_ir_fee, operational_state_id, oracle_setup_id, oracle_keys, borrow_limit, risk_tier_id, total_asset_value_init_limit, oracle_max_age)
        VALUES ($15, $16, $17, $18, $19, (SELECT id FROM combined_bank), $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $20, $21, $22, $23, $24, $25, $26, $27)
        RETURNING id;
        "#;

        let id = diesel::sql_query(sql)
            .bind::<Text, _>(bank_address)
            .bind::<Text, _>(bank_mint_address)
            .bind::<Text, _>(bank_mint_symbol)
            .bind::<SmallInt, _>(bank_mint_decimals)
            .bind::<Numeric, _>(asset_weight_init)
            .bind::<Numeric, _>(asset_weight_maint)
            .bind::<Numeric, _>(liability_weight_init)
            .bind::<Numeric, _>(liability_weight_maint)
            .bind::<Numeric, _>(deposit_limit)
            .bind::<Numeric, _>(optimal_utilization_rate)
            .bind::<Numeric, _>(plateau_interest_rate)
            .bind::<Numeric, _>(max_interest_rate)
            .bind::<Numeric, _>(insurance_fee_fixed_apr)
            .bind::<Numeric, _>(insurance_ir_fee)
            .bind::<Numeric, _>(protocol_fixed_fee_apr)
            .bind::<Numeric, _>(protocol_ir_fee)
            .bind::<Integer, _>(operational_state_id)
            .bind::<Integer, _>(oracle_setup_id)
            .bind::<Text, _>(oracle_keys)
            .bind::<Numeric, _>(borrow_limit)
            .bind::<Integer, _>(risk_tier_id)
            .bind::<Numeric, _>(total_asset_value_init_limit)
            .bind::<Integer, _>(oracle_max_age)
            .bind::<Timestamp, _>(timestamp)
            .bind::<Numeric, _>(slot)
            .bind::<Text, _>(tx_sig)
            .bind::<Bool, _>(in_flashloan)
            .bind::<Text, _>(call_stack)
            .load::<IdResult>(connection)?
            .pop()
            .expect("Expected at least one id")
            .id;

        Ok(id)
    }
}

#[derive(Default, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = configure_bank_events)]
pub struct ConfigureBankEvents {
    #[diesel(skip_insertion)]
    pub id: i32,
    pub timestamp: chrono::NaiveDateTime,
    pub slot: Decimal,
    pub tx_sig: String,
    pub in_flashloan: bool,
    pub call_stack: String,

    pub bank_id: i32,
    pub asset_weight_init: Option<Decimal>,
    pub asset_weight_maint: Option<Decimal>,
    pub liability_weight_init: Option<Decimal>,
    pub liability_weight_maint: Option<Decimal>,
    pub deposit_limit: Option<Decimal>,
    pub optimal_utilization_rate: Option<Decimal>,
    pub plateau_interest_rate: Option<Decimal>,
    pub max_interest_rate: Option<Decimal>,
    pub insurance_fee_fixed_apr: Option<Decimal>,
    pub insurance_ir_fee: Option<Decimal>,
    pub protocol_fixed_fee_apr: Option<Decimal>,
    pub protocol_ir_fee: Option<Decimal>,
    pub operational_state_id: Option<i32>,
    pub oracle_setup_id: Option<i32>,
    pub oracle_keys: Option<String>,
    pub borrow_limit: Option<Decimal>,
    pub risk_tier_id: Option<i32>,
    pub total_asset_value_init_limit: Option<Decimal>,
    pub oracle_max_age: Option<i32>,
}

impl ConfigureBankEvents {
    pub fn insert(
        db_connection: &mut PgConnection,
        bank_id: i32,
        asset_weight_init: Option<Decimal>,
        asset_weight_maint: Option<Decimal>,
        liability_weight_init: Option<Decimal>,
        liability_weight_maint: Option<Decimal>,
        deposit_limit: Option<Decimal>,
        optimal_utilization_rate: Option<Decimal>,
        plateau_interest_rate: Option<Decimal>,
        max_interest_rate: Option<Decimal>,
        insurance_fee_fixed_apr: Option<Decimal>,
        insurance_ir_fee: Option<Decimal>,
        protocol_fixed_fee_apr: Option<Decimal>,
        protocol_ir_fee: Option<Decimal>,
        operational_state_id: Option<i32>,
        oracle_setup_id: Option<i32>,
        oracle_keys: Option<String>,
        borrow_limit: Option<Decimal>,
        risk_tier_id: Option<i32>,
        total_asset_value_init_limit: Option<Decimal>,
        oracle_max_age: Option<i32>,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: String,
        tx_sig: String,
    ) -> QueryResult<Option<i32>> {
        let configure_bank_event = ConfigureBankEvents {
            timestamp,
            slot,
            tx_sig,
            call_stack,
            in_flashloan,
            bank_id,
            asset_weight_init,
            asset_weight_maint,
            liability_weight_init,
            liability_weight_maint,
            deposit_limit,
            optimal_utilization_rate,
            plateau_interest_rate,
            max_interest_rate,
            insurance_fee_fixed_apr,
            insurance_ir_fee,
            protocol_fixed_fee_apr,
            protocol_ir_fee,
            operational_state_id,
            oracle_setup_id,
            oracle_keys,
            borrow_limit,
            risk_tier_id,
            total_asset_value_init_limit,
            oracle_max_age,
            ..Default::default()
        };

        diesel::insert_into(configure_bank_events::table)
            .values(&configure_bank_event)
            .on_conflict_do_nothing()
            .returning(configure_bank_events::id)
            .get_result(db_connection)
            .optional()
    }

    pub fn insert_with_dependents(
        connection: &mut PgConnection,
        bank_address: &str,
        bank_mint_address: &str,
        bank_mint_symbol: &str,
        bank_mint_decimals: i16,
        asset_weight_init: Option<Decimal>,
        asset_weight_maint: Option<Decimal>,
        liability_weight_init: Option<Decimal>,
        liability_weight_maint: Option<Decimal>,
        deposit_limit: Option<Decimal>,
        optimal_utilization_rate: Option<Decimal>,
        plateau_interest_rate: Option<Decimal>,
        max_interest_rate: Option<Decimal>,
        insurance_fee_fixed_apr: Option<Decimal>,
        insurance_ir_fee: Option<Decimal>,
        protocol_fixed_fee_apr: Option<Decimal>,
        protocol_ir_fee: Option<Decimal>,
        operational_state_id: Option<i32>,
        oracle_setup_id: Option<i32>,
        oracle_keys: Option<String>,
        borrow_limit: Option<Decimal>,
        risk_tier_id: Option<i32>,
        total_asset_value_init_limit: Option<Decimal>,
        oracle_max_age: Option<i32>,
        timestamp: chrono::NaiveDateTime,
        slot: Decimal,
        in_flashloan: bool,
        call_stack: &str,
        tx_sig: &str,
    ) -> QueryResult<i32> {
        let sql = r#"
        WITH upsert_bank_mint AS (
            INSERT INTO mints (address, symbol, decimals)
            VALUES ($2, $3, $4)
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank_mint AS (
            SELECT id FROM mints WHERE address = $2
        ), combined_bank_mint AS (
            SELECT id FROM upsert_bank_mint
            UNION ALL
            SELECT id FROM existing_bank_mint
            LIMIT 1
        ),
        upsert_bank AS (
            INSERT INTO banks (address, mint_id)
            VALUES ($1, (SELECT id FROM combined_bank_mint))
            ON CONFLICT (address) DO NOTHING
            RETURNING id
        ), existing_bank AS (
            SELECT id FROM banks WHERE address = $1
        ), combined_bank AS (
            SELECT id FROM upsert_bank
            UNION ALL
            SELECT id FROM existing_bank
            LIMIT 1
        )

        INSERT INTO configure_bank_events (timestamp, slot, tx_sig, in_flashloan, call_stack, bank_id, asset_weight_init, asset_weight_maint, liability_weight_init, liability_weight_maint, deposit_limit, optimal_utilization_rate, plateau_interest_rate, max_interest_rate, insurance_fee_fixed_apr, insurance_ir_fee, protocol_fixed_fee_apr, protocol_ir_fee, operational_state_id, oracle_setup_id, oracle_keys, borrow_limit, risk_tier_id, total_asset_value_init_limit, oracle_max_age)
        VALUES ($15, $16, $17, $18, $19, (SELECT id FROM combined_bank), $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $20, $21, $22, $23, $24, $25, $26, $27)
        RETURNING id;
        "#;

        let id = diesel::sql_query(sql)
            .bind::<Text, _>(bank_address)
            .bind::<Text, _>(bank_mint_address)
            .bind::<Text, _>(bank_mint_symbol)
            .bind::<SmallInt, _>(bank_mint_decimals)
            .bind::<Nullable<Numeric>, _>(asset_weight_init)
            .bind::<Nullable<Numeric>, _>(asset_weight_maint)
            .bind::<Nullable<Numeric>, _>(liability_weight_init)
            .bind::<Nullable<Numeric>, _>(liability_weight_maint)
            .bind::<Nullable<Numeric>, _>(deposit_limit)
            .bind::<Nullable<Numeric>, _>(optimal_utilization_rate)
            .bind::<Nullable<Numeric>, _>(plateau_interest_rate)
            .bind::<Nullable<Numeric>, _>(max_interest_rate)
            .bind::<Nullable<Numeric>, _>(insurance_fee_fixed_apr)
            .bind::<Nullable<Numeric>, _>(insurance_ir_fee)
            .bind::<Nullable<Numeric>, _>(protocol_fixed_fee_apr)
            .bind::<Nullable<Numeric>, _>(protocol_ir_fee)
            .bind::<Nullable<Integer>, _>(operational_state_id)
            .bind::<Nullable<Integer>, _>(oracle_setup_id)
            .bind::<Nullable<Text>, _>(oracle_keys)
            .bind::<Nullable<Numeric>, _>(borrow_limit)
            .bind::<Nullable<Integer>, _>(risk_tier_id)
            .bind::<Nullable<Numeric>, _>(total_asset_value_init_limit)
            .bind::<Nullable<Integer>, _>(oracle_max_age)
            .bind::<Timestamp, _>(timestamp)
            .bind::<Numeric, _>(slot)
            .bind::<Text, _>(tx_sig)
            .bind::<Bool, _>(in_flashloan)
            .bind::<Text, _>(call_stack)
            .load::<IdResult>(connection)?
            .pop()
            .expect("Expected at least one id")
            .id;

        Ok(id)
    }
}
