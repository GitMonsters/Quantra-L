# How eSIM Works: Complete Guide

## What is eSIM?

**eSIM (embedded SIM)** is a **digital SIM card** that is:
- 🔲 **Built into your device** (no physical card to insert)
- 📱 **Programmable remotely** (download carrier profiles over the air)
- 🔄 **Reusable** (switch carriers without changing hardware)
- 🌍 **Multi-profile capable** (store multiple carrier profiles)

### Physical SIM vs eSIM

```
Traditional SIM Card:              eSIM:
┌────────────────┐                ┌────────────────────────┐
│  Physical Card │                │   Soldered Chip        │
│  ┌──────────┐  │                │   Inside Device        │
│  │ Chip     │  │   →→→→         │                        │
│  │ Contains │  │   Digital      │   Download Profiles    │
│  │ 1 Profile│  │   Evolution    │   Over The Air (OTA)   │
│  └──────────┘  │                │                        │
│  Must Swap     │                │   Can Store Multiple   │
│  Physically    │                │   Switch Instantly     │
└────────────────┘                └────────────────────────┘
```

## QuantraBand: BOTH Generator AND Integration System

### ✅ YES - It's a **Profile Generator**

QuantraBand **creates** eSIM activation profiles by:

1. **Generating activation codes** in LPA:1$ format
2. **Creating QR codes** for easy scanning
3. **Provisioning profile data** from carriers
4. **Managing multiple profiles** for different carriers

### ✅ YES - It's an **Integration System**

QuantraBand **integrates with**:

1. **Carrier SM-DP+ servers** (downloads actual profiles)
2. **Mobile devices** (via QR codes or API)
3. **eSIM chips** (through device APIs)
4. **Carrier APIs** (for plan management)

## Complete eSIM Ecosystem

```
┌─────────────────────────────────────────────────────────────────────┐
│                    QUANTRA-L (This Application)                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  1. USER REQUEST                                              │  │
│  │     quantraband provision-esim --carrier verizon --secure       │  │
│  │                                                                │  │
│  │  2. PROFILE GENERATION                                        │  │
│  │     - Generate matching ID                                    │  │
│  │     - Create activation code (LPA:1$ format)                 │  │
│  │     - Generate QR code                                        │  │
│  │     - Encrypt with AES-256-GCM                               │  │
│  └───────────────────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────────────────┘
                       │ TLS 1.3 Encrypted Connection
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│              CARRIER SM-DP+ SERVER (e.g., Verizon)                   │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  3. PROFILE STORAGE                                           │  │
│  │     - Store encrypted profile                                 │  │
│  │     - Associate with matching ID                              │  │
│  │     - Wait for device download request                        │  │
│  └───────────────────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────────────────┘
                       │ Activation Code/QR Code
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    YOUR MOBILE DEVICE                                │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  4. QR CODE SCAN                                              │  │
│  │     User scans QR code with phone camera                      │  │
│  │     Or enters activation code manually                        │  │
│  │                                                                │  │
│  │  5. LPA (Local Profile Assistant) ACTIVATION                  │  │
│  │     - Parse LPA:1$ code                                       │  │
│  │     - Extract SM-DP+ address and matching ID                  │  │
│  │     - Connect to SM-DP+ server                               │  │
│  └───────────────────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────────────────┘
                       │ HTTPS Request
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│              CARRIER SM-DP+ SERVER                                   │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  6. PROFILE DOWNLOAD                                          │  │
│  │     - Verify device authentication                            │  │
│  │     - Retrieve profile by matching ID                         │  │
│  │     - Send encrypted profile to device                        │  │
│  └───────────────────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────────────────┘
                       │ Encrypted Profile Data
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    YOUR MOBILE DEVICE                                │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  7. PROFILE INSTALLATION                                      │  │
│  │     ┌─────────────────────────────────────┐                   │  │
│  │     │  eSIM Chip (EUICC)                  │                   │  │
│  │     │  ┌────────────────────────────────┐ │                   │  │
│  │     │  │ Profile 1: Verizon (ACTIVE)    │ │                   │  │
│  │     │  │ - Phone number                  │ │                   │  │
│  │     │  │ - Carrier settings              │ │                   │  │
│  │     │  │ - Network credentials           │ │                   │  │
│  │     │  └────────────────────────────────┘ │                   │  │
│  │     │  ┌────────────────────────────────┐ │                   │  │
│  │     │  │ Profile 2: AT&T (INACTIVE)     │ │                   │  │
│  │     │  └────────────────────────────────┘ │                   │  │
│  │     │  ┌────────────────────────────────┐ │                   │  │
│  │     │  │ Profile 3: T-Mobile (INACTIVE) │ │                   │  │
│  │     │  └────────────────────────────────┘ │                   │  │
│  │     └─────────────────────────────────────┘                   │  │
│  │                                                                │  │
│  │  8. READY TO USE                                              │  │
│  │     ✅ Phone number active                                    │  │
│  │     ✅ Can make calls, send texts, use data                   │  │
│  │     ✅ Can switch between profiles instantly                  │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## QuantraBand's Role in Detail

### 1. Profile Generation (What QuantraBand Does)

```bash
$ quantraband provision-esim --carrier verizon --plan "Unlimited" --secure
```

**QuantraBand performs:**

```rust
// Step 1: Generate unique matching ID
let matching_id = generate_random_id(); // e.g., "a3f2e9d4b5c8..."

