# RELEASE READINESS AUDIT 2026-03-08

## Executive summary

`develop` は `origin/develop` と同期済みですが、`origin/main` より 17 commits 先行しており、release 導線そのものもまだ `main` に昇格していません。現状は「次の GitHub Release / crates.io 公開 / Homebrew Tap 準備を安全に進める ready」ではありません。

現時点の **binding な blocker は 6 件** です。

1. `Cargo.lock` と `--locked` 前提の不整合
2. `main` 未昇格の release flow
3. crates.io tarball の package surface 未整理
4. Homebrew tap / formula 未実体化
5. 公開配布物の install / artifact 契約の曖昧さ
6. README の install 導線先行

一方で、`skills/design-spec.md` にある multi-arch / signing / provenance / SBOM は、将来の distribution hardening としては妥当ですが、現時点では **draft design spec との差分メモ** として扱うのが安全です。

## Current state snapshot

- branch 状態:
  - `develop` = `1dade34` (`origin/develop` と差分 0)
  - `origin/main...develop` = `0 17`
- `main` から未昇格の release assets/docs:
  - `.github/workflows/release.yml`
  - `RELEASE_HANDOFF_CHECKLIST.md`
  - `CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md`
  - `HOMEBREW_TAP_BOOTSTRAP_PLAN_2026-03-06.md`
- current release workflow:
  - trigger は `push.tags=v*` と `workflow_dispatch(tag)` のみ (`.github/workflows/release.yml:3-17`)
  - build は `ubuntu-latest` / `macos-latest` / `windows-latest` 各 1 本 (`.github/workflows/release.yml:20-35`)
  - artifact は raw binary `envfort-linux`, `envfort-macos`, `envfort-windows.exe` と各 `.sha256` (`.github/workflows/release.yml:49-63`, `.github/workflows/release.yml:95-99`)
- crates metadata は基本項目あり。`include` / `exclude` は未定義 (`Cargo.toml:1-12`)
- README はすでに `cargo install` / `brew install` / GitHub Releases 導線を掲示 (`README.md:23-40`)
- Homebrew formula は repo 内に存在しない。tap は外部 repo 前提の plan のみ (`RELEASE_HANDOFF_CHECKLIST.md:63-87`, `HOMEBREW_TAP_BOOTSTRAP_PLAN_2026-03-06.md:5-7`, `HOMEBREW_TAP_BOOTSTRAP_PLAN_2026-03-06.md:38-65`)

## Findings

### 1. Critical — `Cargo.lock` が release/CI 前提と不整合

Evidence: `.gitignore` は `Cargo.lock` を ignore (`.gitignore:1-4`)。`git ls-files Cargo.lock` は空、`git check-ignore -v Cargo.lock` は ignore 扱いです。一方で CI は `cargo build --release --locked` を実行 (`.github/workflows/ci.yml:46-47`)、release workflow も `cargo build --release --locked` を実行 (`.github/workflows/release.yml:46-47`)。crates.io runbook も「`Cargo.lock` exists and is committed」を前提にしています (`CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md:14-23`)。

Impact: clean checkout の GitHub Actions / release build / publish dry-run 前提が崩れます。repo source of truth 上、`--locked` 系の release path は未解決です。

Assessment: main merge 前 blocker。

### 2. Critical — release flow はまだ `main` 未到達

Evidence: `origin/main...develop` は `0 17`。`release.yml` と handoff/runbook/tap plan は `origin/main` に存在しない create-mode 差分です。release workflow は tag checkout 固定 (`.github/workflows/release.yml:37-41`) で、`workflow_dispatch` も既存 tag 指定前提です (`.github/workflows/release.yml:7-17`)。README も `workflow_dispatch` を「existing release tag の manual rerun」と説明しています (`README.md:213-217`)。

Impact: `develop` のまま tag を切ると、`main` 起点の release handoff 前提とズレます。また、repo-local evidence ベースでは non-publishing rehearsal path も明示されていません。

Assessment: main merge 前 blocker。

### 3. High — crates.io tarball の package surface が未整理

Evidence: `Cargo.toml` に `include` / `exclude` がありません (`Cargo.toml:1-12`)。`git ls-files` には runtime source 以外に `BINCODE3_*`, `CODEX_REVIEW_*`, `DEPENDABOT_*`, `HOMEBREW_TAP_BOOTSTRAP_PLAN_2026-03-06.md`, `.github/**`, `.claude/skills/**`, `.codex/skills/**`, `.cursor/skills/**`, `skills/**` が含まれます。runbook 自体も `include` / `exclude` 未定義を注意点として明記しています (`CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md:20-23`, `CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md:31-42`)。

Impact: crates.io 公開 tarball に運用文書や skill 文書が広く含まれ、package bloat / published surface noise が大きくなります。

Assessment: crates.io 公開前 blocker。

### 4. High — Homebrew Tap はまだ「準備計画」止まり

Evidence: README はすでに `brew tap tt-aixion/tap` / `brew install envfort` を掲示しています (`README.md:31-36`)。しかし repo 内に `Formula/envfort.rb` は存在せず、handoff/checklist も「この repo には formula/tap automation はない」と明記しています (`RELEASE_HANDOFF_CHECKLIST.md:63-87`)。tap plan も外部 repo `TT-AIXion/homebrew-tap` 前提の bootstrap 手順書のみです (`HOMEBREW_TAP_BOOTSTRAP_PLAN_2026-03-06.md:11-21`, `HOMEBREW_TAP_BOOTSTRAP_PLAN_2026-03-06.md:38-65`)。

