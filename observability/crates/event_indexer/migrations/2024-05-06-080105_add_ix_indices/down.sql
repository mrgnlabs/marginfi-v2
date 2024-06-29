ALTER TABLE "unknown_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "unknown_events" DROP COLUMN "inner_ix_index";

ALTER TABLE "create_account_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "create_account_events" DROP COLUMN "inner_ix_index";

ALTER TABLE "transfer_account_authority_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "transfer_account_authority_events" DROP COLUMN "inner_ix_index";

ALTER TABLE "deposit_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "deposit_events" DROP COLUMN "inner_ix_index";

ALTER TABLE "borrow_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "borrow_events" DROP COLUMN "inner_ix_index";

ALTER TABLE "repay_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "repay_events" DROP COLUMN "inner_ix_index";

ALTER TABLE "withdraw_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "withdraw_events" DROP COLUMN "inner_ix_index";

ALTER TABLE "withdraw_emissions_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "withdraw_emissions_events" DROP COLUMN "inner_ix_index";

ALTER TABLE "liquidate_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "liquidate_events" DROP COLUMN "inner_ix_index";

ALTER TABLE "create_bank_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "create_bank_events" DROP COLUMN "inner_ix_index";

ALTER TABLE "configure_bank_events" DROP COLUMN "outer_ix_index";
ALTER TABLE "configure_bank_events" DROP COLUMN "inner_ix_index";
