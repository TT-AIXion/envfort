# Bincode Migration Tasklist — 2026-03-06

目的: Dependabot PR `#7` follow-up。`#7` は merge せず、manual migration PR で処理。

固定判断:
- target は `bincode 3` ではなく `bincode 2.x`。`3.0.0` は対象外。
- 互換優先点: `AadData` byte 列 / 既存 DB record 復号 / 既存 encrypted backup import。
- config は全 callsite `bincode::config::legacy()` 固定で開始。

## File-by-file

| File | Concrete step | Acceptance |
| --- | --- | --- |
| `Cargo.toml` | `bincode = "1"` を `bincode = { version = "2", features = ["serde"] }` に変更。`3.x` へは上げない。 | `bincode::serde::{encode_to_vec, decode_from_slice}` と `bincode::config::legacy()` が import できる。 |
| `Cargo.lock` | lockfile 更新。 | `bincode 2.x` に解決。`3.0.0` 不在。 |
| `src/crypto.rs` | `AadData::to_aad_bytes()` (`src/crypto.rs:82`) を `encode_to_vec(self, legacy())` へ置換。error mapping は維持。unit test 近傍へ AAD byte fixture test 追加。 | `bincode::serialize` 残存なし。固定 `AadData` の bytes が移行前 fixture と一致。既存 `aad_mismatch_is_detected` も通る。 |
| `src/main.rs` | `EncryptedBackupFile` の export (`src/main.rs:841`) を `encode_to_vec(..., legacy())`、import (`src/main.rs:870`) を `decode_from_slice(..., legacy())` へ置換。`bytes_read` を受けて trailing bytes は `CliError::InvalidArgument` で reject。 | 新 build で旧 backup を import 可能。export→import roundtrip 成功。末尾 garbage 付き backup は失敗。 |
| `tests/cli_test.rs` | `seed_secret_for_run()` の `bincode::serialize(&aad)` (`tests/cli_test.rs:164`) を `encode_to_vec(..., legacy())` へ置換。 | seeded integration が通る。`full_workflow_init_set_list_run` 影響なし。 |
| `tests/bincode_compat_test.rs` | 新規追加。`AadData` byte fixture、旧 backup import、trailing-bytes reject を分離。`tests/cli_test.rs` へ詰め込まない。 | 互換回帰だけを個別実行できる。失敗時の切り分けが容易。 |
| `src/storage.rs` | source edit なし想定。既存 rotate/rewrap path の回帰確認だけ行う。 | `rotate_profile_kek_rewraps_deks` が通る。AadData bytes 変更の副作用なし。 |

## 実装順

1. 旧 build で AAD fixture 1件、encrypted backup fixture 1件を採取。
2. `Cargo.toml` / `Cargo.lock` を `bincode 2.x + serde` へ更新。
3. `src/crypto.rs` の AAD encode を置換。
4. `src/main.rs` の backup encode/decode を置換。
5. `tests/cli_test.rs` seed path を置換。
6. `tests/bincode_compat_test.rs` 追加。
7. 既存 test → 新規互換 test → full gate の順で実行。

## Test Commands

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test aad_mismatch_is_detected
cargo test rotate_profile_kek_rewraps_deks
cargo test --test cli_test full_workflow_init_set_list_run
cargo test --test bincode_compat_test
cargo test
cargo build --release --locked
rg -n "bincode::serialize|bincode::deserialize|bincode = \"1\"" Cargo.toml src tests
rg -n "name = \"bincode\"|version = \"3.0.0\"" Cargo.lock
```

期待 pass:
- compile error なし。`serialize` / `deserialize` 旧 API 残存なし。
- AAD fixture test 成功。既存 DB/backup 互換を維持。
- `rotate_profile_kek_rewraps_deks` 成功。storage rewrap regression なし。
- `cli_test` 成功。seeded secret 復号・通常 workflow とも維持。
- `bincode_compat_test` 成功。old backup import / trailing-bytes reject が期待どおり。
- `Cargo.lock` に `bincode 3.0.0` 不在。
- CI も green。

## Abort / Rollback

- 新 build で旧 DB record を復号できない: 即 abort。
- 新 build で既存 encrypted backup を import できない: 即 abort。
- `bincode 2.x + serde` で必要 API が出ない、または lockfile が `3.0.0` に解決: 即 abort。依存方針を再確認。
- trailing-bytes reject で正常 fixture まで落ちる: strict reject はこの PR から外し、互換移行だけ先行。
- rollback は `Cargo.toml` / `Cargo.lock` / `src/crypto.rs` / `src/main.rs` / tests を一括。部分 rollback 禁止。