Impact: source of truth 上、Homebrew install path は未開通です。tap repo 実在、formula 実装、`brew audit/install/test` 成功が未確認のまま README だけ先行しています。

Assessment: Homebrew Tap 準備前 blocker。

### 5. High — 公開配布物の artifact / install 契約がまだ曖昧

Evidence: release workflow の artifact 名は `envfort-linux`, `envfort-macos`, `envfort-windows.exe` で arch suffix がありません (`.github/workflows/release.yml:58-63`, `.github/workflows/release.yml:95-99`)。README の GitHub Releases 節も「PATH に置く」説明が中心で、arch 対応、`chmod +x`、checksum verify、Linux keychain 前提などを十分に案内していません (`README.md:38-40`)。一方 handoff checklist では post-release verification で `chmod +x` を要求しています (`RELEASE_HANDOFF_CHECKLIST.md:96-100`)。

Impact: 利用者視点では、どの binary を選ぶべきか、どう検証するか、どう導入するかの契約がまだ弱い状態です。

Assessment: GitHub Release 前 blocker。

### 6. High — README の install 導線が実装状態より先行している

Evidence: README は `cargo install envfort --locked`、`brew tap tt-aixion/tap` / `brew install envfort`、GitHub Releases を並列で掲示しています (`README.md:23-40`)。しかし repo source of truth では `Cargo.lock` 方針未解決、Homebrew tap 未実体化、release handoff も `main` 未昇格です。

Impact: 利用者向けの install 導線が、実際に安全に提供できる状態より先に見えています。公開後の混乱や support 負荷を増やしやすいです。

Assessment: release 前 blocker。

## Draft design spec gaps (non-blocking yet)

`skills/design-spec.md` は `status: "Draft"` です (`skills/design-spec.md:1-5`)。したがって、以下は現時点では hard blocker ではなく、**将来の distribution hardening 候補** として扱うのが安全です。

- multi-arch distribution target の拡張 (`skills/design-spec.md:233-254`)
- signed binaries / `.sig`
- provenance attestation
- SBOM generation
- README verification instructions の拡充

これらは security bar を上げる上で価値がありますが、binding な release docs に昇格していない限り、「draft design spec との差分」として記録するのが適切です。

## Appendix — Dependabot release-workflow PRs

Dependabot `#4` / `#3` は re-review 後に 2026-03-10 merge 済みです。GitHub PR queue 上では release-workflow 系の open Dependabot PR はなく、未解決 PR は `bincode` `#7` の manual migration 側です (`DEPENDABOT_TRIAGE_2026-03-06.md`, `DEPENDABOT_MERGE_PLAYBOOK_2026-03-06.md`, `BINCODE3_MIGRATION_PLAN.md`)。ただし `#4` / `#3` merge 後の CI 確認 / release rehearsal 完了記録は別残作業として残り得ます。release-readiness の主線ではないため、本監査では appendix 扱いに留めます。

要点だけまとめると:

- `#4` を先、`#3` を後に扱う順序で 2026-03-10 に merge 完了
- 通常 CI だけでは release workflow 変更の安全性は閉じきれない、という整理自体は有効
- open Dependabot PR は `#7` manual migration のみ
- 別途 `#4` / `#3` post-merge verification 記録は未了なら残作業

## Release blockers vs nice-to-have

### Blockers

1. `Cargo.lock` 方針を fix する。
2. `develop` → `main` promotion を完了し、release candidate commit を freeze する。
3. `Cargo.toml` の `include` / `exclude` を定義し、crate tarball surface を絞る。
4. Homebrew tap repo と `Formula/envfort.rb` を実体化し、`brew audit/install/test` を通す。
5. release artifact / install 契約を整理する。少なくとも asset naming、導入手順、checksum verify、platform caveat を明文化する。
6. README の install 導線を「実際に使えるものだけ」に合わせる。

### Nice-to-have

1. production release と分離した dedicated rehearsal workflow / draft release mode。
2. `checksums.txt` 集約。
3. stale な Dependabot review docs の整理。
4. draft design spec にある signing / provenance / SBOM / multi-arch の段階導入。
5. metadata polish。`homepage` 見直し、threat model 記述の top-level 化。

## Recommended next actions

1. `Cargo.lock` を最優先で整理する。`--locked` を維持するなら commit 必須。
2. `develop` の current HEAD を release candidate として固定し、`main` へ promotion する。tag cut はその後。
3. `Cargo.toml` の `include` / `exclude` を追加し、crate tarball surface を絞る。
4. Homebrew tap を実体化する。`TT-AIXion/homebrew-tap`、`Formula/envfort.rb`、asset URL/sha256、`brew audit/install/test` 成果物まで揃えてから README を確定する。
5. release artifact / install guide を binding docs と README に反映する。asset naming、導入手順、verify 手順、platform caveat を揃える。
6. README の install 導線を実装済み経路だけに絞る。
7. その後、必要なら dedicated rehearsal path、signing / provenance / SBOM、multi-arch を順次導入する。
