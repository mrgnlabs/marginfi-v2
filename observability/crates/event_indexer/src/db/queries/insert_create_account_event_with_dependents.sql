WITH upsert_group AS (
    INSERT INTO groups (address, admin)
    VALUES ($10, $11)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_group AS (
    SELECT id FROM groups WHERE address = $10
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
)
INSERT INTO create_account_events (timestamp, slot, tx_sig, in_flashloan, call_stack, outer_ix_index, inner_ix_index, account_id, authority_id)
VALUES ($1, $2, $3, $4, $5, $6, $7, (SELECT id FROM combined_account), (SELECT id FROM combined_authority))
RETURNING id;
