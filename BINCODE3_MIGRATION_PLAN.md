# Bincode 3 Migration Plan

目的: `bincode` 1.x の `serialize` / `deserialize` 直呼びを、config ベースの新 API へ置換する。今回は計画のみ。コード変更なし。

Status sync 2026-03-10:
- Dependabot PR `#7` は re-review 後も as-is merge 不可。
- release-workflow 側の Dependabot PR `#4` / `#3` は同日 merge 済み。
- この文書は `#7` の manual migration planning 専用として維持する。

## 0. 先に押さえる前提

- 現状依存: `Cargo.toml:18` は `bincode = "1"`。
- 現コードは `Options` / `DefaultOptions` 未使用。移行対象は `serialize` / `deserialize` 4 箇所のみ。
- 既存 DB 復号と backup import/export は、`AadData` / `EncryptedBackupFile` のバイト表現互換が前提。
- 公式 migration guide 上の新 API は `bincode::serde::{encode_to_vec, decode_from_slice}` + `bincode::config::*`。
- 実務上の注意: `bincode = "3"` はそのまま bump 先として扱わず、まず「新 API surface を実際に提供する版」を確定すること。現時点の公式 docs/migration guide は 2.x/trunk 系に寄っている。

推奨方針:

- 実装は `bincode::serde::*` + `bincode::config::legacy()` ベースで進める。
- 依存行は next pass で、実際にその API を提供する版へ更新する。
- `legacy()` を使う理由: 既存 encrypted record / backup のバイト互換を崩さないため。

## 1. 現在の usage point

| File | Current pattern | Purpose | Priority |
| --- | --- | --- | --- |
| `Cargo.toml:18` | `bincode = "1"` | 依存定義 | P0 |
| `src/crypto.rs:83` | `bincode::serialize(self)` | AAD bytes 生成 | P0 |
| `src/main.rs:841` | `bincode::serialize(&payload)` | encrypted backup export | P0 |
| `src/main.rs:870` | `bincode::deserialize(&encoded)` | encrypted backup import | P0 |
| `tests/cli_test.rs:164` | `bincode::serialize(&aad)` | integration test seed data | P0 |

補足:

- `src/storage.rs` / `src/ui.rs` は bincode 直呼びなし。
- ただし `AadData::to_aad_bytes()` 経由で暗号処理に依存するため、`src/crypto.rs` の変更は storage/ui/runtime 全体へ波及する。

## 2. Callsite → replacement map

### A. AAD serialization

- 現在: `bincode::serialize(self)`
- 置換先候補:
  - `bincode::serde::encode_to_vec(self, bincode::config::legacy())`
- 対象:
  - `src/crypto.rs:83`
  - `tests/cli_test.rs:164`
- 注意:
  - ここで `standard()` を使うと、既存 secret の AAD byte 列が変わり復号不能化リスクあり。
  - `AadData` は `serde::{Serialize, Deserialize}` 既存 derive のままで十分。`Encode/Decode` への全面移行は不要。

### B. Backup file serialization

- 現在: `bincode::serialize(&payload)`
- 置換先候補:
  - `bincode::serde::encode_to_vec(&payload, bincode::config::legacy())`
- 対象:
  - `src/main.rs:841`
- 注意:
  - 既存 export 済み backup を将来 import する要件があるなら `legacy()` 必須。

### C. Backup file deserialization

- 現在: `bincode::deserialize(&encoded)`
- 置換先候補:
  - `bincode::serde::decode_from_slice(&encoded, bincode::config::legacy())`
- 対象:
  - `src/main.rs:870`
- 必須追加処理:
  - 戻り値が `(value, bytes_read)` になるので unpack が必要。
- 判断ポイント:
  - 互換優先: `bytes_read` は受け取るが trailing bytes は許容。
  - 厳格化優先: `bytes_read == encoded.len()` を確認し、余剰 bytes を invalid backup 扱い。
- 推奨:
  - backup import は厳格化寄り。`bytes_read != encoded.len()` を明示エラーにする。
  - 理由: import payload は外部入力で、緩い decode を残す利点が薄い。

## 3. Required code changes, next pass

### Step 1. 依存更新

- `Cargo.toml`
  - `bincode = "1"` を、新 API + `serde` feature を使える定義へ更新。
