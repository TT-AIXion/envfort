---
title: "Security Review Round 2 — GPT-5.2 Pro"
version: "v1"
status: "Completed"
last_updated: "2026-03-04"
scope: "Second review after v2 modifications"
reviewer: "GPT-5.2 Pro (2026-03-04 23:07 JST)"
references:
  - "RFC 9106 (Argon2)"
  - "RFC 8452 (AES-GCM-SIV)"
  - "NIST SP 800-38D (GCM)"
  - "OWASP Cheat Sheet Series (Secrets/CSRF/Session/CSP/Headers/LLM/Agent)"
  - "SQLite (secure_delete, WAL)"
  - "Apple Platform Security (Keychain)"
  - "Microsoft Learn (DPAPI)"
  - "Freedesktop Secret Service Spec"
---

# envfort Security Review — Round 2 (GPT-5.2 Pro)

## Review Date
2026-03-04 23:07 JST

## Verdict
> v2で「break-glass削除」「エンベロープ暗号導入」「KDF推奨値への整合」は設計上の筋が通っている。
> 一方で **Web UI追加** と **LLM隔離の実効性（allowlist/承認）** が、v2の新しい中心課題。

All 9 original modification points assessed as "妥当" (sound). New issues identified.

---

## New Findings

### P0 — Must Fix Before Release

1. **DNS rebinding defense for `envfort ui`**
   - 127.0.0.1 bind alone is insufficient
   - Need: Host header allowlist, Origin validation on state-changing requests
   - CORS must stay disabled (no `Access-Control-Allow-Origin: *`)
   - **Status: Fixed in v3** — added to Section F

2. **LLM/agent run allowlist + approval flow**
   - Replacing dangerous command detection requires an alternative guard
   - Need: default-deny run, allowlist (path+hash), interactive approval, CI mode
   - **Status: Fixed in v3** — added Section G

3. **AAD binding for envelope crypto**
   - Both `encrypted_dek` and `ciphertext` MUST include AAD
   - AAD fields: `profile_id`, `key_id`, `record_version`, `aead_alg`, `kek_id`
   - Prevents record swap attacks and DoS
   - Serialize with `bincode` (deterministic, no JSON ordering issues)
   - **Status: Fixed in v3** — added to Section B-4

4. **KEK rotation resumability**
   - `kek_id` column mandatory in secrets table
   - Rotation must be crash-safe (track progress in meta table)
   - **Status: Fixed in v3** — added `kek_id` to schema, rotation spec in B-4

### P1 — Reduces Operational Incidents

5. **Injection default reconsideration**
   - If LLM defense is primary goal, default should be FD/socket, not env
   - Compromise: `--llm-safe` flag / `ENVFORT_LLM_MODE=1` switches default
   - **Status: Fixed in v3** — Section G LLM-safe mode

6. **KDF calibration command**
   - Fixed params may be too slow on weak hardware or too fast on strong
   - `envfort kdf calibrate --target-ms 300` to auto-tune
   - **Status: Added to v3** — CLI reference + Section B-3 TODO

7. **SQLite operational policy documentation**
   - WAL behavior, VACUUM needs, secure_delete scope limits
   - Key names may persist in WAL remnants (values are encrypted = OK)
   - Option: encrypt key_name too (search trade-off)
   - **Status: Fixed in v3** — Section D SQLite notes

### P2 — If Resources Allow

8. **Audit log tamper resistance**
   - Hash chain for integrity verification
   - Size limits + rotation design
   - **Status: Noted for future**

9. **Key name secrecy option**
   - Encrypt `key_name` for users who need it
   - Trade-off: no search by name
   - **Status: Noted for future**

---

## Web UI Security Checklist (from this review)

| Defense | Required | Status |
|---------|----------|--------|
| Host header allowlist | P0 | ✅ v3 |
| Origin validation (state-changing) | P0 | ✅ v3 |
| CORS disabled | P0 | ✅ v3 |
| Token NOT in URL | P0 | ✅ v3 (file-based) |
| Authorization header (no cookies) | P1 | ✅ v3 |
| Security headers (CSP, no-store, etc.) | P1 | ✅ v3 |
| Input: type=password, autocomplete=off | P1 | ✅ v3 |
| frame-ancestors: none | P1 | ✅ v3 |

## Injection Path Security Ranking (GPT-5.2 Pro)

| Rank | Method | Leak Risk | Compatibility | Notes |
|------|--------|-----------|---------------|-------|
| 1 | FD (pipe/memfd) | Low | Low-Med | Receiver must support |
| 2 | UNIX domain socket | Low-Med | Low-Med | 0700 dir, peer cred check |
| 3 | stdin | Med | Med | Conflicts with interactive stdin |
| 4 | env var | High | High | /proc/pid/environ, logs, dumps |
| 5 | tmpfile | High | Med | Disk residue, backup/AV scan |

## Envelope Crypto Checklist

| Requirement | Status |
|-------------|--------|
| AAD with profile/key_id/version/alg/kek_id | ✅ v3 |
| Nonce: CSPRNG every time (even with GCM-SIV) | ✅ v3 |
| Algorithm allowlist (reject unknown) | ✅ v3 |
| Write = latest alg only | ✅ v3 |
| kek_id in secrets table | ✅ v3 |
| Rotation crash-safe (resumable) | ✅ v3 |
| KDF params in meta table (not per-record) | ✅ v3 |
