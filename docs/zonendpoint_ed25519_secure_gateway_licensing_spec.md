# ZONENDPOINT SECURE CONNECT & TIME-BASED LICENSING SPECIFICATION (`SecConnect.md`)

> **Target Audience**: AI Coding Agent (Codex / Copilot) & Systems Engineers  
> **Architecture**: Custom Rust Security Gateway (`zon-gateway`) + Hybrid Ed25519 Challenge-Response & Time-Bound Signed License Token (`ed25519-dalek` v2)  
> **Cryptographic Strength**: 256-bit Ed25519 Keys (32 Bytes) / 512-bit Signatures (64 Bytes) / 128-bit Security Level

---

## 1. Architectural Overview & Hybrid Licensing Model

Native ClamAV daemon (`clamd`) listens on unauthenticated plaintext TCP (`3310`) or Unix Domain Socket. **Zonendpoint** secures `clamd` and enforces subscription/license expiration (e.g., 30 days, 1 year, Lifetime) using a **Two-Layer Ed25519 Cryptographic Model**:

1. **Master Authority Keypair (`master.key` / `master.pub`)**: Owned exclusively by the administrator (`anh Zon`). Used via a CLI tool (`zon-keygen`) to issue signed **License Tokens (`zon.lic`)** bound to a specific Client Public Key and an expiration timestamp (`expires_at`).
2. **Client Device Keypair (`id_ed25519.key` / `id_ed25519.pub`)**: Auto-generated on the Windows 11 Tauri Client. Used to sign per-connection live challenges from `zon-gateway` to prove identity and prevent replay attacks.
3. **Server-Side Enforcement & Override (`clients.toml`)**: `zon-gateway` verifies the Master Signature on the Client's License Token against the server's trusted UTC clock (immune to client BIOS/OS clock tampering) and checks an optional server-side revocation/override list (`clients.toml`).

```
[ Admin CLI: `zon-keygen` ]
   │  Signs (ClientPubKey + Plan + IssuedAt + ExpiresAt) with Master Private Key
   ▼
[ Windows 11 Client (Tauri/Rust) ] (Holds `id_ed25519.key` + `zon.lic`)
   │  - UI reads `zon.lic` locally to display: "Plan: 1 Year | Remaining: 364 days"
   │  1. TCP Connect to `zon-gateway` (Port 3311)
   │  2. Receive 44-byte Server Challenge (Magic + ServerTimestamp + 32B Nonce)
   │  3. Send 177-byte Auth & License Packet (LicenseToken [113B] + SessionSignature [64B])
   │  4. Receive 9-byte Auth Result (Status [1B] + EffectiveExpiresAt [8B])
   │  5. If Status == 0x01 (OK), stream standard ClamAV `zINSTREAM\0` chunks
   ▼
[ Ubuntu Server: `zon-gateway` (Port 3311, LAN/VPN) ]
   │  - Checks if ClientPubKey is revoked or overridden in `/etc/zonendpoint/clients.toml`
   │  - Verifies Master Signature on LicenseToken using embedded `MASTER_PUBLIC_KEY`
   │  - Enforces `server_now <= effective_expires_at` (Rejects with `0x02 LICENSE_EXPIRED` if expired)
   │  - Verifies Client's SessionSignature over (Challenge || LicenseToken)
   │  - Zero-copy bidirectional pipe (`tokio::io::copy_bidirectional`)
   ▼
[ Local `clamd` Daemon (`127.0.0.1:3310` or `/run/clamav/clamd.ctl`) ]
```

---

## 2. Signed License Token Specification (`zon.lic` — 113 Bytes Binary / Base64)

A License Token is a fixed **113-byte binary structure** (stored on disk at `%LOCALAPPDATA%\Zonendpoint\keys\zon.lic` as Base64 or Hex, and transmitted on the wire as raw 113 bytes). All integers are **Big-Endian**.

### 2.1. License Payload (`Bytes 0..49`, 49 Bytes)

| Offset | Field | Size | Type | Description |
| :--- | :--- | :--- | :--- | :--- |
| `0..32` | `CLIENT_PUBKEY` | 32 Bytes | `[u8; 32]` | The client device's Ed25519 `VerifyingKey` |
| `32..40` | `ISSUED_AT` | 8 Bytes | `u64` BE | Unix Epoch timestamp (seconds) when license was issued |
| `40..48` | `EXPIRES_AT` | 8 Bytes | `u64` BE | Unix Epoch timestamp (seconds) when license expires (`0xFFFFFFFFFFFFFFFF` = Lifetime) |
| `48..49` | `PLAN_CODE` | 1 Byte | `u8` | `0x01` = Trial (7d), `0x02` = 1 Month (30d), `0x03` = 1 Year (365d), `0xFF` = Lifetime |

