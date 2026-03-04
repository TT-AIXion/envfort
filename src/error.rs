// main.rs で有効化:
// mod error;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid {kind} length: expected {expected} bytes, got {actual}")]
    InvalidLength {
        kind: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("salt too short: need at least {min} bytes, got {actual}")]
    SaltTooShort { min: usize, actual: usize },
    #[error("failed to generate cryptographic randomness: {0}")]
    Randomness(String),
    #[error("argon2id key derivation failed: {0}")]
    Argon2(String),
    #[error("failed to serialize AAD payload: {0}")]
    AadSerialization(String),
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed")]
    DecryptionFailed,
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),
}

#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("keychain backend error: {0}")]
    Backend(#[from] keyring::Error),
    #[error("missing keychain entry: {0}")]
    MissingEntry(String),
    #[error("keychain operation failed: {0}")]
    Operation(String),
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("keychain error: {0}")]
    Keychain(#[from] KeychainError),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid CLI argument: {0}")]
    InvalidArgument(String),
}
