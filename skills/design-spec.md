---
title: "envfort Design Specification"
version: "v3"
status: "Draft"
last_updated: "2026-03-04"
scope: "Full design spec — crypto, storage, CLI, threat model"
reviewers:
  - "GPT-5.2 Pro (Round 1 + 2)"
  - "Codex (gpt-5.3-codex)"
references:
  - "RFC 9106 (Argon2)"
  - "RFC 8452 (AES-GCM-SIV)"
  - "NIST SP 800-38D (GCM)"
  - "OWASP Secrets Management Cheat Sheet"
  - "Apple Platform Security (Keychain)"
  - "Microsoft Learn (DPAPI)"
  - "Freedesktop Secret Service Spec"
---

# envfort Design Specification v3

> Write-only environment variable vault. Inject secrets without exposing them.

## Summary

| Item | Decision |
|------|----------|
| Language | **Rust** — memory safety, `secrecy`/`zeroize`, single binary |
| AEAD | **AES-256-GCM-SIV** (fallback: AES-256-GCM, ChaCha20-Poly1305) |
| KDF | **Argon2id** — RFC 9106 2nd rec: m=64MiB, t=3, p=4 |
| Auth | **OS Keychain** default (macOS/Windows/Linux); passphrase fallback |
| Key Mgmt | **Envelope crypto (KEK/DEK)** — algorithm agility |
| Write-only | No `get`/`show` API; `envfort run` inject-only; no break-glass |
| Injection | env (default) + stdin / FD / socket / tmpfile |
| Storage | **SQLite + encrypted BLOB** (secure_delete ON) |
| Distribution | GitHub Releases + Homebrew Tap + `cargo install` |
| Web UI | `envfort ui` — axum embedded SPA, 127.0.0.1 only |

---

## A. Tech Stack — Rust

### Why Rust
1. Ownership/lifetime → secret scope control, drop-time zeroize
2. `secrecy` + `zeroize` → no accidental Debug/log leak
3. RustCrypto ecosystem (`argon2`, `aes-gcm-siv`, `chacha20poly1305`, `ring`, `keyring`)
4. Single binary, minimal supply-chain surface
5. `keyring` crate for cross-platform OS keychain

### Core Dependencies
- CLI: `clap`
- Secret types: `secrecy`, `zeroize`
- KDF: `argon2`
- AEAD: `aes-gcm-siv` (fallback `aes-gcm`, `chacha20poly1305`)
- RNG: `rand_core` / `getrandom`
- OS Keychain: `keyring`
- DB: `rusqlite`
- Serialization: `serde`, `serde_json` (config only)
- Web UI: `axum`, `tokio` (minimal)

---

## B. Cryptographic Design

### B-1. Auth: OS Keychain vs Master Password

**Default: OS Keychain.** Passphrase fallback for headless.

```
envfort init --auth keychain|passphrase|auto
```
- `auto` (default): macOS/Windows → keychain; Linux desktop → secret service; headless → passphrase

**OS-specific notes (GPT-5.2 Pro verified):**
- macOS: user presence (biometric/passcode) default ON. Keychain = SQLite + AES-GCM, ACL enforced.
- Windows: CRYPTPROTECT_LOCAL_MACHINE **never used** (all users can decrypt). User-scope DPAPI only.
- Linux Secret Service: attributes are NOT encrypted (spec-level). Never put secret info in key names/labels.

### B-2. AEAD Selection

**Primary: AES-256-GCM-SIV** (RFC 8452)
- Nonce misuse resistance — critical for CLI tools where implementation changes may cause nonce accidents
- Note: RFC 8452 is Informational, not FIPS. Document AES-256-GCM alternative for FIPS users.

### B-3. KDF: Argon2id

**Params (fixed):** `m=65536 (64MiB), t=3, p=4, salt≥16B`
- Matches RFC 9106 SECOND RECOMMENDED exactly.
- `--kdf-profile hardened`: 2GiB, t=1, p=4 (RFC FIRST RECOMMENDED)

**TODO (P1):** `envfort kdf calibrate` — auto-tune m/t for target latency on current hardware (prevents DoS on weak machines, underspec on strong ones).

### B-4. Envelope Encryption (KEK/DEK)

```
┌─────────────┐     ┌─────────────┐
│  OS Keychain │     │  Passphrase │
│  or Password │     │  + Argon2id │
└──────┬──────┘     └──────┬──────┘
       │                    │
       └────────┬───────────┘
                ▼
           ┌────────┐
           │  KEK   │  (Key Encryption Key)
           └────┬───┘
                │ wrap/unwrap
                ▼
           ┌────────┐
           │  DEK   │  (Data Encryption Key, per-secret)
           └────┬───┘
                │ encrypt/decrypt
                ▼
           ┌────────┐
           │ Value  │  (secret value)
           └────────┘
```

**AAD binding (P0 — GPT-5.2 Pro R2):**
Both `encrypted_dek` and `ciphertext` MUST include AAD with:
- `profile_id`, `key_id` (immutable UUID), `record_version`, `aead_alg`, `kek_id`
- Prevents record swap/DoS attacks. Serialize with `bincode` (deterministic, no JSON key ordering issues).

