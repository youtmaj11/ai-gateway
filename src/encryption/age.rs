use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose, Engine as _};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::fmt;

const ENCRYPTION_PREFIX: &str = "ENC:";
const NONCE_SIZE: usize = 12;

#[derive(Debug, Clone)]
pub struct AgeEncryption {
    key: Option<String>,
}

#[derive(Debug)]
pub enum EncryptionError {
    Aead(String),
    Decode(base64::DecodeError),
    Utf8(std::string::FromUtf8Error),
    MissingKey,
}

impl fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncryptionError::Aead(err) => write!(f, "encryption error: {err}"),
            EncryptionError::Decode(err) => write!(f, "base64 decode error: {err}"),
            EncryptionError::Utf8(err) => write!(f, "UTF-8 error: {err}"),
            EncryptionError::MissingKey => write!(f, "missing encryption key"),
        }
    }
}

impl std::error::Error for EncryptionError {}

impl From<aes_gcm::Error> for EncryptionError {
    fn from(err: aes_gcm::Error) -> Self {
        EncryptionError::Aead(err.to_string())
    }
}

impl From<base64::DecodeError> for EncryptionError {
    fn from(err: base64::DecodeError) -> Self {
        EncryptionError::Decode(err)
    }
}

impl From<std::string::FromUtf8Error> for EncryptionError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        EncryptionError::Utf8(err)
    }
}

fn derive_key(secret: &str) -> Key<Aes256Gcm> {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b"ai-gateway-encryption-key");
    let result = hasher.finalize();

    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&result);
    Key::<Aes256Gcm>::from_slice(&key_bytes).clone()
}

impl AgeEncryption {
    pub fn new(key: Option<String>) -> Self {
        Self { key }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, EncryptionError> {
        if self.key.as_ref().map(|k| k.is_empty()).unwrap_or(true) {
            return Ok(plaintext.to_string());
        }

        let key = derive_key(self.key.as_ref().unwrap());
        let cipher = Aes256Gcm::new(&key);

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())?;
        let mut payload = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        payload.extend_from_slice(&nonce_bytes);
        payload.extend_from_slice(&ciphertext);

        Ok(format!("{}{}", ENCRYPTION_PREFIX, general_purpose::STANDARD.encode(payload)))
    }

    pub fn decrypt(&self, ciphertext: &str) -> Result<String, EncryptionError> {
        if !ciphertext.starts_with(ENCRYPTION_PREFIX) {
            return Ok(ciphertext.to_string());
        }

        let key = self.key.as_ref().ok_or(EncryptionError::MissingKey)?;
        let key = derive_key(key);
        let cipher = Aes256Gcm::new(&key);

        let encoded = &ciphertext[ENCRYPTION_PREFIX.len()..];
        let payload = general_purpose::STANDARD.decode(encoded)?;

        if payload.len() < NONCE_SIZE {
            return Err(EncryptionError::Aead("invalid payload".into()));
        }

        let (nonce_bytes, ciphertext) = payload.split_at(NONCE_SIZE);
        let plaintext = cipher.decrypt(Nonce::from_slice(nonce_bytes), ciphertext)?;

        Ok(String::from_utf8(plaintext)?)
    }
}
