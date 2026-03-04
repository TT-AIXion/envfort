// main.rs で有効化:
// mod storage;

use std::path::Path;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::StorageError;

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

pub struct VaultDb {
    conn: Connection,
}

impl VaultDb {
    pub fn init_db<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

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
        let mut stmt = self.conn.prepare(
            "SELECT key_name FROM secrets WHERE profile = ?1 ORDER BY key_name ASC",
        )?;

        let rows = stmt.query_map(params![profile], |row| row.get::<_, String>(0))?;
        let key_names = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(key_names)
    }

    pub fn delete_secret(&self, profile: &str, key_name: &str) -> Result<bool, StorageError> {
        let affected = self.conn.execute(
            "DELETE FROM secrets WHERE profile = ?1 AND key_name = ?2",
            params![profile, key_name],
        )?;

        Ok(affected > 0)
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>, StorageError> {
        let value = self
            .conn
            .query_row("SELECT v FROM meta WHERE k = ?1", params![key], |row| row.get(0))
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
}

#[cfg(test)]
mod tests {
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

        assert!(db
            .delete_secret("default", "API_TOKEN")
            .expect("delete secret"));
        assert!(db
            .get_secret("default", "API_TOKEN")
            .expect("get after delete")
            .is_none());
        assert_eq!(
            db.list_secrets("default").expect("list after delete"),
            Vec::<String>::new()
        );
        assert!(!db
            .delete_secret("default", "API_TOKEN")
            .expect("delete absent secret"));
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

        assert!(db
            .delete_secret("profile-a", "SHARED_KEY")
            .expect("delete profile-a"));
        assert!(db
            .get_secret("profile-a", "SHARED_KEY")
            .expect("get profile-a after delete")
            .is_none());
        assert!(db
            .get_secret("profile-b", "SHARED_KEY")
            .expect("get profile-b remains")
            .is_some());
    }
}
