---
title: "Security Review Round 1 — GPT-5.2 Pro"
version: "v1"
status: "Completed"
last_updated: "2026-03-04"
scope: "Initial security review of envfort design v1"
reviewer: "GPT-5.2 Pro (2026-03-04 22:10 JST)"
references:
  - "RFC 9106 (Argon2)"
  - "RFC 8452 (AES-GCM-SIV)"
  - "OWASP Secrets Management Cheat Sheet"
  - "Apple Platform Security"
  - "Microsoft Learn (DPAPI)"
---

# envfort Security Review — Round 1 (GPT-5.2 Pro)

## Review Date
2026-03-04 22:10 JST

## Findings Summary

### P0 — Critical (Must fix before release)

1. **Threat model not documented**
   - Write-only is UI constraint, not cryptographic guarantee
   - Same-user attacker can `envfort run env` to see values
   - LLM isolation requires OS boundary (separate user/container)
   - Must document in README explicitly

2. **Injection path too narrow**
   - env-only injection is insufficient
   - OWASP warns env vars are visible via `/proc/<pid>/environ`
   - Need: stdin, FD, socket, tmpfile alternatives

3. **No envelope encryption**
   - Single master key = re-encrypt everything on rotation
   - Recommended: KEK/DEK separation for algorithm agility

4. **OS Keychain constraints undocumented**
   - macOS: user presence requirement not mentioned
   - Windows: CRYPTPROTECT_LOCAL_MACHINE risk not addressed
   - Linux: Secret Service attributes not encrypted (spec-level)

### P1 — Important

5. **Argon2id p=1 too low**
   - RFC 9106 recommends p=4 for SECOND RECOMMENDED
   - Fixed: p=4

6. **No algorithm agility**
   - Need `aead_alg`, `kdf_alg`, `version` in schema
   - Fixed: added to schema

7. **Memory protection incomplete**
   - Need core dump disable, mlock, String prohibition
   - Fixed: added to architecture

### P2 — Nice to Have

8. **break-glass is attack surface**
   - Provides false security (stdout/copy/screenshot still work)
   - Removed: replaced with encrypted export/KEK rotation

9. **Dangerous command detection not exhaustive**
   - Can't cover all commands that print env
   - Removed: effort redirected to allowlist/isolation

## Resolution
All P0/P1/P2 findings addressed in design-spec v2 → v3.
See `design-spec.md` for current state.
