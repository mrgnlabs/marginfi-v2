#[macro_export]
/// This macro will emit the provided custom program error and log where the error happened,
/// if the condition is not met.
macro_rules! check {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            let error_code: $crate::errors::MarginfiError = $err;
            #[cfg(not(feature = "test-bpf"))]
            anchor_lang::prelude::msg!(
                "Error \"{}\" thrown at {}:{}",
                error_code,
                file!(),
                line!()
            );
            return Err(error_code.into());
        }
    };

    ($cond:expr, $err:expr, $($arg:tt)*) => {
        if !($cond) {
            let error_code: $crate::errors::MarginfiError = $err;
            #[cfg(not(feature = "test-bpf"))]
            anchor_lang::prelude::msg!(
                "Error \"{}\" thrown at {}:{}",
                error_code,
                file!(),
                line!()
            );
            #[cfg(not(feature = "test-bpf"))]
            anchor_lang::prelude::msg!($($arg)*);
            return Err(error_code.into());
        }
    };
}

#[macro_export]
macro_rules! math_error {
    () => {{
        || {
            let error_code = $crate::errors::MarginfiError::MathError;
            anchor_lang::prelude::msg!(
                "Error \"{}\" thrown at {}:{}",
                error_code,
                file!(),
                line!()
            );
            error_code
        }
    }};
}

#[macro_export]
macro_rules! set_if_some {
    ($attr: expr, $val: expr) => {
        if let Some(val) = $val {
            anchor_lang::prelude::msg!("Setting {} to {:?}", stringify!($attr), val);
            $attr = val.into()
        }
    };
}

#[macro_export]
macro_rules! bank_seed {
    ($vault_type: expr, $bank_pk: expr) => {
        &[$vault_type.get_seed(), &$bank_pk.to_bytes()] as &[&[u8]]
    };
}

#[macro_export]
macro_rules! bank_authority_seed {
    ($vault_type: expr, $bank_pk: expr) => {
        &[$vault_type.get_authority_seed(), &$bank_pk.to_bytes()] as &[&[u8]]
    };
}

#[macro_export]
macro_rules! bank_signer {
    ($vault_type: expr, $bank_pk: expr, $authority_bump: expr) => {
        &[&[
            $vault_type.get_authority_seed().as_ref(),
            &$bank_pk.to_bytes(),
            &[$authority_bump],
        ]]
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "debug")]
        {
            anchor_lang::prelude::msg!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! assert_struct_size {
    ($struct: ty, $size: expr) => {
        static_assertions::const_assert_eq!(std::mem::size_of::<$struct>(), $size);
    };
}
