WITH upsert_group AS (
    INSERT INTO groups (address, admin)
    VALUES ($8, $9)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_group AS (
    SELECT id FROM groups WHERE address = $8
), combined_group AS (
    SELECT id FROM upsert_group
    UNION ALL
    SELECT id FROM existing_group
    LIMIT 1
)
INSERT INTO create_group_events (timestamp, slot, tx_sig, in_flashloan, call_stack, outer_ix_index, inner_ix_index, group_id, admin)
VALUES ($1, $2, $3, $4, $5, $6, $7, (SELECT id FROM combined_group), $9)
RETURNING id;
