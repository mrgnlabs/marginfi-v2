WITH upsert_group AS (
    INSERT INTO groups (address, admin)
    VALUES ($17, $18)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_group AS (
    SELECT id FROM groups WHERE address = $17
), combined_group AS (
    SELECT id FROM upsert_group
    UNION ALL
    SELECT id FROM existing_group
    LIMIT 1
),
upsert_authority AS (
    INSERT INTO users (address)
    VALUES ($8)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_authority AS (
    SELECT id FROM users WHERE address = $8
), combined_authority AS (
    SELECT id FROM upsert_authority
    UNION ALL
    SELECT id FROM existing_authority
    LIMIT 1
),
upsert_account AS (
    INSERT INTO accounts (address, user_id, group_id)
    VALUES ($9, (SELECT id FROM combined_authority), (SELECT id FROM combined_group))
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_account AS (
    SELECT id FROM accounts WHERE address = $9
), combined_account AS (
    SELECT id FROM upsert_account
    UNION ALL
    SELECT id FROM existing_account
    LIMIT 1
),
upsert_bank_mint AS (
    INSERT INTO mints (address, symbol, decimals)
    VALUES ($10, $11, $12)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_bank_mint AS (
    SELECT id FROM mints WHERE address = $10
), combined_bank_mint AS (
    SELECT id FROM upsert_bank_mint
    UNION ALL
    SELECT id FROM existing_bank_mint
    LIMIT 1
),
upsert_bank AS (
    INSERT INTO banks (address, mint_id, group_id)
    VALUES ($13, (SELECT id FROM combined_bank_mint), (SELECT id FROM combined_group))
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_bank AS (
    SELECT id FROM banks WHERE address = $13
), combined_bank AS (
    SELECT id
),
upsert_emission_mint AS (
    INSERT INTO mints (address, symbol, decimals)
    VALUES ($14, $15, $16)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_emission_mint AS (
    SELECT id FROM mints WHERE address = $14
), combined_emission_mint AS (
    SELECT id FROM upsert_emission_mint
    UNION ALL
    SELECT id FROM existing_emission_mint
    LIMIT 1
)
INSERT INTO withdraw_emissions_events (timestamp, slot, tx_sig, in_flashloan, call_stack, outer_ix_index, inner_ix_index, account_id, authority_id, bank_id, emission_mint_id, amount)
VALUES ($1, $2, $3, $4, $5, $6, $7, (SELECT id FROM combined_account), (SELECT id FROM combined_authority), (SELECT id FROM combined_bank), (SELECT id FROM combined_emission_mint), $19)
RETURNING id;
