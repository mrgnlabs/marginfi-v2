DROP TABLE IF EXISTS "configure_group_events";

DROP TABLE IF EXISTS "create_group_events";

ALTER TABLE "accounts" DROP COLUMN IF EXISTS "group_id";
ALTER TABLE "banks" DROP COLUMN IF EXISTS "group_id";

DROP TABLE IF EXISTS "groups";
