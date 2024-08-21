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
