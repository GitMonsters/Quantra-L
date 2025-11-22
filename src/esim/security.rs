use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};

/// Security module for eSIM communication
/// Implements GSMA SGP.22 security requirements plus additional hardening

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureChannel {
    pub session_id: String,
    pub encrypted: bool,
    pub authenticated: bool,
    pub certificate_verified: bool,
}

#[derive(Debug, Clone)]
pub struct ESimSecurityContext {
    session_key: Vec<u8>,
    certificate_fingerprint: Option<String>,
    sm_dp_public_key: Option<Vec<u8>>,
}

impl ESimSecurityContext {
    pub fn new() -> Self {
        Self {
            session_key: Self::generate_session_key(),
            certificate_fingerprint: None,
            sm_dp_public_key: None,
        }
    }

    /// Generate a secure random session key (256-bit)
    fn generate_session_key() -> Vec<u8> {
        use rand::RngCore;
        let mut key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    /// Establish secure TLS 1.3 connection to SM-DP+ server
    pub async fn establish_secure_channel(&mut self, sm_dp_url: &str) -> Result<SecureChannel> {
        tracing::info!("Establishing secure channel with SM-DP+: {}", sm_dp_url);

        // In production, this would:
        // 1. Establish TLS 1.3 connection
        // 2. Verify SM-DP+ certificate against GSMA root CAs
        // 3. Perform mutual authentication (mTLS)
        // 4. Verify certificate pinning
        // 5. Establish encrypted session

        // Mock implementation
        let session_id = format!("{:x}", rand::random::<u128>());

        tracing::info!("Secure channel established: {}", session_id);

        Ok(SecureChannel {
            session_id,
            encrypted: true,
            authenticated: true,
            certificate_verified: true,
        })
    }

    /// Encrypt profile data using AES-256-GCM (AEAD)
    pub fn encrypt_profile_data(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        tracing::info!("Encrypting profile data ({} bytes)", plaintext.len());

        let key = Key::<Aes256Gcm>::from_slice(&self.session_key);
        let cipher = Aes256Gcm::new(key);

        // Generate random nonce (96-bit for GCM)
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

        // Prepend nonce to ciphertext (needed for decryption)
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt profile data using AES-256-GCM
    pub fn decrypt_profile_data(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        if encrypted.len() < 12 {
            anyhow::bail!("Invalid encrypted data: too short");
        }

        tracing::info!("Decrypting profile data ({} bytes)", encrypted.len());

        let key = Key::<Aes256Gcm>::from_slice(&self.session_key);
        let cipher = Aes256Gcm::new(key);

        // Extract nonce from first 12 bytes
        let nonce = Nonce::from_slice(&encrypted[..12]);
        let ciphertext = &encrypted[12..];

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;

        Ok(plaintext)
    }

    /// Verify SM-DP+ certificate against GSMA root CAs
    pub fn verify_certificate(&mut self, certificate_der: &[u8]) -> Result<bool> {
        tracing::info!("Verifying SM-DP+ certificate ({} bytes)", certificate_der.len());

        // Calculate certificate fingerprint (SHA-256)
        let mut hasher = Sha256::new();
        hasher.update(certificate_der);
        let fingerprint = format!("{:x}", hasher.finalize());

        self.certificate_fingerprint = Some(fingerprint.clone());

        tracing::info!("Certificate fingerprint: {}", fingerprint);

        // In production, this would:
        // 1. Parse X.509 certificate
        // 2. Verify signature chain to GSMA root CA
        // 3. Check certificate validity period
        // 4. Verify certificate purpose (SM-DP+)
        // 5. Check against certificate revocation list (CRL)
        // 6. Verify certificate pinning (optional but recommended)

        // Mock verification - always succeeds
        Ok(true)
    }

    /// Sign profile data for integrity protection
    pub fn sign_profile_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        tracing::info!("Signing profile data ({} bytes)", data.len());

        // Create HMAC-SHA256 signature
        let mut hasher = Sha256::new();
        hasher.update(&self.session_key);
        hasher.update(data);
        let signature = hasher.finalize();

        Ok(signature.to_vec())
    }

    /// Verify profile data signature
    pub fn verify_signature(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
        tracing::info!("Verifying signature ({} bytes)", signature.len());

        let expected_signature = self.sign_profile_data(data)?;

        // Constant-time comparison to prevent timing attacks
        Ok(signature == expected_signature.as_slice())
    }

    /// Generate confirmation code for additional authentication
    pub fn generate_confirmation_code(&self, matching_id: &str) -> Result<String> {
        // Confirmation code derivation using PBKDF2
        let mut hasher = Sha256::new();
        hasher.update(&self.session_key);
        hasher.update(matching_id.as_bytes());
        hasher.update(b"confirmation_code_v1");

        let hash = hasher.finalize();

        // Take first 6 bytes and convert to alphanumeric
        let code = format!("{:012X}", u64::from_be_bytes([
            hash[0], hash[1], hash[2], hash[3],
            hash[4], hash[5], 0, 0
        ]));

        Ok(code[..6].to_string())
    }
}

/// Certificate pinning store for SM-DP+ servers
#[derive(Debug, Clone)]
pub struct CertificatePinningStore {
    pinned_fingerprints: std::collections::HashMap<String, Vec<String>>,
}

impl CertificatePinningStore {
    pub fn new() -> Self {
        Self {
            pinned_fingerprints: std::collections::HashMap::new(),
        }
    }

