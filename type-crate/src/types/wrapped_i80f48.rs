use bytemuck::Pod;
use bytemuck::Zeroable;
use fixed::types::I80F48;
use std::fmt::Debug;
use std::fmt::Formatter;

#[repr(C, align(8))]
#[derive(Clone, Copy, Hash, Pod, Zeroable)]
pub struct WrappedI80F48 {
    pub value: [u8; 16],
}

impl Debug for WrappedI80F48 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", I80F48::from_le_bytes(self.value))
    }
}

impl From<I80F48> for WrappedI80F48 {
    fn from(i: I80F48) -> Self {
        Self {
            value: i.to_le_bytes(),
        }
    }
}

impl From<WrappedI80F48> for I80F48 {
    fn from(w: WrappedI80F48) -> Self {
        Self::from_le_bytes(w.value)
    }
}

impl PartialEq for WrappedI80F48 {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for WrappedI80F48 {}
