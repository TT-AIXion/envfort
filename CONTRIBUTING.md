# Contributing to envfort

Thanks for contributing.

## Development Setup

1. Install stable Rust toolchain.
2. Clone repo and switch to `develop` branch.
3. Build once:

```bash
cargo build
```

4. Optional local install for manual checks:

```bash
cargo install --path . --locked
```

## Local Quality Gates

Run before opening PR:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## Testing Guidance

- Add or update tests for behavior changes.
- Prefer regression tests for bug fixes.
- Keep tests deterministic and isolated (`HOME` temp dir for integration tests).

## Pull Request Guidelines

1. Branch from `develop`.
2. Use Conventional Commits (`feat`, `fix`, `test`, `docs`, `ci`, etc.).
3. Keep PR focused and reviewable.
4. Include:
   - what changed
   - why it changed
   - security impact (if any)
   - verification commands + results
5. Ensure CI is green on Linux and macOS.

## Security

- Do not include real secrets in tests, fixtures, logs, or docs.
- Follow `SECURITY.md` for vulnerability reporting.
