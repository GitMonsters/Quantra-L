# eSIM Communication Security in Quantra-L

## Overview

Quantra-L implements comprehensive security for eSIM provisioning and communication, following GSMA SGP.22 standards with additional hardening layers.

## Security Layers

### 1. Transport Layer Security (TLS 1.3)

**What it protects:**
- Communication between device and SM-DP+ server
- Protection against man-in-the-middle (MITM) attacks
- Server authentication

**Implementation:**
```rust
// Establish TLS 1.3 connection with perfect forward secrecy
let channel = security_context.establish_secure_channel(sm_dp_url).await?;
```

**Features:**
- ✅ TLS 1.3 (latest protocol version)
- ✅ Strong cipher suites only (AES-256-GCM, ChaCha20-Poly1305)
- ✅ Perfect Forward Secrecy (PFS)
- ✅ No fallback to older TLS versions

### 2. Mutual TLS Authentication (mTLS)

**What it protects:**
- Both server AND client authentication
- Prevents unauthorized access to SM-DP+ server
- Device identity verification

**Implementation:**
```rust
// Both parties present certificates
// Client verifies SM-DP+ certificate
// SM-DP+ verifies device certificate
```

**Benefits:**
- ✅ Two-way authentication
- ✅ Strong device identity
- ✅ Prevents impersonation attacks

### 3. Certificate Verification & Pinning

**What it protects:**
- Against compromised Certificate Authorities (CAs)
- Against rogue SM-DP+ servers
- Long-term trust establishment

**Implementation:**
```rust
// Verify certificate chain to GSMA root CAs
security_context.verify_certificate(certificate_der)?;

// Pin specific SM-DP+ certificates
pinning_store.pin_certificate(sm_dp_url, fingerprint);
pinning_store.verify_pinned_certificate(sm_dp_url, fingerprint)?;
```

**Features:**
- ✅ Verification against GSMA root CAs
- ✅ Certificate pinning for known SM-DP+ servers
- ✅ SHA-256 fingerprint validation
- ✅ Certificate revocation checking (CRL)
- ✅ Validity period verification

### 4. End-to-End Encryption (E2EE)

**What it protects:**
- eSIM profile data confidentiality
- Protection even if TLS is compromised
- Multi-layered security

**Implementation:**
```rust
// AES-256-GCM authenticated encryption
let encrypted = security_context.encrypt_profile_data(plaintext)?;
let decrypted = security_context.decrypt_profile_data(encrypted)?;
```

**Algorithm:** AES-256-GCM (Authenticated Encryption with Associated Data)
- ✅ 256-bit keys (virtually unbreakable)
- ✅ Galois/Counter Mode (authenticated)
- ✅ Built-in integrity protection
- ✅ Nonce-based (no key reuse)

### 5. Data Integrity Protection

**What it protects:**
- Detects tampering with profile data
- Ensures data hasn't been modified
- Cryptographic proof of authenticity

**Implementation:**
```rust
// HMAC-SHA256 signatures
let signature = security_context.sign_profile_data(data)?;
let valid = security_context.verify_signature(data, signature)?;
```

**Features:**
- ✅ HMAC-SHA256 signatures
- ✅ Constant-time comparison (prevents timing attacks)
- ✅ Session-bound keys

### 6. Confirmation Code Authentication

**What it protects:**
- Additional authentication factor
- Prevents unauthorized profile downloads
- Out-of-band verification

**Implementation:**
```rust
// Generate cryptographic confirmation code
let code = security_context.generate_confirmation_code(matching_id)?;
// Format: 6-character hexadecimal (e.g., "A3F2E9")
```

**Benefits:**
- ✅ Additional authentication layer
- ✅ Human-verifiable
- ✅ Cryptographically derived (not random)
- ✅ Tied to specific profile

## Security Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Device/Client                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  1. Generate Session Key (256-bit random)            │  │
│  │  2. Establish TLS 1.3 connection                      │  │
│  │  3. Verify SM-DP+ certificate (GSMA root CAs)         │  │
│  │  4. Check certificate pinning                         │  │
│  │  5. Mutual authentication (mTLS)                      │  │
│  └───────────────────────────────────────────────────────┘  │
└───────────────────┬─────────────────────────────────────────┘
                    │ TLS 1.3 Encrypted Channel
                    │ (AES-256-GCM transport)
                    ▼
┌─────────────────────────────────────────────────────────────┐
│                     SM-DP+ Server                            │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  1. Verify device certificate                         │  │
│  │  2. Authenticate request                              │  │
│  │  3. Encrypt profile data (E2EE layer)                 │  │
│  │  4. Sign profile data (HMAC-SHA256)                   │  │
│  │  5. Send encrypted + signed profile                   │  │
│  └───────────────────────────────────────────────────────┘  │
└───────────────────┬─────────────────────────────────────────┘
                    │ Profile Data (double-encrypted)
                    ▼
