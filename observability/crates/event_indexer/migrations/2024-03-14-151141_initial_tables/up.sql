-- Your SQL goes here

CREATE TABLE "mints"(
	"id" SERIAL PRIMARY KEY,
	"address" VARCHAR NOT NULL,
	"symbol" VARCHAR NOT NULL,
	"decimals" SMALLINT NOT NULL,
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('mints');

CREATE TABLE "banks"(
	"id" SERIAL PRIMARY KEY,
	"address" VARCHAR NOT NULL UNIQUE,
	"mint_id" INT4 NOT NULL REFERENCES "mints"("id"),
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('banks');

CREATE TABLE "users"(
	"id" SERIAL PRIMARY KEY,
	"address" VARCHAR NOT NULL UNIQUE,
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('users');

CREATE TABLE "accounts"(
	"id" SERIAL PRIMARY KEY,
	"address" VARCHAR NOT NULL UNIQUE,
	"user_id" INT4 NOT NULL REFERENCES "users"("id"),
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('accounts');

CREATE TABLE "create_account_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('create_account_events');

CREATE TABLE "transfer_account_authority_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"old_authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"new_authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('transfer_account_authority_events');

CREATE TABLE "deposit_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"amount" NUMERIC NOT NULL,
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('deposit_events');

CREATE TABLE "borrow_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"amount" NUMERIC NOT NULL,
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('borrow_events');

CREATE TABLE "repay_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"amount" NUMERIC NOT NULL,
	"all" BOOLEAN NOT NULL,
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('repay_events');

CREATE TABLE "withdraw_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"amount" NUMERIC NOT NULL,
	"all" BOOLEAN NOT NULL,
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('withdraw_events');

CREATE TABLE "withdraw_emissions_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"emission_mint_id" INT4 NOT NULL REFERENCES "mints"("id"),
	"amount" NUMERIC NOT NULL,
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('withdraw_emissions_events');

CREATE TABLE "liquidate_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"liquidator_account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"liquidatee_account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"liquidator_user_id" INT4 NOT NULL REFERENCES "users"("id"),
	"asset_bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"liability_bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"asset_amount" NUMERIC NOT NULL,
	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('liquidate_events');