**Nonce strategy:** Always CSPRNG-generated per encryption. GCM-SIV misuse resistance is insurance, not an excuse for lazy nonce management.

**Algorithm agility:**
- Allowlist-only: reject unknown/weak `aead_alg`/`kdf_alg`
- Write = latest only; read = all supported versions
- `--min-version` for org policy enforcement

**KEK rotation (P0):**
- `kek_id` column mandatory in secrets table
- `envfort rotate-kek`: new KEK → DB txn re-wrap all DEKs → delete old KEK
- Must be resumable (crash-safe): track progress in `meta` table

---

## C. Write-Only Design

### Core Principle
No `get` / `show` / `export` (plaintext). Decryption only during `envfort run`, in-memory, short-lived.

### What's Provided
- `envfort list` — key names only
- `envfort run` — decrypt → inject → zeroize
- `envfort export --encrypted` — encrypted backup (never plaintext)
- `envfort rotate-kek` — KEK rotation (DEK re-wrap, no plaintext)
- `envfort migrate` — re-encrypt to new vault

### What's NOT Provided
- ~~`envfort get KEY`~~ — removed by design
- ~~break-glass / `--reveal`~~ — removed (GPT-5.2 Pro: attack surface > utility)
- ~~dangerous command detection~~ — removed (GPT-5.2 Pro: not exhaustive, false sense of security)

### Threat Model (README mandatory — P0)
Document explicitly:
1. "Write-only is a UI constraint, not a cryptographic property"
2. "Same-user attacker is out of scope (they can `envfort run env`)"
3. "LLM/AI agent isolation requires OS boundary (separate user/container)"
4. "Environment variables are visible via `/proc/<pid>/environ` during process lifetime (OWASP)"
5. "Browser-mediated attacks (DNS rebinding/CSRF) apply when Web UI is active" (v3 addition)

---

## D. Storage

### Layout
```
~/.envfort/
  config.toml          # non-secret config (0600)
  vault.db             # SQLite (encrypted BLOBs)
  vault.db-wal         # WAL (if enabled)
  vault.db-shm         # SHM (if enabled)
  audit.log            # local audit log (never contains values)
  backups/
    vault-YYYYMMDD-HHMMSS.db.enc
  run/                 # tmpfile injection (0700, XDG_RUNTIME_DIR preferred)
```

### Schema v3

```sql
CREATE TABLE secrets(
  id         INTEGER PRIMARY KEY,
  profile    TEXT    NOT NULL,
  key_name   TEXT    NOT NULL,
  key_id     TEXT    NOT NULL,       -- immutable UUID (for AAD binding)
  kek_id     TEXT    NOT NULL,       -- which KEK wrapped this DEK (P0: rotation)
  version    INTEGER NOT NULL,       -- format generation
  aead_alg   TEXT    NOT NULL,       -- "AES-256-GCM-SIV"
  nonce      BLOB    NOT NULL,
  encrypted_dek BLOB NOT NULL,       -- DEK wrapped by KEK
  ciphertext BLOB    NOT NULL,       -- value encrypted by DEK
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(profile, key_name)
);

CREATE TABLE meta(
  k TEXT PRIMARY KEY,
  v TEXT
);
-- Stores: kek_id, kek_generation, kdf_salt, kdf_params, rotation_state

CREATE TABLE audit_log(
  id        INTEGER PRIMARY KEY,
  timestamp INTEGER NOT NULL,
  action    TEXT    NOT NULL,    -- "set","delete","run","rotate-kek","export"
  key_name  TEXT,
  profile   TEXT,
  detail    TEXT                 -- NEVER contains secret values
);

PRAGMA secure_delete = ON;
```

**SQLite notes (GPT-5.2 Pro):**
- `secure_delete` zeroes on delete/update, but has scope limitations
- WAL may retain deleted data under specific conditions (encrypted values = OK, key names may linger)
- If key name secrecy needed: encrypt `key_name` too (trade-off: no search by name)
- Document these limitations in README

### Profile Resolution
1. `--profile` flag
2. `ENVFORT_PROFILE` env var
3. `default_profile` in config.toml

### File Permissions
- Directories: `0700`
- Files: `0600`

---

## E. Distribution

### Phase 1 (Launch)
- **GitHub Releases**: signed binaries (macOS/Linux/Windows) + `checksums.txt` + `.sig` + provenance attestation
- **Homebrew Tap**: no review wait
- **`cargo install envfort --locked`**

### Phase 2 (Growth)
- Homebrew Core submission (requires notability/stability)

### Targets
| OS | Arch |
|----|------|
| macOS | aarch64, x86_64 |
| Linux | x86_64, aarch64 (glibc + musl) |
| Windows | x86_64 |

### CI/CD
- Matrix build
- SBOM generation
- Signing/attestation automation
- Verification instructions in README

---

## F. Web UI (`envfort ui`)

### Overview
Embedded local web server for browser-based secret management. Write-only maintained.

### Commands
```bash
envfort ui                    # default (auto browser open)
envfort ui --no-open          # URL only
envfort ui --port 8080        # fixed port
envfort ui --timeout 60       # session timeout (minutes)
```

