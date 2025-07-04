#[macro_export]
macro_rules! assert_struct_size {
    ($struct: ty, $size: expr) => {
        static_assertions::const_assert_eq!(std::mem::size_of::<$struct>(), $size);
    };
}

#[macro_export]
macro_rules! assert_struct_align {
    ($struct: ty, $align: expr) => {
        static_assertions::const_assert_eq!(std::mem::align_of::<$struct>(), $align);
    };
}
