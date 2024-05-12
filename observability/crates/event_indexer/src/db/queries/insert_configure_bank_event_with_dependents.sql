WITH upsert_bank_mint AS (
    INSERT INTO mints (address, symbol, decimals)
    VALUES ($9, $10, $11)
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_bank_mint AS (
    SELECT id FROM mints WHERE address = $9
), combined_bank_mint AS (
    SELECT id FROM upsert_bank_mint
    UNION ALL
    SELECT id FROM existing_bank_mint
    LIMIT 1
),
upsert_bank AS (
    INSERT INTO banks (address, mint_id)
    VALUES ($8, (SELECT id FROM combined_bank_mint))
    ON CONFLICT (address) DO NOTHING
    RETURNING id
), existing_bank AS (
    SELECT id FROM banks WHERE address = $8
), combined_bank AS (
    SELECT id FROM upsert_bank
    UNION ALL
    SELECT id FROM existing_bank
    LIMIT 1
)

INSERT INTO configure_bank_events (timestamp, slot, tx_sig, in_flashloan, call_stack, outer_ix_index, inner_ix_index, bank_id, asset_weight_init, asset_weight_maint, liability_weight_init, liability_weight_maint, deposit_limit, optimal_utilization_rate, plateau_interest_rate, max_interest_rate, insurance_fee_fixed_apr, insurance_ir_fee, protocol_fixed_fee_apr, protocol_ir_fee, operational_state_id, oracle_setup_id, oracle_keys, borrow_limit, risk_tier_id, total_asset_value_init_limit, oracle_max_age)
VALUES ($1, $2, $3, $4, $5, $6, $7, (SELECT id FROM combined_bank), $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30)
RETURNING id;
