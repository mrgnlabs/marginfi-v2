-- Your SQL goes here

CREATE TABLE "mints"(
	"id" SERIAL PRIMARY KEY,
	"address" VARCHAR NOT NULL,
	"symbol" VARCHAR NOT NULL,
	"decimals" SMALLINT NOT NULL
);

CREATE TABLE "banks"(
	"id" SERIAL PRIMARY KEY,
	"address" VARCHAR NOT NULL UNIQUE,
	"mint" INT4 NOT NULL REFERENCES "mints"("id")
);

CREATE TABLE "users"(
	"id" SERIAL PRIMARY KEY,
	"address" VARCHAR NOT NULL UNIQUE
);

CREATE TABLE "accounts"(
	"id" SERIAL PRIMARY KEY,
	"address" VARCHAR NOT NULL UNIQUE,
	"user_id" INT4 NOT NULL REFERENCES "users"("id")
);

CREATE TABLE "create_account_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"authority_id" INT4 NOT NULL REFERENCES "users"("id")
);

CREATE TABLE "transfer_account_authority_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"old_authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"new_authority_id" INT4 NOT NULL REFERENCES "users"("id")
);

CREATE TABLE "deposit_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"amount" NUMERIC NOT NULL
);

CREATE TABLE "borrow_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"amount" NUMERIC NOT NULL
);

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
	"all" BOOLEAN NOT NULL
);

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
	"all" BOOLEAN NOT NULL
);

CREATE TABLE "withdraw_emissions_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"account_id" INT4 NOT NULL REFERENCES "accounts"("id"),
	"authority_id" INT4 NOT NULL REFERENCES "users"("id"),
	"bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"emission_mint" VARCHAR NOT NULL,
	"amount" NUMERIC NOT NULL
);
