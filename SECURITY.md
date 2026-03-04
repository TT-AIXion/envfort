# Security Policy

## Supported Versions

`envfort` is currently pre-1.0.
Security fixes are provided on the latest `develop` branch and the latest release tag.

## Reporting a Vulnerability

Do not open public GitHub issues for suspected vulnerabilities.

Report privately with:

- Email: `takuto.tanaka@aix-ion.com`
- Subject: `[envfort][security] <short summary>`
- Include:
  - Affected version/commit
  - Reproduction steps or proof-of-concept
  - Impact assessment (confidentiality/integrity/availability)
  - Suggested mitigation (optional)

## Response Process

- Initial acknowledgment target: within 3 business days
- Triage and severity assignment: as soon as reproducibility is confirmed
- Fix timeline: based on severity and exploitability
- Coordinated disclosure: after patch/release is available

## Scope Guidance

High-priority classes include:

- Secret disclosure in plaintext
- Key handling flaws (KEK/DEK misuse, weak KDF usage)
- Authentication/keychain bypass
- Backup export/import cryptographic bypass
- Command injection or privilege escalation in CLI execution paths

Out of scope examples:

- Social engineering
- Denial-of-service requiring privileged local access
- Issues only reproducible with unsupported toolchains
