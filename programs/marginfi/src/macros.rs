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
