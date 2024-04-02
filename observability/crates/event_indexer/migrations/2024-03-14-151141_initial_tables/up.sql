-- Enums

CREATE TABLE "bank_operational_state" (
	"id" SERIAL PRIMARY KEY,
	"name" VARCHAR NOT NULL
);

INSERT INTO "bank_operational_state" ("id", "name")
VALUES (0, 'Paused'),
			 (1, 'Operational'),
			 (2, 'ReduceOnly');

CREATE TABLE "oracle_setup" (
	"id" SERIAL PRIMARY KEY,
	"name" VARCHAR NOT NULL
);

INSERT INTO "oracle_setup" ("id", "name")
VALUES (0, 'None'),
			 (1, 'PythEma'),
			 (2, 'SwitchboardV2');

CREATE TABLE "risk_tier" (
	"id" SERIAL PRIMARY KEY,
	"name" VARCHAR NOT NULL
);

INSERT INTO "risk_tier" ("id", "name")
VALUES (0, 'Collateral'),
			 (1, 'Isolated');

-- Entities

CREATE TABLE "mints"(
	"id" SERIAL PRIMARY KEY,
	"address" VARCHAR NOT NULL UNIQUE,
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

-- Events

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
	"price" NUMERIC,

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
	"price" NUMERIC,

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
	"price" NUMERIC,
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
	"price" NUMERIC,
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
	"price" NUMERIC,

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
	"asset_price" NUMERIC,
	"liability_price" NUMERIC,

	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('liquidate_events');

CREATE TABLE "create_bank_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,

	"bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"asset_weight_init" NUMERIC NOT NULL,
  "asset_weight_maint" NUMERIC NOT NULL,
  "liability_weight_init" NUMERIC NOT NULL,
  "liability_weight_maint" NUMERIC NOT NULL,
  "deposit_limit" NUMERIC NOT NULL,
	"optimal_utilization_rate" NUMERIC NOT NULL,
  "plateau_interest_rate" NUMERIC NOT NULL,
  "max_interest_rate" NUMERIC NOT NULL,
  "insurance_fee_fixed_apr" NUMERIC NOT NULL,
  "insurance_ir_fee" NUMERIC NOT NULL,
  "protocol_fixed_fee_apr" NUMERIC NOT NULL,
  "protocol_ir_fee" NUMERIC NOT NULL,
  "operational_state_id" INT4 NOT NULL REFERENCES "bank_operational_state"("id"),
  "oracle_setup_id" INT4 NOT NULL REFERENCES "oracle_setup"("id"),
  "oracle_keys" VARCHAR NOT NULL,
  "borrow_limit" NUMERIC NOT NULL,
  "risk_tier_id" INT4 NOT NULL REFERENCES "risk_tier"("id"),
	"total_asset_value_init_limit" NUMERIC NOT NULL,

	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('create_bank_events');

CREATE TABLE "configure_bank_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,

	"bank_id" INT4 NOT NULL REFERENCES "banks"("id"),
	"asset_weight_init" NUMERIC,
  "asset_weight_maint" NUMERIC,
  "liability_weight_init" NUMERIC,
  "liability_weight_maint" NUMERIC,
  "deposit_limit" NUMERIC,
  "borrow_limit" NUMERIC,
  "operational_state_id" INT4 REFERENCES "bank_operational_state"("id"),
  "oracle_setup_id" INT4 REFERENCES "oracle_setup"("id"),
  "oracle_keys" VARCHAR,
	"optimal_utilization_rate" NUMERIC,
  "plateau_interest_rate" NUMERIC,
  "max_interest_rate" NUMERIC,
  "insurance_fee_fixed_apr" NUMERIC,
  "insurance_ir_fee" NUMERIC,
  "protocol_fixed_fee_apr" NUMERIC,
  "protocol_ir_fee" NUMERIC,
  "risk_tier_id" INT4 REFERENCES "risk_tier"("id"),
	"total_asset_value_init_limit" NUMERIC,

	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('configure_bank_events');
