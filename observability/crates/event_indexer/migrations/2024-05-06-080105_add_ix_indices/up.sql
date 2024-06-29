ALTER TABLE "unknown_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;

ALTER TABLE "create_account_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;

ALTER TABLE "transfer_account_authority_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;

ALTER TABLE "deposit_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;

ALTER TABLE "borrow_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;

ALTER TABLE "repay_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;

ALTER TABLE "withdraw_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;

ALTER TABLE "withdraw_emissions_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;

ALTER TABLE "liquidate_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;

ALTER TABLE "create_bank_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;

ALTER TABLE "configure_bank_events"
ADD COLUMN "outer_ix_index" SMALLINT DEFAULT -1 NOT NULL,
ADD COLUMN "inner_ix_index" SMALLINT DEFAULT NULL;
