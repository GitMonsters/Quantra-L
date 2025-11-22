pub mod keystore;

use anyhow::{Context, Result};

pub struct CryptoManager {
    keystore: keystore::KeyStore,
}

pub struct KeyPair {
    pub fingerprint: String,
    pub public_key: String,
}

impl CryptoManager {
    pub fn new(keystore_path: &str) -> Result<Self> {
        let keystore = keystore::KeyStore::new(keystore_path)?;
        Ok(Self { keystore })
    }

    pub async fn generate_keypair(&self, user_id: &str) -> Result<KeyPair> {
        tracing::info!("Generating PGP keypair for {} (mock implementation)", user_id);

        let fingerprint = format!("{:032x}", rand::random::<u128>());
        let public_key = format!("-----BEGIN PGP PUBLIC KEY BLOCK-----\n\nMock public key for {}\n\n-----END PGP PUBLIC KEY BLOCK-----", user_id);

        let keypair = KeyPair {
            fingerprint: fingerprint.clone(),
            public_key: public_key.clone(),
        };

        self.keystore.store_keypair(&fingerprint, &public_key).await?;

        tracing::warn!("Using mock PGP implementation - not suitable for production!");

        Ok(keypair)
    }

    pub async fn encrypt_message(&self, _recipient: &str, message: &[u8]) -> Result<Vec<u8>> {
        tracing::info!("Encrypting message (mock implementation)");
        tracing::warn!("Mock encryption - message is NOT actually encrypted!");

        let encrypted = format!("-----MOCK ENCRYPTED-----\n{}\n-----END MOCK-----",
            String::from_utf8_lossy(message));

        Ok(encrypted.into_bytes())
    }

    pub async fn decrypt_message(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        tracing::info!("Decrypting message (mock implementation)");

        // Extract the middle part as plaintext
        let s = String::from_utf8_lossy(encrypted);
        if let Some(start) = s.find("-----MOCK ENCRYPTED-----\n") {
            if let Some(end) = s.find("\n-----END MOCK-----") {
                let plaintext = &s[start + 24..end];
                return Ok(plaintext.as_bytes().to_vec());
            }
        }

        Ok(encrypted.to_vec())
    }

    pub async fn export_public_key(&self, keypair: &KeyPair) -> Result<String> {
        Ok(keypair.public_key.clone())
    }
}
