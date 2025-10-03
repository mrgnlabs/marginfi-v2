use marginfi_type_crate::types::LiquidationRecord;

use crate::MarginfiResult;

pub trait LiquidationRecordImpl {
    fn _placeholder(&self) -> MarginfiResult;
}

impl LiquidationRecordImpl for LiquidationRecord {
    fn _placeholder(&self) -> MarginfiResult {
        Ok(())
    }
}
