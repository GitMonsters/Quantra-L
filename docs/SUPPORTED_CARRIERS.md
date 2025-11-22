# Supported eSIM Carriers in Quantra-L

## Overview

Quantra-L supports **26+ carriers** across **multiple countries** with full eSIM provisioning capabilities.

## Quick Start

```bash
# List all carriers
quantra-l list-carriers

# Search for specific carrier
quantra-l list-carriers --search "verizon"

# Filter by country
quantra-l list-carriers --country "United States"

# Provision eSIM securely
quantra-l provision-esim --carrier verizon --plan "Unlimited" --secure
```

## Supported Carriers by Region

### 🇺🇸 United States (6 carriers)

| Carrier | ID | Secure API | Notes |
|---------|-----|------------|-------|
| **Verizon Wireless** | `verizon` | ✅ | Largest US carrier |
| **AT&T** | `att` | ✅ | Full eSIM support |
| **T-Mobile USA** | `tmobile` | ✅ | Merged with Sprint |
| **Sprint** | `sprint` | ✅ | Now T-Mobile network |
| **Cricket Wireless** | `cricket` | ❌ | AT&T MVNO |
| **U.S. Cellular** | `uscellular` | ❌ | Regional carrier |

**MVNOs (Virtual Carriers):**
| Carrier | ID | Secure API | Parent Network |
|---------|-----|------------|----------------|
| **Google Fi** | `google_fi` | ✅ | T-Mobile/US Cellular |
| **Mint Mobile** | `mint_mobile` | ❌ | T-Mobile |
| **Visible** | `visible` | ❌ | Verizon |

### 🇬🇧 United Kingdom (3 carriers)

| Carrier | ID | Secure API |
|---------|-----|------------|
| **EE (Everything Everywhere)** | `ee` | ❌ |
| **Vodafone UK** | `vodafone_uk` | ✅ |
| **O2 UK** | `o2_uk` | ❌ |

### 🇩🇪 Germany (2 carriers)

| Carrier | ID | Secure API |
|---------|-----|------------|
| **Deutsche Telekom** | `telekom_de` | ❌ |
| **Vodafone Germany** | `vodafone_de` | ❌ |

### 🇨🇦 Canada (3 carriers)

| Carrier | ID | Secure API |
|---------|-----|------------|
| **Rogers Wireless** | `rogers` | ❌ |
| **Bell Canada** | `bell` | ❌ |
| **TELUS** | `telus` | ❌ |

### 🇦🇺 Australia (2 carriers)

| Carrier | ID | Secure API |
|---------|-----|------------|
| **Telstra** | `telstra` | ❌ |
| **Optus** | `optus` | ❌ |

### 🇯🇵 Japan (2 carriers)

| Carrier | ID | Confirmation Required |
|---------|-----|----------------------|
| **NTT DoCoMo** | `ntt_docomo` | ✅ Required |
| **SoftBank** | `softbank` | ❌ Optional |

### 🇨🇳 China (2 carriers)

| Carrier | ID | Confirmation Required |
|---------|-----|----------------------|
| **China Mobile** | `china_mobile` | ✅ Required |
| **China Unicom** | `china_unicom` | ✅ Required |

### 🌍 Global / Travel eSIM (3 providers)

| Provider | ID | Coverage | Secure API |
|----------|-----|----------|------------|
| **Airalo** | `airalo` | 190+ countries | ✅ |
| **Truphone** | `truphone` | Global | ❌ |
| **GigSky** | `gigsky` | 190+ countries | ❌ |

## Security Features by Carrier

### Full Security Support (TLS 1.3 + E2EE)

These carriers support Quantra-L's complete security stack:

- ✅ **Verizon** (`verizon`)
- ✅ **AT&T** (`att`)
- ✅ **T-Mobile** (`tmobile`)
- ✅ **Vodafone UK** (`vodafone_uk`)
- ✅ **Google Fi** (`google_fi`)
- ✅ **Airalo** (`airalo`)

Use `--secure` flag for maximum protection:
```bash
quantra-l provision-esim --carrier verizon --plan "Unlimited" --secure
```

### Basic Security (TLS only)

All other carriers support standard TLS encryption but may not have API endpoints for additional features.

## Usage Examples

### Example 1: Provision Verizon eSIM (Secure)
```bash
quantra-l provision-esim \
  --carrier verizon \
  --plan "Unlimited 5G" \
  --secure
```