### 2.2. Master Authority Signature (`Bytes 49..113`, 64 Bytes)

| Offset | Field | Size | Type | Description |
| :--- | :--- | :--- | :--- | :--- |
| `49..113` | `MASTER_SIG` | 64 Bytes | `[u8; 64]` | Ed25519 Signature created by `master.key` over `Bytes 0..49` prefixed with context `b"ZONLICv1"` |

$$\text{MasterSignInput (57 Bytes)} = \text{b"ZONLICv1" (8B)} \parallel \text{LicensePayload (49B)}$$

---

## 3. Binary Wire Protocol Specification (Byte-by-Byte)

Codex Agent MUST implement the exact binary framing below before piping `zINSTREAM` data. All multi-byte integers are **Big-Endian**.

### Step 1: Server Challenge (`Server -> Client`, 44 Bytes Fixed)

Immediately upon accepting a TCP connection on port `3311`, `zon-gateway` generates and sends **44 bytes**:

| Offset | Field | Size | Description |
| :--- | :--- | :--- | :--- |
| `0..4` | `MAGIC` | 4 Bytes | Fixed ASCII bytes: `b"ZONA"` (`0x5A, 0x4F, 0x4E, 0x41`) |
| `4..12` | `SERVER_TIME` | 8 Bytes | `u64` Big-Endian current Server Unix Epoch timestamp (seconds) |
| `12..44` | `NONCE` | 32 Bytes | Cryptographically random per-connection nonce (`rand::rngs::OsRng`) |

### Step 2: Client Auth & License Packet (`Client -> Server`, 177 Bytes Fixed)

The client reads the 44-byte challenge, verifies `MAGIC == b"ZONA"`, and constructs a **157-byte session transcript**:

$$\text{SessionTranscript (157 Bytes)} = \text{Challenge (44B)} \parallel \text{LicenseToken (113B)}$$

The client signs this 157-byte transcript using its device private key (`id_ed25519.key`) and sends **177 bytes**:

| Offset | Field | Size | Description |
| :--- | :--- | :--- | :--- |
| `0..113` | `LICENSE_TOKEN` | 113 Bytes | Raw 113-byte License Token (`Payload [49B] + MasterSig [64B]`) |
| `113..177` | `CLIENT_SIG` | 64 Bytes | Client Ed25519 signature over the 157-byte `SessionTranscript` |

> **Unlicensed / Server-Registry Fallback Mode**: If a client does not have a `zon.lic` file yet, it can still authenticate if its public key is manually registered in the server's `/etc/zonendpoint/clients.toml`. In this mode, the client fills `LICENSE_TOKEN` with its `CLIENT_PUBKEY (32B)` followed by `81 zero bytes (0x00)`, and signs the 157-byte transcript normally.

### Step 3: Server Verification & Auth Result (`Server -> Client`, 9 Bytes Fixed)

`zon-gateway` performs verification in strict order and replies with **9 bytes**:

| Offset | Field | Size | Description |
| :--- | :--- | :--- | :--- |
| `0..1` | `STATUS_CODE` | 1 Byte | Auth & License Status (see table below) |
| `1..9` | `EFFECTIVE_EXPIRES_AT` | 8 Bytes | `u64` Big-Endian Unix timestamp of effective expiration (so Client UI can sync exact server expiry) |

#### Status Codes (`STATUS_CODE`):
* `0x01` (`AUTH_OK`): Authentication & License valid. Connection immediately transitions to raw ClamAV passthrough (`zINSTREAM\0`, `zPING\0`).
* `0x02` (`LICENSE_EXPIRED`): Client identity is valid, but `SERVER_TIME > EFFECTIVE_EXPIRES_AT`. Server closes socket. Client UI displays *"License Expired — Please renew"*.
* `0x03` (`KEY_REVOKED`): Client key is marked `revoked = true` in `clients.toml`. Server closes socket.
* `0x04` (`UNAUTHORIZED_KEY`): Neither a valid Master-signed `zon.lic` nor an entry in `clients.toml` was found.
* `0x00` (`INVALID_SIGNATURE`): Cryptographic verification of `CLIENT_SIG` failed.

---

## 4. Server-Side Configuration & Override (`/etc/zonendpoint/clients.toml`)

