ALTER TABLE "create_bank_events"
ADD COLUMN "oracle_max_age" INTEGER DEFAULT 0 NOT NULL;

ALTER TABLE "configure_bank_events"
ADD COLUMN "oracle_max_age" INTEGER DEFAULT 0 NOT NULL;
