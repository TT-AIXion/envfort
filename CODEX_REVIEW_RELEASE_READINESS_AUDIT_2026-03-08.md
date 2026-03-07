# Review: RELEASE_READINESS_AUDIT_2026-03-08

## Scope reviewed

- 対象: `RELEASE_READINESS_AUDIT_2026-03-08.md` のみ
- 照合元: `git` 状態、`.github/workflows/{ci,release}.yml`、`Cargo.toml`、`.gitignore`、`README.md`、`RELEASE_HANDOFF_CHECKLIST.md`、`CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md`、`HOMEBREW_TAP_BOOTSTRAP_PLAN_2026-03-06.md`、`skills/design-spec.md`
- 手法: local deterministic inspection のみ。repo 外の live state は source of truth に使っていない

## Findings

1. High — session/host 依存の事実が repo fact として書かれている。
   - 対象: `RELEASE_READINESS_AUDIT_2026-03-08.md:21`, `RELEASE_READINESS_AUDIT_2026-03-08.md:29-31`, `RELEASE_READINESS_AUDIT_2026-03-08.md:99-105`
   - `local tag 一覧は空`、``gh` は api.github.com 接続不可`、`local Rust toolchain は壊れており cargo 実行不可` は clone / 監査セッション依存。durable repo doc に置くと stale 化が早い。
   - 特に `~/.cargo/bin/* -> rustup` 起因まで踏み込む表現は、repo source of truth では立証されていない。repo に残すなら「この監査セッションでは `cargo` を実行できなかった」までが上限。

2. High — blocker 数と重み付けが文書内で自己矛盾。
   - 対象: `RELEASE_READINESS_AUDIT_2026-03-08.md:7`, `RELEASE_READINESS_AUDIT_2026-03-08.md:75-81`, `RELEASE_READINESS_AUDIT_2026-03-08.md:109-118`
   - Executive summary は「最重要 blocker は 6 件」。
   - ただし Findings 6 は `blocker 寄り` 止まり。
   - さらに後段 `Blockers` では 8 項目列挙。
   - いまのままでは summary と本文で severity taxonomy が一致しない。

3. Medium/High — draft design spec を hard gate 扱いしていて根拠が強すぎる。
   - 対象: `RELEASE_READINESS_AUDIT_2026-03-08.md:67-81`, `RELEASE_READINESS_AUDIT_2026-03-08.md:116-117`
   - 根拠にしている `skills/design-spec.md` は `status: "Draft"` 明記 (`skills/design-spec.md:1-5`)。
   - このため multi-arch、署名、`.sig`、provenance、SBOM を「現行 release blocker」と断定するには弱い。repo の binding な release docs は `RELEASE_HANDOFF_CHECKLIST.md` と `CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md` で、そこまでは要求していない。
   - repo 事実として安全なのは「draft design spec との差分がある」まで。

4. Medium — rehearsal 不在の言い切りが evidence を少し越える。
   - 対象: `RELEASE_READINESS_AUDIT_2026-03-08.md:21`, `RELEASE_READINESS_AUDIT_2026-03-08.md:45-47`
   - repo から言えるのは「この clone では local tag が無い」「`release.yml` に non-publishing dry-run 経路が無い」まで。
   - `次の tag が実質 first live fire` は remote/history 側の可能性を閉じ切れていない。`repo-local evidence of rehearsal not found` 程度が安全。

5. Medium — package surface 指摘の impact wording が過剰。
   - 対象: `RELEASE_READINESS_AUDIT_2026-03-08.md:53-55`
   - `Cargo.toml` に `include` / `exclude` が無いこと、non-runtime file が多数 tracked であること自体は repo と整合。
   - ただし `accidental leak` は強すぎる。ここで挙げている docs/skill files は既に public repo 上の tracked files。repo 事実ベースでは `package bloat` / `published surface noise` が適切。

6. Low — Dependabot 運用判断の節は大筋整合だが、release-readiness 監査の主線から少し外れる。
   - 対象: `RELEASE_READINESS_AUDIT_2026-03-08.md:9`, `RELEASE_READINESS_AUDIT_2026-03-08.md:83-89`, `RELEASE_READINESS_AUDIT_2026-03-08.md:132`
   - `DEPENDABOT_TRIAGE_2026-03-06.md` と `DEPENDABOT_MERGE_PLAYBOOK_2026-03-06.md` とは整合。
   - ただし docs-only 監査文書としては scope を広げる。残すなら appendix 扱いの方が読み手に優しい。

補足:

- repo 実体と強く整合している中核指摘もある。`Cargo.lock` と `--locked` の不整合、`origin/main` 未昇格の release docs、Homebrew tap 実体不在、README の install 導線先行は、現 repo source of truth と噛み合っている。

## Final verdict

`safe to commit as docs-only: No`

## If No, exact fixes required

1. `RELEASE_READINESS_AUDIT_2026-03-08.md:21`, `:29-31`, `:99-105` の session-local 記述を削るか、`Audit environment limitations (this session only)` のような明示ラベル付き補遺へ隔離する。
2. `cargo` 不可の原因断定を削る。残すなら観測事実だけにする: 例 `この監査セッションでは cargo を実行できなかった`。
3. blocker taxonomy を一本化する。`6 blockers` と言うなら後段も 6 件に揃える。揃えないなら executive summary を `主要 gaps` 表現へ変更する。
4. `RELEASE_READINESS_AUDIT_2026-03-08.md:67-81`, `:116-117` は `draft design spec との差分` に格下げするか、binding な release doc を追加してから blocker 扱いにする。
5. `RELEASE_READINESS_AUDIT_2026-03-08.md:45-47` の `first live fire` 表現を、`repo-local evidence of rehearsal not found` 等の非断定表現へ差し替える。
6. `RELEASE_READINESS_AUDIT_2026-03-08.md:55` の `accidental leak` は `package bloat` / `published surface noise` 系へ差し替える。

上記修正後なら、docs-only commit 候補としてはかなり安全になる。
