-- Remove oracle_max_age from create_bank_events
ALTER TABLE "create_bank_events"
DROP COLUMN "oracle_max_age";

-- Remove oracle_max_age from configure_bank_events
ALTER TABLE "configure_bank_events"
DROP COLUMN "oracle_max_age";
