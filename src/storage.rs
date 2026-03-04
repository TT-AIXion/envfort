// main.rs で有効化:
// mod storage;

use std::path::Path;

use chrono::Utc;
use rusqlite::{Connection, params};
#[cfg(test)]
use rusqlite::OptionalExtension;

use crate::crypto::{AadData, KEK, NONCE_SIZE, decrypt_value, encrypt_value, unwrap_dek, wrap_dek};
use crate::error::CryptoError;
use crate::error::StorageError;
use zeroize::Zeroize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretRecord {
    pub profile: String,
    pub key_name: String,
    pub key_id: String,
    pub kek_id: String,
    pub version: i64,
    pub aead_alg: String,
    pub nonce: Vec<u8>,
    pub encrypted_dek: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSecret {
    pub id: i64,
    pub profile: String,
    pub key_name: String,
    pub key_id: String,
    pub kek_id: String,
    pub version: i64,
    pub aead_alg: String,
    pub nonce: Vec<u8>,
    pub encrypted_dek: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    pub id: i64,
    pub timestamp: i64,
    pub action: String,
    pub key_name: Option<String>,
    pub profile: Option<String>,
    pub detail: Option<String>,
}

pub struct VaultDb {
    conn: Connection,
}

impl VaultDb {
    pub fn init_db<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    #[cfg(test)]
    pub fn init_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, StorageError> {
        let db = Self { conn };

        db.conn.execute_batch(
            "
            PRAGMA secure_delete = ON;

            CREATE TABLE IF NOT EXISTS secrets(
              id            INTEGER PRIMARY KEY,
              profile       TEXT    NOT NULL,
              key_name      TEXT    NOT NULL,
              key_id        TEXT    NOT NULL,
              kek_id        TEXT    NOT NULL,
              version       INTEGER NOT NULL,
              aead_alg      TEXT    NOT NULL,
              nonce         BLOB    NOT NULL,
              encrypted_dek BLOB    NOT NULL,
              ciphertext    BLOB    NOT NULL,
              created_at    INTEGER NOT NULL,
              updated_at    INTEGER NOT NULL,
              UNIQUE(profile, key_name)
            );

            CREATE TABLE IF NOT EXISTS meta(
              k TEXT PRIMARY KEY,
              v TEXT
            );

            CREATE TABLE IF NOT EXISTS profiles(
              name       TEXT PRIMARY KEY,
              created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_log(
              id        INTEGER PRIMARY KEY,
              timestamp INTEGER NOT NULL,
              action    TEXT    NOT NULL,
              key_name  TEXT,
              profile   TEXT,
              detail    TEXT
            );
            ",
        )?;

        Ok(db)
    }

    pub fn set_secret(&self, record: &SecretRecord) -> Result<(), StorageError> {
        let now = Utc::now().timestamp();

        self.conn.execute(
            "
            INSERT INTO secrets (
                profile, key_name, key_id, kek_id, version, aead_alg,
                nonce, encrypted_dek, ciphertext, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(profile, key_name) DO UPDATE SET
                key_id = excluded.key_id,
                kek_id = excluded.kek_id,
                version = excluded.version,
                aead_alg = excluded.aead_alg,
                nonce = excluded.nonce,
                encrypted_dek = excluded.encrypted_dek,
                ciphertext = excluded.ciphertext,
                updated_at = excluded.updated_at
            ",
            params![
                &record.profile,
                &record.key_name,
                &record.key_id,
                &record.kek_id,
                record.version,
                &record.aead_alg,
                &record.nonce,
                &record.encrypted_dek,
                &record.ciphertext,
                now,
                now,
            ],
        )?;

        Ok(())
    }

    #[cfg(test)]
    pub fn get_secret(
        &self,
        profile: &str,
        key_name: &str,
    ) -> Result<Option<StoredSecret>, StorageError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT
                id, profile, key_name, key_id, kek_id, version, aead_alg,
                nonce, encrypted_dek, ciphertext, created_at, updated_at
            FROM secrets
            WHERE profile = ?1 AND key_name = ?2
            LIMIT 1
            ",
        )?;

        let record = stmt
            .query_row(params![profile, key_name], |row| {
                Ok(StoredSecret {
                    id: row.get(0)?,
                    profile: row.get(1)?,
                    key_name: row.get(2)?,
                    key_id: row.get(3)?,
                    kek_id: row.get(4)?,
                    version: row.get(5)?,
                    aead_alg: row.get(6)?,
                    nonce: row.get(7)?,
                    encrypted_dek: row.get(8)?,
                    ciphertext: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .optional()?;

        Ok(record)
    }

    pub fn list_secrets(&self, profile: &str) -> Result<Vec<String>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT key_name FROM secrets WHERE profile = ?1 ORDER BY key_name ASC")?;

        let rows = stmt.query_map(params![profile], |row| row.get::<_, String>(0))?;
        let key_names = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(key_names)
    }

    pub fn list_secret_records(&self, profile: &str) -> Result<Vec<StoredSecret>, StorageError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT
                id, profile, key_name, key_id, kek_id, version, aead_alg,
                nonce, encrypted_dek, ciphertext, created_at, updated_at
            FROM secrets
            WHERE profile = ?1
            ORDER BY key_name ASC
            ",
        )?;

        let rows = stmt.query_map(params![profile], |row| {
            Ok(StoredSecret {
                id: row.get(0)?,
                profile: row.get(1)?,
                key_name: row.get(2)?,
                key_id: row.get(3)?,
                kek_id: row.get(4)?,
                version: row.get(5)?,
                aead_alg: row.get(6)?,
                nonce: row.get(7)?,
                encrypted_dek: row.get(8)?,
                ciphertext: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?;

        let records = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn delete_secret(&self, profile: &str, key_name: &str) -> Result<bool, StorageError> {
        let affected = self.conn.execute(
            "DELETE FROM secrets WHERE profile = ?1 AND key_name = ?2",
            params![profile, key_name],
        )?;

        Ok(affected > 0)
    }

    #[cfg(test)]
    pub fn get_meta(&self, key: &str) -> Result<Option<String>, StorageError> {
        let value = self
            .conn
            .query_row("SELECT v FROM meta WHERE k = ?1", params![key], |row| {
                row.get(0)
            })
            .optional()?;

        Ok(value)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "
            INSERT INTO meta(k, v)
            VALUES (?1, ?2)
            ON CONFLICT(k) DO UPDATE SET
                v = excluded.v
            ",
            params![key, value],
        )?;

        Ok(())
    }

    pub fn rotate_profile_kek(
        &self,
        profile: &str,
        old_kek: &KEK,
        new_kek: &KEK,
        new_kek_id: &str,
    ) -> Result<usize, StorageError> {
        let status_key = format!("rotation:{profile}:status");
        let total_key = format!("rotation:{profile}:total");
        let done_key = format!("rotation:{profile}:done");
        let new_kek_id_key = format!("rotation:{profile}:new_kek_id");

        self.set_meta(&status_key, "preparing")?;
        self.set_meta(&done_key, "0")?;
        self.set_meta(&new_kek_id_key, new_kek_id)?;

        self.conn.execute("BEGIN IMMEDIATE TRANSACTION", [])?;
        let result = (|| -> Result<usize, StorageError> {
            set_meta_with_conn(&self.conn, &status_key, "in_progress")?;

            let mut stmt = self.conn.prepare(
                "
                SELECT
                    id, profile, key_name, key_id, kek_id, version, aead_alg,
                    nonce, encrypted_dek, ciphertext, created_at, updated_at
                FROM secrets
                WHERE profile = ?1
                ORDER BY id ASC
                ",
            )?;

            let rows = stmt.query_map(params![profile], |row| {
                Ok(StoredSecret {
                    id: row.get(0)?,
                    profile: row.get(1)?,
                    key_name: row.get(2)?,
                    key_id: row.get(3)?,
                    kek_id: row.get(4)?,
                    version: row.get(5)?,
                    aead_alg: row.get(6)?,
                    nonce: row.get(7)?,
                    encrypted_dek: row.get(8)?,
                    ciphertext: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })?;

            let records = rows.collect::<Result<Vec<_>, _>>()?;
            set_meta_with_conn(&self.conn, &total_key, &records.len().to_string())?;

            let mut processed = 0usize;
            let now = Utc::now().timestamp();

            for record in records {
                if record.encrypted_dek.len() < NONCE_SIZE {
                    return Err(StorageError::Crypto(CryptoError::InvalidLength {
                        kind: "wrapped dek payload",
                        expected: NONCE_SIZE,
                        actual: record.encrypted_dek.len(),
                    }));
                }

                let record_version = u32::try_from(record.version).map_err(|_| {
                    StorageError::Crypto(CryptoError::AadSerialization(
                        "record_version overflow".to_string(),
                    ))
                })?;

                let aad_old = AadData {
                    profile_id: record.profile.clone(),
                    key_id: record.key_id.clone(),
                    record_version,
                    aead_alg: record.aead_alg.clone(),
                    kek_id: record.kek_id.clone(),
                };

                let (wrap_nonce, wrapped_dek) = record.encrypted_dek.split_at(NONCE_SIZE);
                let dek = unwrap_dek(old_kek, wrap_nonce, wrapped_dek, &aad_old)?;

                let aad_new = AadData {
                    profile_id: record.profile.clone(),
                    key_id: record.key_id.clone(),
                    record_version,
                    aead_alg: record.aead_alg.clone(),
                    kek_id: new_kek_id.to_string(),
                };

                let mut plaintext = decrypt_value(&dek, &record.nonce, &record.ciphertext, &aad_old)?;
                let (new_nonce, new_ciphertext) = encrypt_value(&dek, &plaintext, &aad_new)?;
                plaintext.zeroize();
                let (new_wrap_nonce, new_wrapped_dek) = wrap_dek(new_kek, &dek, &aad_new)?;
                let mut payload =
                    Vec::with_capacity(new_wrap_nonce.len() + new_wrapped_dek.len());
                payload.extend_from_slice(&new_wrap_nonce);
                payload.extend_from_slice(&new_wrapped_dek);

                self.conn.execute(
                    "
                    UPDATE secrets
                    SET nonce = ?1, encrypted_dek = ?2, ciphertext = ?3, kek_id = ?4, updated_at = ?5
                    WHERE id = ?6
                    ",
                    params![new_nonce, payload, new_ciphertext, new_kek_id, now, record.id],
                )?;

                processed += 1;
                set_meta_with_conn(&self.conn, &done_key, &processed.to_string())?;
            }

            Ok(processed)
        })();

        match result {
            Ok(processed) => {
                self.conn.execute("COMMIT", [])?;
                self.set_meta(&status_key, "completed")?;
                Ok(processed)
            }
            Err(err) => {
                let _ = self.conn.execute("ROLLBACK", []);
                let _ = self.set_meta(&status_key, "failed");
                Err(err)
            }
        }
    }

    pub fn create_profile(&self, profile: &str) -> Result<(), StorageError> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "
            INSERT INTO profiles(name, created_at)
            VALUES (?1, ?2)
            ON CONFLICT(name) DO NOTHING
            ",
            params![profile, now],
        )?;

        Ok(())
    }

    pub fn list_profiles(&self) -> Result<Vec<String>, StorageError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT name
            FROM (
                SELECT name FROM profiles
                UNION
                SELECT DISTINCT profile AS name FROM secrets
            )
            ORDER BY name ASC
            ",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let profiles = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(profiles)
    }

    pub fn delete_profile(&self, profile: &str) -> Result<bool, StorageError> {
        self.conn
            .execute("DELETE FROM secrets WHERE profile = ?1", params![profile])?;
        let affected = self
            .conn
            .execute("DELETE FROM profiles WHERE name = ?1", params![profile])?;
        Ok(affected > 0)
    }

    pub fn log_audit(
        &self,
        action: &str,
        key_name: Option<&str>,
        profile: Option<&str>,
        detail: Option<&str>,
    ) -> Result<(), StorageError> {
        let timestamp = Utc::now().timestamp();

        self.conn.execute(
            "
            INSERT INTO audit_log(timestamp, action, key_name, profile, detail)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![timestamp, action, key_name, profile, detail],
        )?;

        Ok(())
    }

    pub fn list_audit_entries(&self, limit: usize) -> Result<Vec<AuditEntry>, StorageError> {
        let safe_limit = i64::try_from(limit).map_err(|_| {
            StorageError::Crypto(CryptoError::AadSerialization(
                "audit tail value too large".to_string(),
            ))
        })?;

        let mut stmt = self.conn.prepare(
            "
            SELECT id, timestamp, action, key_name, profile, detail
            FROM audit_log
            ORDER BY id DESC
            LIMIT ?1
            ",
        )?;
        let rows = stmt.query_map(params![safe_limit], |row| {
            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                action: row.get(2)?,
                key_name: row.get(3)?,
                profile: row.get(4)?,
                detail: row.get(5)?,
            })
        })?;

        let mut entries = rows.collect::<Result<Vec<_>, _>>()?;
        entries.reverse();
        Ok(entries)
    }
}

fn set_meta_with_conn(conn: &Connection, key: &str, value: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "
        INSERT INTO meta(k, v)
        VALUES (?1, ?2)
        ON CONFLICT(k) DO UPDATE SET
            v = excluded.v
        ",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::crypto::{
        AadData, decrypt_value, derive_kek_from_passphrase, encrypt_value, generate_dek,
        unwrap_dek, wrap_dek, NONCE_SIZE,
    };

    use super::*;

    fn sample_secret(profile: &str, key_name: &str, marker: u8) -> SecretRecord {
        SecretRecord {
            profile: profile.to_string(),
            key_name: key_name.to_string(),
            key_id: format!("{profile}-{key_name}-id"),
            kek_id: "kek-v1".to_string(),
            version: 1,
            aead_alg: "AES-256-GCM-SIV".to_string(),
            nonce: vec![marker; 12],
            encrypted_dek: vec![marker; 48],
            ciphertext: vec![marker; 64],
        }
    }

    #[test]
    fn crud_operations() {
        let db = VaultDb::init_in_memory().expect("init db");

        let secure_delete: i64 = db
            .conn
            .query_row("PRAGMA secure_delete;", [], |row| row.get(0))
            .expect("query secure_delete pragma");
        assert_eq!(secure_delete, 1);

        db.set_meta("kdf_params", "argon2id:m=65536,t=3,p=4")
            .expect("set meta");
        assert_eq!(
            db.get_meta("kdf_params").expect("get meta"),
            Some("argon2id:m=65536,t=3,p=4".to_string())
        );

        let secret = sample_secret("default", "API_TOKEN", 0xAA);
        db.set_secret(&secret).expect("set secret");

        let stored = db
            .get_secret("default", "API_TOKEN")
            .expect("get secret")
            .expect("secret exists");
        assert_eq!(stored.profile, "default");
        assert_eq!(stored.key_name, "API_TOKEN");
        assert_eq!(stored.key_id, "default-API_TOKEN-id");
        assert_eq!(stored.kek_id, "kek-v1");
        assert_eq!(stored.version, 1);

        let listed = db.list_secrets("default").expect("list secrets");
        assert_eq!(listed, vec!["API_TOKEN".to_string()]);

        db.log_audit(
            "set",
            Some("API_TOKEN"),
            Some("default"),
            Some("created in test"),
        )
        .expect("log audit");

        let audit_count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
            .expect("count audit rows");
        assert_eq!(audit_count, 1);

        assert!(
            db.delete_secret("default", "API_TOKEN")
                .expect("delete secret")
        );
        assert!(
            db.get_secret("default", "API_TOKEN")
                .expect("get after delete")
                .is_none()
        );
        assert_eq!(
            db.list_secrets("default").expect("list after delete"),
            Vec::<String>::new()
        );
        assert!(
            !db.delete_secret("default", "API_TOKEN")
                .expect("delete absent secret")
        );
    }

    #[test]
    fn profile_isolation() {
        let db = VaultDb::init_in_memory().expect("init db");

        let secret_a = sample_secret("profile-a", "SHARED_KEY", 0x11);
        let secret_b = sample_secret("profile-b", "SHARED_KEY", 0x22);

        db.set_secret(&secret_a).expect("set profile-a secret");
        db.set_secret(&secret_b).expect("set profile-b secret");

        let fetched_a = db
            .get_secret("profile-a", "SHARED_KEY")
            .expect("get profile-a")
            .expect("profile-a exists");
        let fetched_b = db
            .get_secret("profile-b", "SHARED_KEY")
            .expect("get profile-b")
            .expect("profile-b exists");

        assert_eq!(fetched_a.profile, "profile-a");
        assert_eq!(fetched_b.profile, "profile-b");
        assert_ne!(fetched_a.ciphertext, fetched_b.ciphertext);

        assert_eq!(
            db.list_secrets("profile-a").expect("list a"),
            vec!["SHARED_KEY".to_string()]
        );
        assert_eq!(
            db.list_secrets("profile-b").expect("list b"),
            vec!["SHARED_KEY".to_string()]
        );

        assert!(
            db.delete_secret("profile-a", "SHARED_KEY")
                .expect("delete profile-a")
        );
        assert!(
            db.get_secret("profile-a", "SHARED_KEY")
                .expect("get profile-a after delete")
                .is_none()
        );
        assert!(
            db.get_secret("profile-b", "SHARED_KEY")
                .expect("get profile-b remains")
                .is_some()
        );
    }

    #[test]
    fn audit_logging_and_meta_operations() {
        let db = VaultDb::init_in_memory().expect("init db");

        db.set_meta("rotation_state", "idle").expect("set meta v1");
        db.set_meta("rotation_state", "running")
            .expect("set meta v2");
        assert_eq!(
            db.get_meta("rotation_state").expect("get meta"),
            Some("running".to_string())
        );
        assert_eq!(db.get_meta("missing").expect("get missing meta"), None);

        db.log_audit("set", Some("API_TOKEN"), Some("default"), Some("created"))
            .expect("log set");
        db.log_audit("run", None, Some("default"), Some("child started"))
            .expect("log run");

        let audit_count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
            .expect("count audit rows");
        assert_eq!(audit_count, 2);

        let actions = db
            .conn
            .prepare("SELECT action FROM audit_log ORDER BY id ASC")
            .expect("prepare action query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query actions")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect actions");
        assert_eq!(actions, vec!["set".to_string(), "run".to_string()]);
    }

    #[test]
    fn rotate_profile_kek_rewraps_deks() {
        let db = VaultDb::init_in_memory().expect("init db");
        let profile = "default";
        let key_name = "TOKEN";

        let old_kek = derive_kek_from_passphrase("old-passphrase", b"0123456789abcdef")
            .expect("derive old kek");
        let new_kek = derive_kek_from_passphrase("new-passphrase", b"fedcba9876543210")
            .expect("derive new kek");
        let dek = generate_dek().expect("generate dek");

        let aad_old = AadData {
            profile_id: profile.to_string(),
            key_id: "key-1".to_string(),
            record_version: 1,
            aead_alg: "AES-256-GCM-SIV".to_string(),
            kek_id: "kek-old".to_string(),
        };
        let (nonce, ciphertext) = encrypt_value(&dek, b"secret-value", &aad_old).expect("encrypt");
        let (wrap_nonce, wrapped_dek) = wrap_dek(&old_kek, &dek, &aad_old).expect("wrap dek");
        let mut encrypted_dek = Vec::with_capacity(wrap_nonce.len() + wrapped_dek.len());
        encrypted_dek.extend_from_slice(&wrap_nonce);
        encrypted_dek.extend_from_slice(&wrapped_dek);

        db.set_secret(&SecretRecord {
            profile: profile.to_string(),
            key_name: key_name.to_string(),
            key_id: "key-1".to_string(),
            kek_id: "kek-old".to_string(),
            version: 1,
            aead_alg: "AES-256-GCM-SIV".to_string(),
            nonce,
            encrypted_dek,
            ciphertext,
        })
        .expect("set secret");

        let rotated = db
            .rotate_profile_kek(profile, &old_kek, &new_kek, "kek-new")
            .expect("rotate kek");
        assert_eq!(rotated, 1);

        let stored = db
            .get_secret(profile, key_name)
            .expect("get rotated secret")
            .expect("secret exists");
        assert_eq!(stored.kek_id, "kek-new");

        let aad_new = AadData {
            profile_id: stored.profile.clone(),
            key_id: stored.key_id.clone(),
            record_version: 1,
            aead_alg: stored.aead_alg.clone(),
            kek_id: stored.kek_id.clone(),
        };
        let (new_wrap_nonce, new_wrapped_dek) = stored.encrypted_dek.split_at(NONCE_SIZE);
        let unwrapped = unwrap_dek(&new_kek, new_wrap_nonce, new_wrapped_dek, &aad_new)
            .expect("unwrap rotated dek");
        let plaintext =
            decrypt_value(&unwrapped, &stored.nonce, &stored.ciphertext, &aad_new).expect("decrypt");
        assert_eq!(plaintext, b"secret-value");
    }
}
