use anchor_lang::error_code;

#[error_code]
pub enum ErrorCode {
    #[msg("This is an error.")]
    SomeError, // 6000
}
