WITH upsert_authority AS (
    INSERT INTO users (address)
    VALUES ($1)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_authority AS (
    SELECT id FROM users WHERE address = $1
), combined_authority AS (
    SELECT id FROM upsert_authority
    UNION ALL
    SELECT id FROM existing_authority
    LIMIT 1
),
upsert_account AS (
    INSERT INTO accounts (address, user_id)
    VALUES ($2, (SELECT id FROM combined_authority))
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_account AS (
    SELECT id
), combined_account AS (
    SELECT id
),
upsert_bank_mint AS (
    INSERT INTO mints (address, symbol, decimals)
    VALUES ($3, $4, $5)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_bank_mint AS (
    SELECT id
), combined_bank_mint AS (
    SELECT id
),
upsert_bank AS (
    INSERT INTO banks (address, mint_id)
    VALUES ($6, (SELECT id FROM combined_bank_mint))
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_bank AS (
    SELECT id
), combined_bank AS (
    SELECT id
),
upsert_emission_mint AS (
    INSERT INTO mints (address, symbol, decimals)
    VALUES ($7, $8, $9)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_emission_mint AS (
    SELECT id
), combined_emission_mint AS (
    SELECT id
)
INSERT INTO withdraw_emissions_events (timestamp, slot, tx_sig, in_flashloan, call_stack, account_id, authority_id, bank_id, emission_mint_id, amount)
VALUES ($10, $11, $12, $13, $14, (SELECT id FROM combined_account), (SELECT id FROM combined_authority), (SELECT id FROM combined_bank), (SELECT id FROM combined_emission_mint), $15)
RETURNING id;
