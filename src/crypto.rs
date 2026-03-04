// main.rs で有効化:
// mod crypto;

use aes_gcm_siv::aead::{Aead, KeyInit, Payload};
use aes_gcm_siv::{Aes256GcmSiv, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use secrecy::{ExposeSecret, SecretVec};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::error::CryptoError;

pub const KEY_SIZE: usize = 32;
pub const NONCE_SIZE: usize = 12;
pub const ARGON2_M_COST_KIB: u32 = 65_536;
pub const ARGON2_T_COST: u32 = 3;
pub const ARGON2_P_COST: u32 = 4;
pub const ARGON2_MIN_SALT_LEN: usize = 16;

#[derive(Clone)]
pub struct KEK {
    material: SecretVec<u8>,
}

impl KEK {
    pub fn from_slice(key_material: &[u8]) -> Result<Self, CryptoError> {
        if key_material.len() != KEY_SIZE {
            return Err(CryptoError::InvalidLength {
                kind: "kek",
                expected: KEY_SIZE,
                actual: key_material.len(),
            });
        }

        Ok(Self {
            material: SecretVec::new(key_material.to_vec()),
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.material.expose_secret()
    }
}

#[derive(Clone)]
pub struct DEK {
    material: SecretVec<u8>,
}

impl DEK {
    pub fn from_slice(key_material: &[u8]) -> Result<Self, CryptoError> {
        if key_material.len() != KEY_SIZE {
            return Err(CryptoError::InvalidLength {
                kind: "dek",
                expected: KEY_SIZE,
                actual: key_material.len(),
            });
        }

        Ok(Self {
            material: SecretVec::new(key_material.to_vec()),
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.material.expose_secret()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AadData {
    pub profile_id: String,
    pub key_id: String,
    pub record_version: u32,
    pub aead_alg: String,
    pub kek_id: String,
}

impl AadData {
    pub fn to_aad_bytes(&self) -> Result<Vec<u8>, CryptoError> {
        bincode::serde::encode_to_vec(self, bincode::config::standard())
            .map_err(|err| CryptoError::AadSerialization(err.to_string()))
    }
}

pub fn encrypt_value(
    dek: &DEK,
    plaintext: &[u8],
    aad: &AadData,
) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let aad_bytes = aad.to_aad_bytes()?;
    encrypt_with_key(dek.as_bytes(), plaintext, &aad_bytes)
}

pub fn decrypt_value(
    dek: &DEK,
    nonce: &[u8],
    ciphertext: &[u8],
    aad: &AadData,
) -> Result<Vec<u8>, CryptoError> {
    let aad_bytes = aad.to_aad_bytes()?;
    decrypt_with_key(dek.as_bytes(), nonce, ciphertext, &aad_bytes)
}

pub fn wrap_dek(kek: &KEK, dek: &DEK, aad: &AadData) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let aad_bytes = aad.to_aad_bytes()?;
    encrypt_with_key(kek.as_bytes(), dek.as_bytes(), &aad_bytes)
}

pub fn unwrap_dek(
    kek: &KEK,
    nonce: &[u8],
    encrypted_dek: &[u8],
    aad: &AadData,
) -> Result<DEK, CryptoError> {
    let aad_bytes = aad.to_aad_bytes()?;
    let mut decrypted = decrypt_with_key(kek.as_bytes(), nonce, encrypted_dek, &aad_bytes)?;
    let dek = DEK::from_slice(&decrypted);
    decrypted.zeroize();
    dek
}

pub fn generate_dek() -> Result<DEK, CryptoError> {
    let mut key_material = [0_u8; KEY_SIZE];
    fill_random_bytes(&mut key_material)?;
    let dek = DEK::from_slice(&key_material);
    key_material.zeroize();
    dek
}

pub fn derive_kek_from_passphrase(passphrase: &str, salt: &[u8]) -> Result<KEK, CryptoError> {
    if salt.len() < ARGON2_MIN_SALT_LEN {
        return Err(CryptoError::SaltTooShort {
            min: ARGON2_MIN_SALT_LEN,
            actual: salt.len(),
        });
    }

    let params = Params::new(
        ARGON2_M_COST_KIB,
        ARGON2_T_COST,
        ARGON2_P_COST,
        Some(KEY_SIZE),
    )
    .map_err(|err| CryptoError::Argon2(err.to_string()))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key_material = [0_u8; KEY_SIZE];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key_material)
        .map_err(|err| CryptoError::Argon2(err.to_string()))?;

    let kek = KEK::from_slice(&key_material);
    key_material.zeroize();
    kek
}

fn fill_random_bytes(buffer: &mut [u8]) -> Result<(), CryptoError> {
    getrandom::fill(buffer).map_err(|err| CryptoError::Randomness(err.to_string()))
}

fn new_cipher(key: &[u8]) -> Result<Aes256GcmSiv, CryptoError> {
    if key.len() != KEY_SIZE {
        return Err(CryptoError::InvalidLength {
            kind: "aes-256-gcm-siv key",
            expected: KEY_SIZE,
            actual: key.len(),
        });
    }

    Aes256GcmSiv::new_from_slice(key).map_err(|_| CryptoError::InvalidLength {
        kind: "aes-256-gcm-siv key",
        expected: KEY_SIZE,
        actual: key.len(),
    })
}

fn encrypt_with_key(
    key: &[u8],
    plaintext: &[u8],
    aad_bytes: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let cipher = new_cipher(key)?;

    let mut nonce = [0_u8; NONCE_SIZE];
    fill_random_bytes(&mut nonce)?;

    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad: aad_bytes,
            },
        )
        .map_err(|_| CryptoError::EncryptionFailed)?;

    Ok((nonce.to_vec(), ciphertext))
}

fn decrypt_with_key(
    key: &[u8],
    nonce: &[u8],
    ciphertext: &[u8],
    aad_bytes: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if nonce.len() != NONCE_SIZE {
        return Err(CryptoError::InvalidLength {
            kind: "nonce",
            expected: NONCE_SIZE,
            actual: nonce.len(),
        });
    }

    let cipher = new_cipher(key)?;

    cipher
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad: aad_bytes,
            },
        )
        .map_err(|_| CryptoError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn sample_aad() -> AadData {
        AadData {
            profile_id: "default".to_string(),
            key_id: "key-123".to_string(),
            record_version: 1,
            aead_alg: "AES-256-GCM-SIV".to_string(),
            kek_id: "kek-001".to_string(),
        }
    }

    #[test]
    fn roundtrip_encrypt_decrypt() {
        let dek = generate_dek().expect("generate DEK");
        let aad = sample_aad();
        let plaintext = b"super-secret-value";

        let (nonce, ciphertext) = encrypt_value(&dek, plaintext, &aad).expect("encrypt");
        let decrypted = decrypt_value(&dek, &nonce, &ciphertext, &aad).expect("decrypt");

        assert_eq!(decrypted, plaintext);

        let kek = derive_kek_from_passphrase(
            "correct horse battery staple",
            b"0123456789abcdef",
        )
        .expect("derive KEK");
        let (wrap_nonce, encrypted_dek) = wrap_dek(&kek, &dek, &aad).expect("wrap DEK");
        let unwrapped_dek = unwrap_dek(&kek, &wrap_nonce, &encrypted_dek, &aad).expect("unwrap DEK");

        let (nonce2, ciphertext2) = encrypt_value(&unwrapped_dek, plaintext, &aad).expect("encrypt");
        let decrypted2 = decrypt_value(&unwrapped_dek, &nonce2, &ciphertext2, &aad).expect("decrypt");
        assert_eq!(decrypted2, plaintext);
    }

    #[test]
    fn aad_mismatch_is_detected() {
        let dek = generate_dek().expect("generate DEK");
        let aad = sample_aad();

        let mut wrong_aad = aad.clone();
        wrong_aad.key_id = "different-key".to_string();

        let (nonce, ciphertext) = encrypt_value(&dek, b"secret", &aad).expect("encrypt");
        let result = decrypt_value(&dek, &nonce, &ciphertext, &wrong_aad);

        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    #[test]
    fn nonce_uniqueness() {
        let dek = generate_dek().expect("generate DEK");
        let aad = sample_aad();

        let mut seen = HashSet::new();
        for _ in 0..128 {
            let (nonce, _ciphertext) = encrypt_value(&dek, b"constant", &aad).expect("encrypt");
            assert_eq!(nonce.len(), NONCE_SIZE);
            assert!(seen.insert(nonce), "nonce collision detected");
        }
    }
}
