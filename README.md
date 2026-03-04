# envfort

[![CI](https://github.com/TT-AIXion/envfort/actions/workflows/ci.yml/badge.svg)](https://github.com/TT-AIXion/envfort/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/envfort.svg)](https://crates.io/crates/envfort)
[![License](https://img.shields.io/crates/l/envfort.svg)](https://github.com/TT-AIXion/envfort/blob/main/LICENSE)

`envfort` is a write-only secret vault for environment-variable injection.
Secrets are encrypted at rest, stored in SQLite, KEK-protected in OS keychain, and decrypted only for short-lived command execution.

## Features

- Write-only secret workflow (`set`, `list`, `run`, `rm`)
- Envelope encryption (`KEK` and per-secret `DEK`)
- AEAD encryption with `AES-256-GCM-SIV`
- Argon2id support and `kdf calibrate` command
- OS keychain integration via `keyring`
- Transactional KEK rotation (`rotate-kek`) with progress metadata
- Encrypted backup export/import (`export --encrypted`, `import --encrypted`)
- Local audit log with tail view (`audit --tail`)
- Profile isolation (`profile create|list|delete`)
- LLM isolation allowlist for `run` commands (`config.toml`, `--ci`, `--llm-safe`)

## Installation

### Cargo

```bash
cargo install envfort --locked
```

### Homebrew

```bash
brew tap tt-aixion/tap
brew install envfort
```

### GitHub Releases

Download prebuilt binaries from Releases page and place `envfort` in your `PATH`.

## Usage

### init

```bash
envfort init --profile default
```

Creates `~/.envfort/` (`0700`), initializes `vault.db`, and stores generated KEK in OS keychain.

### set

```bash
envfort set API_TOKEN --profile default
```

Prompts securely for secret value (no terminal echo).

### list

```bash
envfort list --profile default
```

Shows key names only.

### run

```bash
envfort run --profile default -- env | grep API_TOKEN
```

Decrypts in memory, injects env vars to child process, then zeroizes buffers.

Allowlist policy:

- `run` checks `~/.envfort/config.toml` `[run.allowlist]`.
- Unknown command in interactive mode: approval prompt, then auto-add with SHA-256 hash.
- CI mode (`--ci` or `ENVFORT_CI=1`): unknown command is rejected.
- `--llm-safe`: default injection mode becomes `fd` and allowlist is strictly enforced.

Example:

```toml
[run.allowlist]
commands = [
  { path = "/usr/bin/python3", hash = "sha256:..." },
  { path = "/usr/bin/node" }
]
```

### rm

```bash
envfort rm API_TOKEN --profile default
```

Deletes a secret after confirmation prompt.

### rotate-kek

```bash
envfort rotate-kek --profile default
```

Generates new KEK and re-wraps profile DEKs in a DB transaction.

### export

```bash
envfort export --encrypted --output ./backups/vault.db.enc --profile default
```

Exports encrypted backup only (no plaintext export mode).

### import

```bash
envfort import --encrypted ./backups/vault.db.enc --profile default
```

Imports encrypted backup and restores `vault.db`.

### audit

```bash
envfort audit --tail 50
```

Shows latest audit entries.

### kdf calibrate

```bash
envfort kdf calibrate --target-ms 150
```

Benchmarks Argon2id candidates and stores recommended params in metadata.

### profile

```bash
envfort profile create team-a
envfort profile list
envfort profile delete team-a
```

Manages profile namespace and profile-specific KEKs.

### ui

```bash
envfort ui
```

Starts localhost-only Web UI with token auth and DNS rebinding protections.

## Security Design Overview

- `KEK`: stored in keychain backend (OS keychain or passphrase-derived fallback)
- `DEK`: generated per secret, wrapped by KEK
- `AAD`: binds record metadata (`profile_id`, `key_id`, `record_version`, `aead_alg`, `kek_id`)
- `Storage`: encrypted BLOBs in SQLite (`PRAGMA secure_delete = ON`)
- `Runtime`: decryption only during `run`/maintenance paths, then memory zeroization

## Injection Modes

| Mode | Command shape | Status | Notes |
|---|---|---|---|
| Environment variables | `envfort run --inject env -- <cmd>` | Implemented | Default mode |
| Stdin payload | `envfort run --inject stdin -- <cmd>` | Implemented | For tools reading from stdin |
| File descriptor | `envfort run --inject fd -- <cmd>` | Implemented | Reduced env exposure |
| Unix socket | `envfort run --inject socket -- <cmd>` | Implemented | One-shot local channel |
| Temporary file | `envfort run --inject tmpfile -- <cmd>` | Implemented | `0600` file + cleanup |

## Threat Model

From `.codex/skills/design-spec.md` Section C:

1. "Write-only is a UI constraint, not a cryptographic property."
2. "Same-user attacker is out of scope (they can `envfort run env`)."
3. "LLM/AI agent isolation requires OS boundary (separate user/container)."
4. "Environment variables are visible via `/proc/<pid>/environ` during process lifetime."
5. "Browser-mediated attacks (DNS rebinding/CSRF) apply when Web UI is active."

## Contributing

1. Branch from `develop`.
2. Keep commits focused and use Conventional Commits.
3. Run local quality gates before PR:
   - `cargo fmt`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
4. Add/adjust tests for behavior changes.
5. Open PR with summary, security impact, and verification logs.

## License

MIT. See [LICENSE](./LICENSE).
