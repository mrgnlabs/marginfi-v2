-- Add oracle_max_age to create_bank_events with a default value and NOT NULL constraint
ALTER TABLE "create_bank_events"
ADD COLUMN "oracle_max_age" INTEGER DEFAULT 0 NOT NULL;

-- Add oracle_max_age to configure_bank_events with a default value and NOT NULL constraint
ALTER TABLE "configure_bank_events"
ADD COLUMN "oracle_max_age" INTEGER DEFAULT 0 NOT NULL;