`zon-gateway` supports both **Stateless Licensing** (via `MASTER_PUBLIC_KEY` verifying `zon.lic`) and **Stateful Server Overrides** via `clients.toml`. This allows the admin to extend a client's license or revoke a compromised laptop on the server without touching the client machine.

```toml
# /etc/zonendpoint/clients.toml
master_public_key = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"

[[clients]]
client_id = "laptop-win11-anh-zon"
public_key = "8f4b2e19c0a3d4f56789abcdef0123456789abcdef0123456789abcdef012345"
plan = "LIFETIME"
expires_at = 18446744073709551615 # u64::MAX (Lifetime)
revoked = false

[[clients]]
client_id = "pc-accounting-01"
public_key = "3a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef"
plan = "1_MONTH"
expires_at = 1798761600 # Unix epoch override (e.g. renewed on server for +30 days)
revoked = false
```

### Priority Resolution Logic in `zon-gateway`:
1. If `CLIENT_PUBKEY` exists in `clients.toml` and `revoked == true` $\rightarrow$ Return `0x03 (KEY_REVOKED)`.
2. Extract `token_expires_at` if `MASTER_SIG` is valid against `master_public_key`.
3. Extract `config_expires_at` if `CLIENT_PUBKEY` exists in `clients.toml`.
4. `EFFECTIVE_EXPIRES_AT = max(token_expires_at, config_expires_at)`. (Allows seamless server-side renewal!).
5. If neither is present $\rightarrow$ Return `0x04 (UNAUTHORIZED_KEY)`.
6. Verify `CLIENT_SIG` using `verifying_key.verify_strict(&session_transcript, &client_sig)`. If invalid $\rightarrow$ Return `0x00`.
7. If `server_now > EFFECTIVE_EXPIRES_AT` $\rightarrow$ Return `0x02 (LICENSE_EXPIRED)`.
8. Return `0x01 (AUTH_OK)` + `EFFECTIVE_EXPIRES_AT` and pipe stream to `clamd`.

---

## 5. Critical Technical Pitfalls & Edge Cases for Codex Agent

1. **Always Use `verify_strict()`**:
   * Both Master Signature verification and Client Session Signature verification MUST use `verifying_key.verify_strict(&msg, &sig)` from `ed25519-dalek` v2.x to prevent small-subgroup key malleability.
2. **Never Trust Client OS Clock for Enforcement**:
   * The Windows 11 client may display remaining days in the React UI by comparing `zon.lic` against local time or the last known `SERVER_TIME`, but actual scan authorization MUST be enforced by `zon-gateway` using Ubuntu Server's UTC clock.
3. **Anti-Replay Protection**:
   * `zon-gateway` generates a fresh 32-byte `OsRng` nonce per TCP connection and enforces a **5-second Handshake Timeout** (`tokio::time::timeout(Duration::from_secs(5), ...)`).
4. **TCP Partial Reads (`read_exact`)**:
   * Use `AsyncReadExt::read_exact` for the 44-byte Challenge, 177-byte Auth Packet, and 9-byte Auth Result.
5. **Hot-Reload `clients.toml`**:
   * Wrap the parsed `clients.toml` struct in `Arc<RwLock<ClientRegistry>>` and reload it automatically when the file's `mtime` changes or on `SIGHUP`, so renewing a client license takes effect immediately without restarting `zon-gateway`.

---

## 6. Workspace Structure & CLI Key/License Generator (`zon-keygen`)

Create a Cargo workspace or two standalone binary crates alongside `src-tauri`:

1. **`src-tauri` (Windows 11 Client)**:
   * Auto-generates `%LOCALAPPDATA%\Zonendpoint\keys\id_ed25519.key` on first launch.
   * Exposes Tauri commands:
     * `get_license_status() -> LicenseInfo` (Returns `client_pubkey_hex`, `plan_name`, `expires_at`, `days_remaining`, `status`).
     * `import_license_token(base64_token: String) -> Result<LicenseInfo, String>` (Validates format, saves `zon.lic`, updates UI).
2. **`zon-gateway` (Ubuntu Server Proxy + CLI License Generator)**:
   * Subcommand `zon-gateway serve --listen 0.0.0.0:3311 --clamd 127.0.0.1:3310 --config /etc/zonendpoint/clients.toml`
   * Subcommand `zon-gateway keygen-master --out-dir ./keys` (Generates `master.key` and `master.pub`).
   * Subcommand `zon-gateway issue-license --master-key ./keys/master.key --client-pubkey <64-HEX> --days 30` (Outputs Base64 `zon.lic` string for 1 month, 365 days for 1 year, or `--lifetime`).

