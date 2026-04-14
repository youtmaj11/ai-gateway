use age::{armor::{ArmoredReader, ArmoredWriter}, Decryptor, Encryptor, DecryptError, EncryptError, secrecy::SecretString};
use std::fmt;
use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub struct AgeEncryption {
    key: Option<String>,
}

#[derive(Debug)]
pub enum EncryptionError {
    Encrypt(EncryptError),
    Decrypt(DecryptError),
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
    MissingKey,
}

impl fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncryptionError::Age(err) => write!(f, "age encryption error: {err}"),
            EncryptionError::Io(err) => write!(f, "IO error: {err}"),
            EncryptionError::Utf8(err) => write!(f, "UTF-8 error: {err}"),
            EncryptionError::MissingKey => write!(f, "missing encryption key"),
        }
    }
}

impl std::error::Error for EncryptionError {}

impl From<EncryptError> for EncryptionError {
    fn from(err: EncryptError) -> Self {
        EncryptionError::Encrypt(err)
    }
}

impl From<DecryptError> for EncryptionError {
    fn from(err: DecryptError) -> Self {
        EncryptionError::Decrypt(err)
    }
}

impl From<std::io::Error> for EncryptionError {
    fn from(err: std::io::Error) -> Self {
        EncryptionError::Io(err)
    }
}

impl From<std::string::FromUtf8Error> for EncryptionError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        EncryptionError::Utf8(err)
    }
}

impl AgeEncryption {
    pub fn new(key: Option<String>) -> Self {
        Self { key }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, EncryptionError> {
        if self.key.as_ref().map(|k| k.is_empty()).unwrap_or(true) {
            return Ok(plaintext.to_string());
        }

        let passphrase = SecretString::new(self.key.clone().unwrap());
        let encryptor = Encryptor::with_user_passphrase(passphrase);
        let mut output = Vec::new();
        let mut armor = ArmoredWriter::new(&mut output);
        let mut writer = encryptor.wrap_output(&mut armor)?;
        writer.write_all(plaintext.as_bytes())?;
        writer.finish()?;
        armor.finish()?;

        Ok(String::from_utf8(output)?)
    }

    pub fn decrypt(&self, ciphertext: &str) -> Result<String, EncryptionError> {
        if !ciphertext.starts_with("-----BEGIN AGE ENCRYPTED FILE-----") {
            return Ok(ciphertext.to_string());
        }

        let key = self.key.as_ref().ok_or(EncryptionError::MissingKey)?;
        let passphrase = SecretString::new(key.clone());
        let mut input = ArmoredReader::new(ciphertext.as_bytes());
        let decryptor = Decryptor::new(&mut input)?;

        let mut output = Vec::new();
        match decryptor {
            Decryptor::Passphrase(d) => {
                let mut reader = d.decrypt(&passphrase, None)?;
                reader.read_to_end(&mut output)?;
            }
            _ => return Err(EncryptionError::MissingKey),
        }

        Ok(String::from_utf8(output)?)
    }
}
