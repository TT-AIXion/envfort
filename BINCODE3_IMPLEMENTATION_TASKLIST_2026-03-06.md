# Bincode Migration Tasklist — 2026-03-06

目的: Dependabot PR `#7` follow-up。`#7` は merge せず、manual migration PR で処理。

Status sync 2026-03-10:
- PR `#7` は re-review 後も blocked。manual migration 別PR方針のまま。
- GitHub PR status sync 上、PR `#4` / `#3` は merge 済み。open PR queue として残る Dependabot follow-up はこの tasklist のみ。
- ただし `#4` / `#3` merge 後の CI / release rehearsal 記録は playbook 側の別残件。

固定判断:
- Dependabot PR `#7` の提案先 `bincode 3.0.0` は reject。unmaintained / as-is merge 不可。
- target は maintained `bincode 2.x` のみ。`serde` feature + `bincode::config::legacy()` 前提。
- 互換優先点: `AadData` byte 列 / 既存 DB record 復号 / 既存 encrypted backup import。
- 移行は小分け batch のみ。all-at-once 変更禁止。各 batch ごとに gate を通す。

現行 tree メモ (`2026-03-07` audit):
- `tests/` 直下の integration test file は `tests/cli_test.rs` のみ。
- `tests/bincode_compat_test.rs` は未存在。新規追加するなら、追加後の concrete test 名で gate する。

## File-by-file

| File | Concrete step | Acceptance |
| --- | --- | --- |
| `Cargo.toml` | `bincode = "1"` を `bincode = { version = "2", features = ["serde"] }` に変更。`3.x` へは上げない。 | dependency 行が `bincode 2.x` + `serde` を示し、legacy-configured serde encode/decode path が compile する。 |
| `Cargo.lock` | lockfile 更新。 | `Cargo.lock` の `name = "bincode"` stanza が `2.x` を示す。`3.0.0` 不在。 |
| `src/crypto.rs` | `AadData::to_aad_bytes()` の encode call を bincode 2 の legacy-configured serde encode へ置換。error mapping は維持。必要なら同 file の test module に AAD byte fixture assertion を追加。 | `AadData::to_aad_bytes()` 内の旧 `bincode::serialize` が消える。固定 `AadData` の bytes が移行前 fixture と一致。既存 `aad_mismatch_is_detected` も通る。 |
| `src/main.rs` | `cmd_export()` / `cmd_import()` の `EncryptedBackupFile` encode/decode を bincode 2 の legacy-configured serde path へ置換。decode 側は `bytes_read` を見て trailing bytes を `CliError::InvalidArgument` で reject。 | 旧 backup import / export→import roundtrip / trailing-bytes reject をカバーする fixture-backed check が追加され、pass する。 |
| `tests/cli_test.rs` | `seed_secret_for_run()` 内の AAD encode call を main 実装と同じ legacy-configured serde encode へ合わせる。 | seeded integration が通る。`full_workflow_init_set_list_run` 影響なし。 |
| `tests/bincode_compat_test.rs` | 新規追加候補。現状未存在。追加するなら `AadData` byte fixture、旧 backup import、trailing-bytes reject を分離し、`tests/cli_test.rs` へ詰め込まない。 | 追加後、その file の concrete test 名で個別実行できる。失敗時の切り分けが容易。 |
| `src/storage.rs` | source edit なし想定。既存 rotate/rewrap path の回帰確認だけ行う。 | `rotate_profile_kek_rewraps_deks` が通る。AadData bytes 変更の副作用なし。 |

## Batch Plan

| Batch | Scope | Files | Gate |
| --- | --- | --- | --- |
| 0 | PR 方針固定。Dependabot PR `#7` は defer-only。`bincode 3.0.0` は採らない。 | planning docs / PR notes のみ | 次 batch 着手前に「manual migration PR でやる」合意あり。 |
| 1 | 旧 build で fixture 採取。AAD bytes 1件、encrypted backup 1件を保存。 | tests fixture assets or inline bytes | fixture が再利用可能。採取手順がメモ化済み。 |
| 2 | dependency 切替 + AAD path 移行。`Cargo.toml` / `Cargo.lock`、`src/crypto.rs`、`tests/cli_test.rs`。 | `Cargo.toml`, `Cargo.lock`, `src/crypto.rs`, `tests/cli_test.rs` | `cargo test aad_mismatch_is_detected` と `cargo test rotate_profile_kek_rewraps_deks` が pass。`src/crypto.rs` / `tests/cli_test.rs` の旧 `bincode::serialize` call がゼロ。 |
| 3 | backup export/import 移行。strict decode (`bytes_read`) 追加。 | `src/main.rs` | 旧 backup import / trailing-bytes reject / 通常 export→import roundtrip をカバーする fixture-backed check が追加され、pass。 |
| 4 | 互換 test 分離。必要なら `tests/bincode_compat_test.rs` を追加。 | `tests/bincode_compat_test.rs` | file 追加後、そこで導入した concrete test 名を個別実行可能。 |
| 5 | full validation。残存旧 API 掃除確認。 | repo-wide validation only | `fmt` / `clippy` / `test` / grep gate 全 pass。`Cargo.lock` に `bincode 3.0.0` 不在。 |

進め方:
- batch 完了ごとに差分を小さく review。
- gate fail 時、その batch で停止。次 batch へ進まない。
- Batch 2 と Batch 3 を同時に混ぜない。原因切り分け優先。

## Test Commands

```bash
# current repo targets / filters
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test aad_mismatch_is_detected
cargo test rotate_profile_kek_rewraps_deks
cargo test --test cli_test full_workflow_init_set_list_run
cargo test
cargo build --release --locked
rg -n "bincode::serialize|bincode::deserialize" src tests
rg -n '^bincode\s*=' Cargo.toml
sed -n '/^name = "bincode"$/,/^$/p' Cargo.lock
```

追加 gate:
- `tests/bincode_compat_test.rs` を作った後だけ `cargo test --test bincode_compat_test` を実行。
- 互換 test 名は file 作成時点の concrete names をそのまま使う。事前に仮名を固定しない。

期待 pass:
- compile error なし。`serialize` / `deserialize` 旧 API 残存なし。
- 追加した AAD fixture check が成功。既存 DB/backup 互換を維持。
- `rotate_profile_kek_rewraps_deks` 成功。storage rewrap regression なし。
- `cli_test` 成功。seeded secret 復号・通常 workflow とも維持。
- `tests/bincode_compat_test.rs` を追加した場合、その file の concrete tests が成功。old backup import / trailing-bytes reject が期待どおり。
- `Cargo.lock` の `bincode` stanza が `2.x` を示し、`3.0.0` は不在。
- CI も green。

## Abort / Rollback

- 新 build で旧 DB record を復号できない: 即 abort。
- 新 build で既存 encrypted backup を import できない: 即 abort。
- `bincode 2.x + serde` で必要 API が出ない、または lockfile が `3.0.0` に解決: 即 abort。依存方針を再確認。
- trailing-bytes reject で正常 fixture まで落ちる: strict reject はこの PR から外し、互換移行だけ先行。
- rollback は `Cargo.toml` / `Cargo.lock` / `src/crypto.rs` / `src/main.rs` / tests を一括。部分 rollback 禁止。