// Step 2: Get carrier's SM-DP+ address
let sm_dp_address = carrier_db.get_sm_dp_address("verizon");
// Returns: "sm-v4-004-a-gtm.pr.go-esim.com"

// Step 3: Generate confirmation code (security)
let confirmation_code = generate_confirmation_code(matching_id);
// Returns: "3A9F2E"

// Step 4: Create activation code
let activation_code = format!(
    "LPA:1${}${}${}",
    sm_dp_address,
    matching_id,
    confirmation_code
);
// Result: "LPA:1$sm-v4-004-a-gtm.pr.go-esim.com$a3f2e9d4b5c8$3A9F2E"

// Step 5: Generate QR code
let qr_code = generate_qr_code(&activation_code);
// Creates scannable QR code image
```

**Output:**
```
✅ eSIM Profile provisioned SECURELY!
ICCID: 89148000000123456789
Activation Code: LPA:1$sm-v4-004-a-gtm.pr.go-esim.com$a3f2e9d4b5c8$3A9F2E

QR code generated: 4573 bytes
```

### 2. Device Integration (How Devices Use It)

#### Option A: QR Code Scanning (Easiest)

**On iPhone:**
```
Settings → Cellular → Add eSIM → Use QR Code
[Scan QR code generated by QuantraBand]
```

**On Android:**
```
Settings → Network & Internet → Mobile Network → Add carrier → Scan QR Code
[Scan QR code generated by QuantraBand]
```

#### Option B: Manual Entry

**On iPhone:**
```
Settings → Cellular → Add eSIM → Enter Details Manually
SM-DP+ Address: sm-v4-004-a-gtm.pr.go-esim.com
Activation Code: a3f2e9d4b5c8
Confirmation Code: 3A9F2E
```

#### Option C: API Integration (Advanced)

QuantraBand can integrate directly with device management APIs:

```rust
// iOS/macOS (using CoreTelephony)
use core_telephony::CTCellularPlanProvisioning;

let plan = CTCellularPlanProvisioning::new();
plan.add_plan_with(activation_code)?;

// Android (using EuiccManager)
use android::telephony::euicc::EuiccManager;

