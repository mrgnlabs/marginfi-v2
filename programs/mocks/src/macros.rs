#[macro_export]
macro_rules! pool_auth_signer_seeds {
    ($pool_auth:expr) => {
        &[
            &$pool_auth.nonce.to_le_bytes(),
            b"pool_auth".as_ref(),
            &[$pool_auth.bump_seed],
        ]
    };
}

#[macro_export]
macro_rules! state_signer_seeds {
    ($state:expr) => {
        &[b"state", $state.admin.as_ref(), &[$state.bump]]
    };
}