### Security (GPT-5.2 Pro R2 hardened)

**Network:**
- `127.0.0.1` bind only (no `0.0.0.0`)
- OS-assigned port (`TcpListener::bind("127.0.0.1:0")`)

**DNS Rebinding Defense (P0):**
- Host header validation: only `127.0.0.1:<port>` / `localhost:<port>` / `[::1]:<port>`
- Origin header validation on state-changing requests (POST/PUT/DELETE)
- CORS disabled (no `Access-Control-Allow-Origin`)

**Auth (token NOT in URL — GPT-5.2 Pro R2):**
- ~~URL query token~~ → replaced with file-based token exchange:
  1. Generate CSPRNG token → write to `~/.envfort/ui-token` (0600)
  2. Browser reads token via initial page JS → sends as `Authorization: Bearer <token>`
  3. Token file deleted immediately after first read
- Or: CLI prints token, user pastes into auth prompt (manual mode)

**Session:**
- Authorization header-based (no cookies — avoids Secure flag issues on localhost HTTP)
- 30-min inactivity timeout → auto-shutdown

**Headers (fixed):**
```http
Cache-Control: no-store
Pragma: no-cache
Referrer-Policy: no-referrer
X-Content-Type-Options: nosniff
Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; frame-ancestors 'none'; form-action 'self'
```

**Input Protection:**
- `type="password"` for secret value fields
- `autocomplete="off" spellcheck="false" autocapitalize="off" autocorrect="off"`

### UI Features
| Feature | Description |
|---------|-------------|
| Set key/value | Form → encrypt → save (value hidden after save) |
| List keys | Key names only, values always masked |
| Delete key | Confirmation dialog |
| Profile mgmt | Create / switch / delete |
| Settings | Auth method, KDF params |
| Audit log | Access history (no values) |
| .env import | Bulk read → encrypt → suggest secure delete of source |

### Tech
- `axum` (lightweight async)
- SPA embedded via `include_str!`/`include_bytes!` (single binary, no extra files)
- Browser launch: `open` (macOS) / `xdg-open` (Linux) / `start` (Windows)

---

## G. LLM Isolation (GPT-5.2 Pro R2 — P0)

### Problem
Removing dangerous command detection leaves no guard against `envfort run -- env` by an AI agent.

### Solution: Allowlist + Approval

**Default-deny mode:**
- `envfort run` requires command to be in allowlist (config.toml)
- First use of new command → interactive approval prompt (stored in allowlist)
- CI mode (`--ci` / `ENVFORT_CI=1`): skip approval, trust allowlist only

**Allowlist format (config.toml):**
```toml
[run.allowlist]
commands = [
  { path = "/usr/bin/node", hash = "sha256:abc123..." },
  { path = "/usr/local/bin/python3", hash = "sha256:def456..." },
]
```

**LLM-safe injection default:**
- When `ENVFORT_LLM_MODE=1` or `--llm-safe`: default injection switches to `--inject=fd`
- Environment variable injection disabled unless explicitly `--inject=env`

---

## H. Memory Protection

- `secrecy::SecretString` — no Debug, no Display, no String conversion
- `zeroize` on drop — all secret buffers
- `setrlimit(RLIMIT_CORE, 0)` — disable core dumps (parent + verify child inheritance)
- `mlock` — best-effort (document failure is normal on some systems)
- Test: verify child process inherits RLIMIT_CORE=0 (GPT-5.2 Pro R2 finding)

---

## CLI Command Reference

```
envfort init [--auth keychain|passphrase|auto]
envfort set KEY
envfort list [--profile <name>]
envfort run [--inject env|stdin|fd|socket|tmpfile] [--llm-safe] -- <cmd> [args...]
envfort rm KEY
envfort rotate-kek
envfort export --encrypted [--output <path>]
envfort import --encrypted <path>
envfort migrate --target <new-vault-path>
envfort profile create|list|delete <name>
envfort ui [--no-open] [--port <n>] [--timeout <min>]
envfort kdf calibrate [--target-ms <n>]
envfort audit [--tail <n>]
```

---

## References
- RFC 9106 (Argon2): https://www.rfc-editor.org/rfc/rfc9106
- RFC 8452 (AES-GCM-SIV): https://www.rfc-editor.org/rfc/rfc8452
- NIST SP 800-38D (GCM): https://csrc.nist.gov/pubs/sp/800/38/d/final
- RFC 8439 (ChaCha20-Poly1305): https://www.rfc-editor.org/rfc/rfc8439
- OWASP Password Storage: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
- OWASP Secrets Management: https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html
- Apple Platform Security: https://support.apple.com/guide/security
- Microsoft DPAPI: https://learn.microsoft.com/en-us/windows/win32/seccrypto/crypt-protect-data
- Freedesktop Secret Service: https://specifications.freedesktop.org/secret-service/latest/
- SQLite secure_delete: https://sqlite.org/pragma.html#pragma_secure_delete
- `secrecy` crate: https://docs.rs/secrecy
- `zeroize` crate: https://docs.rs/zeroize
