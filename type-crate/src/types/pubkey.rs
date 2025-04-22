use bytemuck::{Pod, Zeroable};
use core::fmt;
use std::str::FromStr;

/// A 32‐byte ed25519 public key, suitable for zero‐copy.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    /// Construct from raw bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Pubkey(bytes)
    }

    /// Return the raw byte array.
    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl Default for Pubkey {
    /// The “all zero” key.
    fn default() -> Self {
        Pubkey([0; 32])
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // e.g. Pubkey(3M9n…aZK)
        write!(f, "Pubkey({})", self)
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // base58‐encode
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

impl FromStr for Pubkey {
    type Err = bs58::decode::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(s).into_vec()?;
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Pubkey(array))
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