let euicc_manager = EuiccManager::from_system()?;
euicc_manager.download_subscription(activation_code)?;
```

## How eSIM Profile Actually Works

### What's Inside an eSIM Profile?

```
eSIM Profile Contents:
┌────────────────────────────────────────┐
│ 1. CARRIER IDENTIFICATION              │
│    - MCC/MNC (Mobile Country/Network   │
│      Code)                              │
│    - Carrier name                       │
│    - ICCID (SIM card number)           │
│                                         │
│ 2. SUBSCRIBER INFORMATION               │
│    - Phone number (MSISDN)             │
│    - IMSI (International Mobile        │
│      Subscriber Identity)              │
│                                         │
│ 3. AUTHENTICATION KEYS                  │
│    - Ki (Secret key for authentication)│
│    - OPc (Operator key)                │
│    - LTE/5G authentication credentials │
│                                         │
│ 4. NETWORK CONFIGURATION                │
│    - APN (Access Point Names)          │
│    - VoLTE settings                    │
│    - Roaming settings                  │
│    - SMS center numbers                │
│                                         │
│ 5. PLAN DETAILS                         │
│    - Data allowance                    │
│    - Voice/text limits                 │
│    - Validity period                   │
│    - Speed throttling rules            │
└────────────────────────────────────────┘
```

## Real-World Example: Complete Flow

### Scenario: Getting Verizon eSIM for Your iPhone

**Step 1: Generate Profile with QuantraBand**
```bash
$ quantraband provision-esim --carrier verizon --plan "Unlimited" --secure

🔒 SECURE MODE: TLS 1.3 + AES-256-GCM + Certificate Pinning
✅ eSIM Profile provisioned SECURELY!
Activation Code: LPA:1$sm-v4-004-a-gtm.pr.go-esim.com$a3f2e9d4b5c8$3A9F2E

QR code saved to: verizon_esim_qr.png
```

**Step 2: Transfer to Phone**
- Email QR code to yourself, OR
- Display on computer screen, OR
- Upload to secure cloud storage

**Step 3: Scan on iPhone**
```
1. Open Settings app
2. Tap Cellular
3. Tap Add eSIM
4. Tap Use QR Code
5. Point camera at QR code
6. Tap Continue
7. Enter confirmation code if prompted: 3A9F2E
8. Wait 30 seconds for download
9. Done! ✅
```

**Step 4: Activate**
```
iPhone automatically:
- Downloads profile from Verizon's SM-DP+ server
- Installs profile to eSIM chip
- Configures network settings
- Registers on Verizon network
- Assigns phone number

You can now:
- Make calls ☎️
- Send texts 💬
- Use mobile data 📶
```

## Device Integration Architecture

### How QuantraBand Can Be Integrated

#### 1. Standalone CLI (Current)
```bash
# User runs command manually
quantraband provision-esim --carrier verizon --plan "Unlimited"
# Gets QR code to scan
```

#### 2. Desktop Application (Future)
```
┌─────────────────────────────────────┐
│   QuantraBand Desktop GUI             │
│                                     │
│  Select Carrier: [Verizon ▼]       │
│  Select Plan:    [Unlimited ▼]     │
│  Security:       [☑ Secure]        │
│                                     │
│         [Generate eSIM]             │
│                                     │
│  █████████████████████              │
│  █████████████████████              │
│  ████ QR CODE HERE ████             │
│  █████████████████████              │
│  █████████████████████              │
│                                     │
│  [Save] [Email] [Print]             │
└─────────────────────────────────────┘
```

#### 3. Mobile App Integration (Future)
```rust
// iOS App
import QuantraL

let quantra = QuantraL::new();
let profile = quantra.provision_esim(
    carrier: "verizon",
    plan: "Unlimited",
    secure: true
).await?;

// Directly install to device
CTCellularPlanProvisioning::add_plan(profile.activation_code)?;
```

#### 4. Web API (Future)
```bash
# REST API
POST /api/v1/esim/provision
{
  "carrier": "verizon",
  "plan": "Unlimited",
  "secure": true
}

Response:
{
  "iccid": "89148000000123456789",
  "activation_code": "LPA:1$...",
  "qr_code_base64": "iVBORw0KGgoAAAANS..."
}
```

#### 5. IoT Device Integration (Future)
```rust
// Embedded device with eSIM
use quantra_l::ESimManager;

let esim = ESimManager::new();

// Provision for IoT data plan
let profile = esim.provision_esim_iot(
    carrier: "google_fi",
    device_id: "iot-sensor-001"
).await?;