---

## 7. Reference Implementation Snippets for Codex

### 7.1. License Token Issuing & Verification (`zon-gateway/src/license.rs`)

```rust
use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use std::time::{SystemTime, UNIX_EPOCH};

pub const LIC_CONTEXT: &[u8; 8] = b"ZONLICv1";
pub const TOKEN_LEN: usize = 113;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanCode {
    Trial = 0x01,
    OneMonth = 0x02,
    OneYear = 0x03,
    Lifetime = 0xFF,
}

impl PlanCode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0x01 => PlanCode::Trial,
            0x02 => PlanCode::OneMonth,
            0x03 => PlanCode::OneYear,
            _ => PlanCode::Lifetime,
        }
    }
}

pub fn issue_license_token(
    master_signing_key: &SigningKey,
    client_pubkey: &[u8; 32],
    days: Option<u64>,
    plan: PlanCode,
) -> Result<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let expires_at = match days {
        Some(d) => now.saturating_add(d * 86_400),
        None => u64::MAX, // Lifetime
    };

    let mut payload = [0u8; 49];
    payload[0..32].copy_from_slice(client_pubkey);
    payload[32..40].copy_from_slice(&now.to_be_bytes());
    payload[40..48].copy_from_slice(&expires_at.to_be_bytes());
    payload[48] = plan as u8;

    let mut sign_input = [0u8; 57];
    sign_input[0..8].copy_from_slice(LIC_CONTEXT);
    sign_input[8..57].copy_from_slice(&payload);

    let master_sig = master_signing_key.sign(&sign_input);

    let mut token = [0u8; TOKEN_LEN];
    token[0..49].copy_from_slice(&payload);
    token[49..113].copy_from_slice(&master_sig.to_bytes());

    Ok(BASE64.encode(token))
}

pub fn verify_license_token(
    master_verifying_key: &VerifyingKey,
    token_bytes: &[u8; TOKEN_LEN],
) -> Result<([u8; 32], u64, PlanCode)> {
    let mut client_pubkey = [0u8; 32];
    client_pubkey.copy_from_slice(&token_bytes[0..32]);

    let expires_at = u64::from_be_bytes(token_bytes[40..48].try_into()?);
    let plan = PlanCode::from_u8(token_bytes[48]);

    let mut sign_input = [0u8; 57];
    sign_input[0..8].copy_from_slice(LIC_CONTEXT);
    sign_input[8..57].copy_from_slice(&token_bytes[0..49]);

    let sig = Signature::from_slice(&token_bytes[49..113])?;
    master_verifying_key.verify_strict(&sign_input, &sig)?;

    Ok((client_pubkey, expires_at, plan))
}
```

---

## 8. Prompt for Codex Agent in VS Code

Copy and paste the prompt below into Codex in VS Code:

> **Prompt**:  
> *"Read `SecConnect.md` in the project root. Implement the Ed25519 challenge-response authentication protocol and Time-Based Hybrid Licensing system (`ed25519-dalek` v2) exactly as specified:*  
> *1. Remove the old `SecureConnect.md` if present and use `SecConnect.md` as the single source of truth.*  
> *2. In `src-tauri`: Create `src/auth.rs` and `src/license.rs` to manage client keypair generation (`%LOCALAPPDATA%\Zonendpoint\keys\id_ed25519.key`), local license file (`zon.lic`), Tauri commands (`get_license_status`, `import_license_token`), and the 177-byte handshake in `scanner.rs` (handling status codes `0x01` OK, `0x02` LICENSE_EXPIRED, `0x03` KEY_REVOKED, `0x04` UNAUTHORIZED_KEY).*  
> *3. In the React UI (`src/`): Add a License & Security badge displaying the Client Public Key (with Copy button), License Status (Active / Expired / Unlicensed), remaining days countdown, and an input modal to paste/import a Base64 `zon.lic` key.*  
> *4. Create the `zon-gateway` crate with CLI subcommands (`serve`, `keygen-master`, `issue-license --days <N>`) supporting both stateless `zon.lic` verification and `/etc/zonendpoint/clients.toml` hot-reload overrides.*  
> *5. Write an integration test verifying: valid 30-day license scan succeeds, expired license returns `0x02 LICENSE_EXPIRED`, and server-side renewal in `clients.toml` extends an expired token."*