- 依存選定時チェック:
  - `bincode::serde::*` が利用可能か
  - `config::legacy()` が提供されるか
  - `EncodeError` / `DecodeError` の error message が現行 error wrapping と相性問題ないか

### Step 2. 共通 helper の導入

実装を散らさないため、薄い helper を 1 箇所へ寄せるのが安全。

候補:

- `encode_legacy<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, _>`
- `decode_legacy<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<(T, usize), _>`

配置候補:

- 最小差分なら `src/crypto.rs` 内 private helper + `src/main.rs` 側は直書き
- 再利用優先なら `src/codec.rs` 新設

今回の repo 規模なら、next pass は「直書き 3 箇所」でも十分。将来呼び出しが増えるなら helper 化でよい。

### Step 3. AAD path 更新

- `src/crypto.rs`
  - `AadData::to_aad_bytes()` を `encode_to_vec(..., legacy())` 化
  - error mapping は現状どおり `CryptoError::AadSerialization(err.to_string())`

### Step 4. Backup export/import path 更新

- `src/main.rs`
  - export: `encode_to_vec(..., legacy())`
  - import: `decode_from_slice(..., legacy())`
  - import 側で `bytes_read` を扱う
  - trailing bytes を reject するなら `CliError::InvalidArgument` へ寄せる

### Step 5. Test seed path 更新

- `tests/cli_test.rs`
  - `bincode::serialize(&aad)` を `encode_to_vec(..., legacy())` へ更新
  - test fixture seed が production AAD bytes と同一 config を使うよう維持

## 4. Test strategy

既存 test 更新だけでは弱い。互換回帰を 1 本足すべき。

### 既存 test で守れるもの

- `src/crypto.rs` unit tests
  - AAD mismatch / decrypt / unwrap 系
- `src/storage.rs` unit tests
  - rotation 時の decrypt/rewrap 整合
- `tests/cli_test.rs`
  - seeded secret を使う CLI integration

### 追加推奨 test

1. `AAD byte compatibility` test
- 目的: `legacy()` から逸脱したとき即落とす
- 内容: 固定 `AadData` を encode し、期待 bytes か既知 fixture と一致確認

2. `backup export/import roundtrip` integration test
- 目的: `EncryptedBackupFile` encode/decode 変更の実害を直接検出
- 内容: `export --encrypted` → `import --encrypted` → secret が復元されること

3. `backup trailing bytes rejection` test
- 条件: import を strict にする場合のみ
- 内容: 正常 backup の末尾に garbage を足して import が失敗すること

### 実行順

1. `cargo test` で unit/integration 一式
2. 可能なら `cargo test --test cli_test` を個別再実行
3. export/import 新規 test が入るならそれを最後に単体再実行

補足:

- 現作業環境では `cargo` 不在のため、今回は test 実行未確認。

## 5. Implementation order, practical

1. 旧 branch/HEAD から「現行 backup fixture」または期待 AAD bytes を 1 つ採取
2. `Cargo.toml` の `bincode` 定義を新 API 利用可能な版へ更新
3. `src/crypto.rs` の AAD encode を先に置換
4. `src/main.rs` の backup export/import を置換
5. `tests/cli_test.rs` の seed 生成を置換
6. AAD compatibility test を追加
7. backup roundtrip test を追加
8. full test 実行

## 6. Open decisions

### Decision 1. Semver target

- 選ぶこと:
  - 実装 target を「official migration guide に沿う新 API surface」へ置くか
  - その surface を提供する crate version / package 名を何にするか

### Decision 2. Import strictness

- 選ぶこと:
  - `decode_from_slice` の `bytes_read` を無視して互換優先
  - 末尾 garbage を reject して安全側へ倒す

推奨:

- AAD / stored-secret path は互換最優先
- backup import は strict 化

## 7. Short next-pass checklist

- [ ] 依存 target 版を確定
- [ ] `serde` feature を有効化
- [ ] `legacy()` で全 callsite を統一
- [ ] `decode_from_slice` の `bytes_read` 方針を実装
- [ ] AAD compatibility test 追加
- [ ] backup export/import roundtrip test 追加
- [ ] full `cargo test` 実行
