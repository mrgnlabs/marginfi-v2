WITH upsert_user_liquidator AS (
    INSERT INTO users (address)
    VALUES ($8)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_user_liquidator AS (
    SELECT id FROM users WHERE address = $8
), combined_user_liquidator AS (
    SELECT id FROM upsert_user_liquidator
    UNION ALL
    SELECT id FROM existing_user_liquidator
    LIMIT 1
),
upsert_user_liquidatee AS (
    INSERT INTO users (address)
    VALUES ($9)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_user_liquidatee AS (
    SELECT id FROM users WHERE address = $9
), combined_user_liquidatee AS (
    SELECT id FROM upsert_user_liquidatee
    UNION ALL
    SELECT id FROM existing_user_liquidatee
    LIMIT 1
),
upsert_account_liquidator AS (
    INSERT INTO accounts (address, user_id)
    VALUES ($10, (SELECT id FROM combined_user_liquidator))
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_account_liquidator AS (
    SELECT id FROM accounts WHERE address = $10
), combined_account_liquidator AS (
    SELECT id FROM upsert_account_liquidator
    UNION ALL
    SELECT id FROM existing_account_liquidator
    LIMIT 1
),
upsert_account_liquidatee AS (
    INSERT INTO accounts (address, user_id)
    VALUES ($11, (SELECT id FROM combined_user_liquidatee))
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_account_liquidatee AS (
    SELECT id FROM accounts WHERE address = $11
), combined_account_liquidatee AS (
    SELECT id FROM upsert_account_liquidatee
    UNION ALL
    SELECT id FROM existing_account_liquidatee
    LIMIT 1
),
upsert_mint_asset AS (
    INSERT INTO mints (address, symbol, decimals)
    VALUES ($12, $13, $14)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_mint_asset AS (
    SELECT id FROM mints WHERE address = $12
), combined_mint_asset AS (
    SELECT id FROM upsert_mint_asset
    UNION ALL
    SELECT id FROM existing_mint_asset
    LIMIT 1
),
upsert_mint_liability AS (
    INSERT INTO mints (address, symbol, decimals)
    VALUES ($15, $16, $17)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_mint_liability AS (
    SELECT id FROM mints WHERE address = $15
), combined_mint_liability AS (
    SELECT id FROM upsert_mint_liability
    UNION ALL
    SELECT id FROM existing_mint_liability
    LIMIT 1
),
upsert_bank_asset AS (
    INSERT INTO banks (address, mint_id)
    VALUES ($18, (SELECT id FROM combined_mint_asset))
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_bank_asset AS (
    SELECT id FROM banks WHERE address = $18
), combined_bank_asset AS (
    SELECT id FROM upsert_bank_asset
    UNION ALL
    SELECT id FROM existing_bank_asset
    LIMIT 1
),
upsert_bank_liability AS (
    INSERT INTO banks (address, mint_id)
    VALUES ($19, (SELECT id FROM combined_mint_liability))
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_bank_liability AS (
    SELECT id FROM banks WHERE address = $19
), combined_bank_liability AS (
    SELECT id FROM upsert_bank_liability
    UNION ALL
    SELECT id FROM existing_bank_liability
    LIMIT 1
)
INSERT INTO liquidate_events (timestamp, slot, tx_sig, in_flashloan, call_stack, outer_ix_index, inner_ix_index, liquidator_account_id, liquidatee_account_id, liquidator_user_id, asset_bank_id, liability_bank_id, asset_amount)
VALUES ($1, $2, $3, $4, $5, $6, $7, (SELECT id FROM combined_account_liquidator), (SELECT id FROM combined_account_liquidatee), (SELECT id FROM combined_user_liquidator), (SELECT id FROM combined_bank_asset), (SELECT id FROM combined_bank_liability), $20)
RETURNING id;