Output:
```
🔒 SECURE MODE: TLS 1.3 + AES-256-GCM + Certificate Pinning
✅ eSIM Profile provisioned SECURELY!
🔐 Encryption: AES-256-GCM
🔐 Transport: TLS 1.3
🔐 Authentication: Mutual TLS (mTLS)
🔐 Integrity: HMAC-SHA256
```

### Example 2: Provision Google Fi (International Travel)
```bash
quantra-l provision-esim \
  --carrier google_fi \
  --plan "Flexible" \
  --secure
```

### Example 3: Provision Airalo (Global eSIM)
```bash
quantra-l provision-esim \
  --carrier airalo \
  --plan "Europe 5GB" \
  --secure
```

### Example 4: List Carriers by Country
```bash
# List all US carriers
quantra-l list-carriers --country "United States"

# List all Japanese carriers
quantra-l list-carriers --country "Japan"
```

## Carrier Requirements

### Confirmation Codes

Some carriers require additional confirmation codes:

- **China Mobile** (`china_mobile`) - SMS or email confirmation
- **China Unicom** (`china_unicom`) - SMS or email confirmation
- **NTT DoCoMo** (`ntt_docomo`) - SMS confirmation
- **U.S. Cellular** (`uscellular`) - Email confirmation

When provisioning these carriers, you'll receive a confirmation code that must be entered separately.

### Device Compatibility

All carriers require:
- eSIM-capable device
- Unlocked device (except carrier-specific plans)
- iOS 12.1+ or Android 9.0+ (with eSIM support)

## SM-DP+ Addresses

Each carrier has a unique SM-DP+ (Subscription Manager Data Preparation) server address. Quantra-L automatically uses the correct address based on the carrier ID.

**Examples:**
- Verizon: `sm-v4-004-a-gtm.pr.go-esim.com`
- AT&T: `sm-dp-plus.att.com`
- T-Mobile: `prod.smpc.t-mobile.com`
- Google Fi: `prod.smdp.rsp.goog`

## Adding New Carriers

Want to add a carrier? Edit `src/esim/carriers.rs`:

```rust
self.add_carrier("new_carrier", CarrierInfo {
    name: "New Carrier Name".to_string(),
    country: "Country".to_string(),
    sm_dp_address: "sm-dp-plus.newcarrier.com".to_string(),
    supports_esim: true,
    requires_confirmation: false,
    api_endpoint: Some("https://api.newcarrier.com/esim".to_string()),
});
```

Then rebuild:
```bash
cargo build --release
```

## API Integration

Carriers with API endpoints support advanced features:

### Verizon API
```bash
# Automatic plan discovery
# Real-time activation status
# Multi-device management
```

### AT&T API
```bash
# Plan recommendations
# Usage tracking
# Account integration
```

### Google Fi API
```bash
# International roaming
# Data-only eSIMs
# Family plan support
```

### Airalo API
```bash
# 190+ country coverage
# Instant activation
# Top-up support
```

## Testing

Test carrier integration without actual provisioning:

```bash
# Dry run (list carriers)
quantra-l list-carriers

# Search for specific carrier
quantra-l list-carriers --search "tmobile"

# Check carrier details
quantra-l list-carriers --search "verizon"
```

## Troubleshooting

### "Carrier not found"
```bash
# Check available carriers
quantra-l list-carriers

# Use exact carrier ID (lowercase)
quantra-l provision-esim --carrier verizon  # ✅ Correct
quantra-l provision-esim --carrier Verizon  # ❌ Wrong (case-sensitive)
```

### "Confirmation code required"
Some carriers (China Mobile, NTT DoCoMo) require additional verification. Check your email/SMS for the confirmation code.

### "SM-DP+ connection failed"
1. Check internet connectivity
2. Verify carrier supports eSIM in your region
3. Try with `--secure` flag for better reliability

## Future Carriers

Planned additions:
- 🇫🇷 Orange France
- 🇮🇹 TIM Italy
- 🇪🇸 Movistar Spain
- 🇰🇷 SK Telecom Korea
- 🇮🇳 Jio India
- 🇧🇷 Vivo Brazil

Want a specific carrier? [Open an issue on GitHub](https://github.com/GitMonsters/Quantra-L/issues)!

## License Compliance

All carrier integrations comply with:
- GSMA SGP.22 specification
- Carrier terms of service
- Regional telecommunications regulations

---

**Total Carriers**: 26+
**Countries Covered**: 10+
**Global eSIM Providers**: 3
**Last Updated**: 2025-11-21
