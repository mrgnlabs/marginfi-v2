CREATE TABLE "groups"(
	"id" SERIAL PRIMARY KEY,
	"address" VARCHAR NOT NULL UNIQUE,
	"admin" VARCHAR NOT NULL UNIQUE,

	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('groups');

INSERT INTO "groups" ("id", "address", "admin")
VALUES (0, '4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8', 'AZtUUe9GvTFq9kfseu9jxTioSgdSfjgmZfGQBmhVpTj1');

ALTER TABLE "banks"
ADD COLUMN "group_id" INT4 NOT NULL REFERENCES "groups"("id") DEFAULT 0;

ALTER TABLE "accounts"
ADD COLUMN "group_id" INT4 NOT NULL REFERENCES "groups"("id") DEFAULT 0;

CREATE TABLE "create_group_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"slot" NUMERIC NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"outer_ix_index" SMALLINT NOT NULL,
	"inner_ix_index" SMALLINT,

	"group_id" INT4 NOT NULL REFERENCES "groups"("id"),
  "admin" VARCHAR NOT NULL UNIQUE,

	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('create_group_events');

CREATE TABLE "configure_group_events"(
	"id" SERIAL PRIMARY KEY,
	"timestamp" TIMESTAMP NOT NULL,
	"slot" NUMERIC NOT NULL,
	"tx_sig" VARCHAR NOT NULL,
	"in_flashloan" BOOLEAN NOT NULL,
	"call_stack" VARCHAR NOT NULL,
	"outer_ix_index" SMALLINT NOT NULL,
	"inner_ix_index" SMALLINT,

	"group_id" INT4 NOT NULL REFERENCES "groups"("id"),
  "admin" VARCHAR UNIQUE,

	"created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
	"updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('configure_group_events');

