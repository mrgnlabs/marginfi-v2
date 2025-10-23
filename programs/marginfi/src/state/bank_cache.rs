use fixed::types::I80F48;
use marginfi_type_crate::types::{milli_to_u32, BankCache};

pub fn update_interest_rates(bank_cache: &mut BankCache, interest_rates: &ComputedInterestRates) {
    bank_cache.base_rate = milli_to_u32(interest_rates.base_rate_apr);
    bank_cache.lending_rate = milli_to_u32(interest_rates.lending_rate_apr);
    bank_cache.borrowing_rate = milli_to_u32(interest_rates.borrowing_rate_apr);
}

#[derive(Debug, Clone)]
pub struct ComputedInterestRates {
    pub base_rate_apr: I80F48,
    pub lending_rate_apr: I80F48,
    pub borrowing_rate_apr: I80F48,
    pub group_fee_apr: I80F48,
    pub insurance_fee_apr: I80F48,
    pub protocol_fee_apr: I80F48,
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixed::types::I80F48;

    #[test]
    fn test_apr_to_u32_boundaries_and_midpoints() {
        let neg_apr = I80F48::from_num(-2.0);
        let zero_apr = I80F48::from_num(0.0);
        let full_apr = I80F48::from_num(10.0); // 1000%
        let one_apr = I80F48::from_num(1.0); // 100%
        let five_apr = I80F48::from_num(5.0); // 500%
        let over_apr = I80F48::from_num(15.0); // over max

        assert_eq!(milli_to_u32(neg_apr), 0);
        assert_eq!(milli_to_u32(zero_apr), 0);
        assert_eq!(milli_to_u32(full_apr), u32::MAX);
        assert_eq!(milli_to_u32(one_apr), u32::MAX / 10);
        assert_eq!(milli_to_u32(five_apr), u32::MAX / 2);
        assert_eq!(milli_to_u32(over_apr), u32::MAX); // clamped by to_num::<u32>()
    }
}
