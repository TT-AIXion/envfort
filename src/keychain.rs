use keyring::Entry;

use crate::crypto::{KEK, derive_kek_from_passphrase};
use crate::error::KeychainError;

const SERVICE_NAME: &str = "envfort";
const DEFAULT_PASSPHRASE_SALT: &[u8] = b"envfort-passphrase-salt-v1";

pub trait KeychainBackend {
    fn store_kek(&self, profile: &str, kek: &KEK) -> Result<(), KeychainError>;
    fn retrieve_kek(&self, profile: &str) -> Result<KEK, KeychainError>;
    fn delete_kek(&self, profile: &str) -> Result<(), KeychainError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OsKeychain;

impl OsKeychain {
    fn entry(profile: &str) -> Result<Entry, KeychainError> {
        Entry::new(SERVICE_NAME, profile).map_err(KeychainError::from)
    }
}

impl KeychainBackend for OsKeychain {
    fn store_kek(&self, profile: &str, kek: &KEK) -> Result<(), KeychainError> {
        let entry = Self::entry(profile)?;
        let encoded = encode_hex(kek.as_bytes());
        entry.set_password(&encoded)?;
        Ok(())
    }

    fn retrieve_kek(&self, profile: &str) -> Result<KEK, KeychainError> {
        let entry = Self::entry(profile)?;
        let encoded = match entry.get_password() {
            Ok(value) => value,
            Err(keyring::Error::NoEntry) => {
                return Err(KeychainError::MissingEntry(profile.to_string()));
            }
            Err(err) => return Err(KeychainError::Backend(err)),
        };

        let bytes = decode_hex(&encoded)?;
        KEK::from_slice(&bytes).map_err(|err| KeychainError::Operation(err.to_string()))
    }

    fn delete_kek(&self, profile: &str) -> Result<(), KeychainError> {
        let entry = Self::entry(profile)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(KeychainError::Backend(err)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PassphraseKeychain {
    passphrase: String,
    salt: Vec<u8>,
}

impl PassphraseKeychain {
    pub fn new(passphrase: String, salt: Vec<u8>) -> Self {
        Self { passphrase, salt }
    }

    fn profile_salt(&self, profile: &str) -> Vec<u8> {
        let mut salt = Vec::with_capacity(self.salt.len() + profile.len());
        salt.extend_from_slice(&self.salt);
        salt.extend_from_slice(profile.as_bytes());
        if salt.len() < 16 {
            salt.resize(16, b'0');
        }
        salt
    }
}

impl KeychainBackend for PassphraseKeychain {
    fn store_kek(&self, _profile: &str, _kek: &KEK) -> Result<(), KeychainError> {
        Ok(())
    }

    fn retrieve_kek(&self, profile: &str) -> Result<KEK, KeychainError> {
        let salt = self.profile_salt(profile);
        derive_kek_from_passphrase(&self.passphrase, &salt)
            .map_err(|err| KeychainError::Operation(err.to_string()))
    }

    fn delete_kek(&self, _profile: &str) -> Result<(), KeychainError> {
        Ok(())
    }
}

pub fn get_backend() -> Box<dyn KeychainBackend> {
    match std::env::var("ENVFORT_PASSPHRASE") {
        Ok(passphrase) if !passphrase.is_empty() => {
            let salt = std::env::var("ENVFORT_PASSPHRASE_SALT")
                .map(|value| value.into_bytes())
                .unwrap_or_else(|_| DEFAULT_PASSPHRASE_SALT.to_vec());
            Box::new(PassphraseKeychain::new(passphrase, salt))
        }
        _ => Box::new(OsKeychain),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_hex(input: &str) -> Result<Vec<u8>, KeychainError> {
    let bytes = input.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(KeychainError::Operation(
            "invalid hex length in keychain entry".to_string(),
        ));
    }

    let mut output = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let hi = hex_digit(chunk[0])?;
        let lo = hex_digit(chunk[1])?;
        output.push((hi << 4) | lo);
    }
    Ok(output)
}

fn hex_digit(value: u8) -> Result<u8, KeychainError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(KeychainError::Operation(
            "invalid hex payload in keychain entry".to_string(),
        )),
    }
}