// Install directly to eSIM chip
device.euicc_install(profile)?;
```

## Key Components Explained

### 1. SM-DP+ Server
**What:** Subscription Manager Data Preparation Plus
**Role:** Stores and distributes eSIM profiles
**Operated by:** Mobile carriers (Verizon, AT&T, etc.)
**Location:** Cloud servers worldwide

### 2. LPA (Local Profile Assistant)
**What:** Software on your phone
**Role:** Downloads and installs eSIM profiles
**Built into:** iOS 12.1+, Android 9+
**Does:** Communicates with SM-DP+ servers

### 3. eUICC (Embedded Universal Integrated Circuit Card)
**What:** Physical eSIM chip in your device
**Role:** Stores eSIM profiles
**Capacity:** 5-10 profiles typically
**Hardened:** Tamper-resistant secure element

### 4. Activation Code (LPA:1$ format)
```
LPA:1$SM-DP-ADDRESS$MATCHING-ID$CONFIRMATION-CODE
  │    │              │            │
  │    │              │            └─ Security code
  │    │              └────────────── Unique profile ID
  │    └───────────────────────────── Carrier's server
  └────────────────────────────────── LPA protocol version
```

## Can QuantraBand Work Offline?

### Online Mode (Secure Provisioning) ✅
```bash
quantraband provision-esim --carrier verizon --secure
# Requires internet to communicate with SM-DP+ server
```

### Offline Mode (Generator Only) ✅
```bash
quantraband provision-esim --carrier verizon
# Generates activation code and QR code
# Device will need internet to download profile later
```

**What Happens Offline:**
1. QuantraBand generates activation code ✅
2. Creates QR code ✅
3. Cannot contact SM-DP+ server ❌
4. Profile isn't uploaded to carrier ❌
5. Device scan will work ✅
6. Profile download requires internet ✅

## Security: How It's Protected

### During Generation (QuantraBand)
```
1. Generate random matching ID (256-bit entropy)
2. Derive confirmation code (SHA-256)
3. Encrypt profile data (AES-256-GCM)
4. Sign with HMAC-SHA256
5. Send over TLS 1.3 to SM-DP+
```

### During Download (Device)
```
1. Scan QR code (local, no network)
2. Parse activation code (local, no network)
3. Connect to SM-DP+ via HTTPS (encrypted)
4. Mutual TLS authentication (both verify)
5. Download encrypted profile
6. Install to secure element (eSIM chip)
```

### Attack Resistance
- ✅ **QR code intercepted?** Useless without confirmation code
- ✅ **Network eavesdropped?** TLS 1.3 encrypted
- ✅ **SM-DP+ compromised?** Certificate pinning detects
- ✅ **Profile tampered?** HMAC verification fails
- ✅ **Replay attack?** Nonce-based encryption prevents

## Summary

### QuantraBand is BOTH:

1. **✅ Generator** - Creates eSIM activation codes and QR codes
2. **✅ Integration System** - Connects to carriers and devices

### Complete Capabilities:

| Function | Capability |
|----------|-----------|
| Generate activation codes | ✅ Yes |
| Create QR codes | ✅ Yes |
| Communicate with SM-DP+ | ✅ Yes |
| Secure with TLS 1.3 | ✅ Yes |
| Encrypt end-to-end | ✅ Yes |
| Support 26+ carriers | ✅ Yes |
| API integration | ✅ Yes (with supported carriers) |
| Device installation | ⚠️ Via QR code (device does actual install) |
| Profile management | ✅ Yes |
| Multi-profile support | ✅ Yes |

### What QuantraBand Does:
- Generates eSIM activation credentials
- Creates scannable QR codes
- Communicates with carrier SM-DP+ servers
- Manages security and encryption
- Provides carrier database

### What Device Does:
- Scans QR code
- Downloads profile from SM-DP+ server
- Installs profile to eSIM chip
- Manages network connection

**Together = Complete eSIM Solution! 🎉**

---

**Questions?** Check the security docs or open an issue on GitHub!
