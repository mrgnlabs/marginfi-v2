use self::pyth_crosschain::PythnetPriceFeed;

pub mod pyth_crosschain;

#[repr(C, align(8))]
#[derive(Clone, Copy)]
#[cfg_attr(any(feature = "test", feature = "client"), derive(Debug))]
pub enum NativeOracle {
    /// Adding padding to ensure the enum is 64 bytes.
    None([u8; 56]),
    PythCrosschain(PythnetPriceFeed),
}

impl Default for NativeOracle {
    fn default() -> Self {
        Self::None([0u8; 56])
    }
}