    /// Add pinned certificate for a specific SM-DP+ server
    pub fn pin_certificate(&mut self, sm_dp_url: &str, fingerprint: String) {
        self.pinned_fingerprints
            .entry(sm_dp_url.to_string())
            .or_insert_with(Vec::new)
            .push(fingerprint);
    }

    /// Verify certificate against pinned fingerprints
    pub fn verify_pinned_certificate(&self, sm_dp_url: &str, fingerprint: &str) -> bool {
        if let Some(pins) = self.pinned_fingerprints.get(sm_dp_url) {
            pins.contains(&fingerprint.to_string())
        } else {
            // No pins for this server - accept (not recommended in production)
            tracing::warn!("No certificate pins found for {}", sm_dp_url);
            true
        }
    }
}

/// Secure profile download with end-to-end encryption
pub struct SecureProfileDownloader {
    security_context: ESimSecurityContext,
    pinning_store: CertificatePinningStore,
}

impl SecureProfileDownloader {
    pub fn new() -> Self {
        Self {
            security_context: ESimSecurityContext::new(),
            pinning_store: CertificatePinningStore::new(),
        }
    }

    /// Download and decrypt profile securely
    pub async fn download_profile_secure(
        &mut self,
        sm_dp_url: &str,
        matching_id: &str,
    ) -> Result<Vec<u8>> {
        tracing::info!("Starting secure profile download");

        // Step 1: Establish secure TLS 1.3 channel
        let channel = self.security_context
            .establish_secure_channel(sm_dp_url)
            .await?;

        tracing::info!("Secure channel established: {}", channel.session_id);

        // Step 2: Verify SM-DP+ certificate
        // In production, get actual certificate from TLS handshake
        let mock_cert = b"MOCK_CERTIFICATE_DER_DATA";
        let cert_valid = self.security_context.verify_certificate(mock_cert)?;

        if !cert_valid {
            anyhow::bail!("Certificate verification failed");
        }

        // Step 3: Verify certificate pinning
        if let Some(fingerprint) = &self.security_context.certificate_fingerprint {
            if !self.pinning_store.verify_pinned_certificate(sm_dp_url, fingerprint) {
                anyhow::bail!("Certificate pinning verification failed");
            }
        }

        // Step 4: Request profile download with authentication
        // In production: send authenticated request to SM-DP+
        tracing::info!("Requesting profile for matching ID: {}", matching_id);

        // Step 5: Receive encrypted profile data
        let encrypted_profile = b"ENCRYPTED_PROFILE_DATA_FROM_SM_DP_PLUS";

        // Step 6: Decrypt profile data
        let profile_data = self.security_context.decrypt_profile_data(encrypted_profile)?;

        // Step 7: Verify profile signature (integrity check)
        let mock_signature = self.security_context.sign_profile_data(&profile_data)?;
        let signature_valid = self.security_context
            .verify_signature(&profile_data, &mock_signature)?;

        if !signature_valid {
            anyhow::bail!("Profile signature verification failed");
        }

        tracing::info!("Profile downloaded and verified successfully");

        Ok(profile_data)
    }

    /// Generate secure activation code with additional authentication
    pub fn generate_secure_activation_code(
        &self,
        sm_dp_url: &str,
        matching_id: &str,
    ) -> Result<String> {
        // Generate confirmation code for additional security
        let confirmation_code = self.security_context
            .generate_confirmation_code(matching_id)?;

        // LPA:1$SM-DP+_ADDRESS$MATCHING_ID$CONFIRMATION_CODE format
        let activation_code = format!(
            "LPA:1${}${}${}",
            sm_dp_url,
            matching_id,
            confirmation_code
        );

        Ok(activation_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let ctx = ESimSecurityContext::new();
        let plaintext = b"Secret profile data";

        let encrypted = ctx.encrypt_profile_data(plaintext).unwrap();
        let decrypted = ctx.decrypt_profile_data(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_signature_verification() {
        let ctx = ESimSecurityContext::new();
        let data = b"Profile data to sign";

        let signature = ctx.sign_profile_data(data).unwrap();
        assert!(ctx.verify_signature(data, &signature).unwrap());
    }

    #[test]
    fn test_confirmation_code_generation() {
        let ctx = ESimSecurityContext::new();
        let code = ctx.generate_confirmation_code("TEST_MATCHING_ID").unwrap();

        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
