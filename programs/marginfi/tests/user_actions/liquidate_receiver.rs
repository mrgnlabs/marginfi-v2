// TODO Set up tests where the liquidator always has the asset to repay, e.g. they can seize and
// repay without restriction or need to swap at some third-party venue.

// TODO happy path init liquidation record without permission. Health improves,
// `liquidation_flat_sol_fee` is collected, and liquidator makes a profit. validate the liquidator
// made a profit that is within the `liquidation_max_fee`

// TODO happy path with a withdraw from several positions and/or repay to several positions.

// TODO happy path of liquidating a user who is unhealthy with liquidate_start and liquidate_end by
// withdrawing some of their asset position and repaying some of their debt.

// TODO validate we cannot liquidate healthy accounts with liquidate_start

// TODO validate the various invariants in liquidate_start related to instruction introspection
// (e.g. must be first, must be exclusive, end must be last, and only withdraw/repay/init record may
// appear)

// TODO validate we cannot reduce the health of account when liquidating it