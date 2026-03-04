# envfort

Write-only environment variable vault for local development and automation.
Secrets are encrypted at rest, decrypted only for child-process injection, and never exposed via a `get`/`show` command.

## Features

- Rust single-binary CLI
- Envelope encryption (KEK/DEK)
- AES-256-GCM-SIV secret encryption
- Argon2id key derivation support for passphrase backend
- OS keychain integration (`keyring`) for KEK storage
- SQLite-backed encrypted secret store
- Write-only flow: `set`, `list`, `run`, `rm`

## Installation

Install from crates.io:

```bash
cargo install envfort --locked
```

Build from source:

```bash
git clone <your-fork-or-repo-url>
cd envfort
cargo build --release
```

## Usage

### Initialize vault

```bash
envfort init --profile default
```

Creates `~/.envfort/` (mode `0700`), initializes `vault.db`, generates a random 32-byte KEK, and stores the KEK in OS keychain.

### Set a secret

```bash
envfort set API_TOKEN --profile default
```

You will be prompted securely for the value (no echo).

### List keys

```bash
envfort list --profile default
```

Prints key names only (never plaintext values).

### Run a command with injected secrets

```bash
envfort run --profile default -- env | grep API_TOKEN
```

Loads secrets for the profile, decrypts in-memory, injects as environment variables, executes the child command, and zeroizes temporary plaintext buffers.

### Remove a secret

```bash
envfort rm API_TOKEN --profile default
```

Prompts for confirmation before deletion.

## Threat Model

This section reflects `skills/design-spec.md` Section C.

1. Write-only is a UI and API constraint, not a cryptographic guarantee by itself.
2. Same-user attacker is out of scope (a same-user process can run `envfort run env`).
3. LLM/agent isolation requires an OS boundary (separate user account, container, or VM).
4. Environment variables may be observable during process lifetime (for example via `/proc/<pid>/environ` on Linux).
5. If a Web UI mode is used, browser-mediated threats (for example DNS rebinding or CSRF) must be considered.

## Contributing

1. Fork and create a feature branch from `develop`.
2. Make focused changes with tests.
3. Run checks before opening a PR:
   - `cargo fmt`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
4. Use Conventional Commits (`feat:`, `fix:`, `chore:`, etc.).
5. Open a pull request with a clear summary, rationale, and test evidence.

## License

MIT. See [LICENSE](./LICENSE).
