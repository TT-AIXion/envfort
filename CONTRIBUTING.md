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
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --locked
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

## Issue and PR Flow

1. Open a GitHub Issue first for bugs or feature requests (use issue templates).
2. For larger or breaking changes, align scope in the Issue before implementation.
3. Open a PR using `.github/pull_request_template.md`.
4. Link the Issue in the PR description (for example: `Closes #123`).

## Release Flow

1. Merge approved changes into `develop`.
2. Prepare the release commit (typically on `main` after branch promotion).
3. Create and push a version tag `v*` on the release commit.
4. GitHub Actions `release.yml` builds Linux/macOS/Windows binaries and publishes checksums to GitHub Releases.

## Automerge Label Process

Automerge is fail-safe and opt-in:

1. Add `automerge` label to a ready PR.
2. Ensure PR is not draft.
3. Wait for `CI` success.
4. `automerge.yml` enables GitHub auto-merge only when all policy checks pass.
5. If any condition is unmet, automerge is not enabled.

## Community and Security Expectations

- Follow `CODE_OF_CONDUCT.md` in all project interactions.
- Do not report vulnerabilities in public issues; use `SECURITY.md` reporting guidance.

## Security

- Do not include real secrets in tests, fixtures, logs, or docs.
- Follow `SECURITY.md` for vulnerability reporting.
