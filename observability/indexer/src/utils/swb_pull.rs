use fixed::types::I80F48;
use marginfi::constants::EXP_10_I80F48;
use marginfi::state::price::SwitchboardPullPriceFeed;
use solana_sdk::account_info::AccountInfo;
use switchboard_on_demand::PullFeedAccountData;

use super::crossbar::SimulatedPrice;

pub fn overwrite_price_from_sim(
    current_data: &mut SwitchboardPullPriceFeed,
    simulated_price: &SimulatedPrice,
) {
    let value: i128 = I80F48::from_num(simulated_price.value)
        .checked_mul(EXP_10_I80F48[switchboard_on_demand::PRECISION as usize])
        .unwrap()
        .to_num();
    let std_dev: i128 = I80F48::from_num(simulated_price.std_dev)
        .checked_mul(EXP_10_I80F48[switchboard_on_demand::PRECISION as usize])
        .unwrap()
        .to_num();

    current_data.feed.result.value = value;
    current_data.feed.result.std_dev = std_dev;
    // other fields are ignored because not used by the indexer
}

pub fn load_swb_pull_account(account_info: &AccountInfo) -> anyhow::Result<PullFeedAccountData> {
    let bytes = &account_info.data.borrow().to_vec();

    if bytes
        .as_ptr()
        .align_offset(std::mem::align_of::<PullFeedAccountData>())
        != 0
    {
        return Err(anyhow::anyhow!("Invalid alignment"));
    }

    let num = bytes.len() / std::mem::size_of::<PullFeedAccountData>();
    let mut vec: Vec<PullFeedAccountData> = Vec::with_capacity(num);

    unsafe {
        vec.set_len(num);
        std::ptr::copy_nonoverlapping(
            bytes[8..std::mem::size_of::<PullFeedAccountData>() + 8].as_ptr(),
            vec.as_mut_ptr() as *mut u8,
            bytes.len(),
        );
    }

    Ok(vec[0])
}
