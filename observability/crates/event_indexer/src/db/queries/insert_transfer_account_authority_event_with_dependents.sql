WITH upsert_old_authority AS (
    INSERT INTO users (address)
    VALUES ($1)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_old_authority AS (
    SELECT id FROM users WHERE address = $1
), combined_old_authority AS (
    SELECT id FROM upsert_old_authority
    UNION ALL
    SELECT id FROM existing_old_authority
    LIMIT 1
),
upsert_new_authority AS (
    INSERT INTO users (address)
    VALUES ($2)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_new_authority AS (
    SELECT id FROM users WHERE address = $2
), combined_new_authority AS (
    SELECT id FROM upsert_new_authority
    UNION ALL
    SELECT id FROM existing_new_authority
    LIMIT 1
),
upsert_account AS (
    INSERT INTO accounts (address, user_id)
    VALUES ($3, (SELECT id FROM combined_new_authority))
), existing_old_authority AS (
    SELECT id FROM users WHERE address = $1
), combined_old_authority AS (
    SELECT id FROM upsert_old_authority
    UNION ALL
    SELECT id FROM existing_old_authority
    LIMIT 1
),
INSERT INTO transfer_account_authority_events (timestamp, slot, tx_sig, in_flashloan, call_stack, account_id, old_authority_id, new_authority_id)
VALUES ($4, $5, $6, $7, $8, (SELECT id FROM combined_account), (SELECT id FROM combined_old_authority), (SELECT id FROM combined_new_authority))
RETURNING id;