┌─────────────────────────────────────────────────────────────┐
│                        Device/Client                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  1. Receive encrypted profile                         │  │
│  │  2. Decrypt TLS layer                                 │  │
│  │  3. Verify signature (integrity check)                │  │
│  │  4. Decrypt E2EE layer (AES-256-GCM)                  │  │
│  │  5. Install profile to eSIM                           │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Usage

### Basic (Insecure) Mode
```bash
quantra-l provision-esim --carrier "Verizon" --plan "Unlimited"
```

### Secure Mode (Recommended)
```bash
quantra-l provision-esim --carrier "Verizon" --plan "Unlimited" --secure
```

### Secure Mode Output
```
🔒 SECURE MODE: TLS 1.3 + AES-256-GCM + Certificate Pinning
✅ eSIM Profile provisioned SECURELY!
🔐 Encryption: AES-256-GCM
🔐 Transport: TLS 1.3
🔐 Authentication: Mutual TLS (mTLS)
🔐 Integrity: HMAC-SHA256

🔒 Security Features:
  ✓ Certificate verified against GSMA root CAs
  ✓ Certificate pinning enabled
  ✓ Profile data encrypted end-to-end
  ✓ Confirmation code required

⚠️  IMPORTANT: Store this QR code securely!
   Only share via encrypted channels
```

## Threat Model

### Protected Against:

1. **Man-in-the-Middle (MITM) Attacks**
   - TLS 1.3 encryption
   - Certificate verification
   - Certificate pinning

2. **Eavesdropping**
   - TLS 1.3 transport encryption
   - E2EE profile encryption
   - Perfect Forward Secrecy

3. **Data Tampering**
   - HMAC signatures
   - GCM authentication tags
   - Integrity verification

4. **Impersonation**
   - Mutual TLS authentication
   - Certificate verification
   - Confirmation codes

5. **Replay Attacks**
   - Nonce-based encryption
   - Session-bound keys
   - Timestamp validation

6. **Compromised CAs**
   - Certificate pinning
   - Multi-layer trust model

### Defense in Depth

Even if one layer is compromised:
- **TLS broken** → E2EE still protects profile data
- **CA compromised** → Certificate pinning detects rogue certificates
- **Session key leaked** → Cannot decrypt past/future sessions (PFS)
- **Profile intercepted** → Signature verification fails if modified

## Best Practices

1. **Always use `--secure` flag in production**
2. **Store QR codes encrypted** (never in plaintext)
3. **Share activation codes only through encrypted channels**
4. **Verify confirmation codes out-of-band** (SMS, email, etc.)
5. **Regularly update certificate pins** for SM-DP+ servers
6. **Monitor for certificate expiration**
7. **Use strong device authentication** (mTLS certificates)

## Compliance

This implementation aims to comply with:

- ✅ GSMA SGP.22 (RSP Technical Specification)
- ✅ NIST SP 800-52 Rev. 2 (TLS Guidelines)
- ✅ FIPS 140-2 (Cryptographic Module Security)
- ✅ OWASP Mobile Security Guidelines

## Technical Details

### Encryption Algorithms
- **Symmetric**: AES-256-GCM
- **Hash**: SHA-256
- **MAC**: HMAC-SHA256
- **TLS**: TLS 1.3 only

### Key Sizes
- **Session Keys**: 256 bits
- **AES Keys**: 256 bits
- **HMAC Keys**: 256 bits
- **Nonces**: 96 bits (GCM standard)

### Random Number Generation
- Uses OS-provided CSPRNG (Cryptographically Secure PRNG)
- Hardware RNG when available
- Never reuses nonces

## Future Enhancements

Planned security improvements:

- [ ] Hardware Security Module (HSM) integration
- [ ] Quantum-resistant algorithms (post-quantum cryptography)
- [ ] Zero-knowledge proofs for privacy
- [ ] Secure enclave support (TPM, SGX)
- [ ] Multi-party computation for profile generation
- [ ] Blockchain-based audit trail

## References

- [GSMA SGP.22](https://www.gsma.com/esim/resources/)
- [TLS 1.3 RFC 8446](https://tools.ietf.org/html/rfc8446)
- [AES-GCM RFC 5288](https://tools.ietf.org/html/rfc5288)
- [Certificate Pinning OWASP](https://owasp.org/www-community/controls/Certificate_and_Public_Key_Pinning)

---

**Security Audit Status**: Implementation reviewed ✅
**Last Updated**: 2025-11-21
**Next Review**: 2025-12-21
