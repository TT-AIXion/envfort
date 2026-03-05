use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm_siv::aead::{Aead, KeyInit, Payload};
use aes_gcm_siv::{Aes256GcmSiv, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TEST_PASSPHRASE: &str = "integration-test-passphrase";
const TEST_PROFILE: &str = "default";
const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;
const DEFAULT_PASSPHRASE_SALT: &[u8] = b"envfort-passphrase-salt-v1";

#[derive(Debug, Serialize, Deserialize)]
struct TestAadData {
    profile_id: String,
    key_id: String,
    record_version: u32,
    aead_alg: String,
    kek_id: String,
}

fn new_temp_home(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "envfort-it-{tag}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ))
}

fn create_base_command(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_envfort"));
    cmd.env("HOME", home)
        .env("ENVFORT_PASSPHRASE", TEST_PASSPHRASE);
    cmd
}

fn resolve_python3_path() -> String {
    let output = Command::new("python3")
        .args([
            "-c",
            "import os,sys; print(os.path.realpath(sys.executable))",
        ])
        .output()
        .expect("resolve python3 path");
    assert!(output.status.success(), "python3 path resolution failed");
    String::from_utf8(output.stdout)
        .expect("python3 path utf8")
        .trim()
        .to_string()
}

fn write_allowlist_config(home: &Path, commands: &[(&str, Option<&str>)]) {
    let envfort_dir = home.join(".envfort");
    fs::create_dir_all(&envfort_dir).expect("create envfort dir");
    fs::set_permissions(&envfort_dir, fs::Permissions::from_mode(0o700))
        .expect("chmod envfort dir");

    let mut entries = Vec::with_capacity(commands.len());
    for (path, hash) in commands {
        let escaped_path = format!("{path:?}");
        match hash {
            Some(hash) => entries.push(format!("{{ path = {escaped_path}, hash = {hash:?} }}")),
            None => entries.push(format!("{{ path = {escaped_path} }}")),
        }
    }

    let body = format!("[run.allowlist]\ncommands = [{}]\n", entries.join(", "));
    let config_path = envfort_dir.join("config.toml");
    fs::write(&config_path, body).expect("write config.toml");
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600))
        .expect("chmod config.toml");
}

fn allowlist_python3(home: &Path) {
    let python_path = resolve_python3_path();
    write_allowlist_config(home, &[(&python_path, None)]);
}

fn derive_test_kek(passphrase: &str, profile: &str) -> [u8; KEY_SIZE] {
    let mut salt = Vec::with_capacity(DEFAULT_PASSPHRASE_SALT.len() + profile.len());
    salt.extend_from_slice(DEFAULT_PASSPHRASE_SALT);
    salt.extend_from_slice(profile.as_bytes());
    if salt.len() < 16 {
        salt.resize(16, b'0');
    }

    let params = Params::new(65_536, 3, 4, Some(KEY_SIZE)).expect("argon2 params");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0_u8; KEY_SIZE];
    argon2
        .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
        .expect("derive kek");
    key
}

fn encrypt_with_key(key: &[u8], nonce: &[u8; NONCE_SIZE], msg: &[u8], aad: &[u8]) -> Vec<u8> {
    let cipher = Aes256GcmSiv::new_from_slice(key).expect("cipher");
    cipher
        .encrypt(Nonce::from_slice(nonce), Payload { msg, aad })
        .expect("encrypt")
}

fn seed_secret_for_run(home: &Path, key_name: &str, value: &str) {
    let vault_dir = home.join(".envfort");
    fs::create_dir_all(&vault_dir).expect("create vault dir");
    fs::set_permissions(&vault_dir, fs::Permissions::from_mode(0o700)).expect("chmod vault dir");

    let db_path = vault_dir.join("vault.db");
    let conn = Connection::open(&db_path).expect("open db");
    conn.execute_batch(
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
        CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v TEXT);
        CREATE TABLE IF NOT EXISTS profiles(name TEXT PRIMARY KEY, created_at INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS audit_log(
          id INTEGER PRIMARY KEY,
          timestamp INTEGER NOT NULL,
          action TEXT NOT NULL,
          key_name TEXT,
          profile TEXT,
          detail TEXT
        );
        ",
    )
    .expect("create schema");

    let kek = derive_test_kek(TEST_PASSPHRASE, TEST_PROFILE);
    let dek = [0x7a_u8; KEY_SIZE];
    let key_id = Uuid::new_v4().to_string();
    let aad = TestAadData {
        profile_id: TEST_PROFILE.to_string(),
        key_id: key_id.clone(),
        record_version: 1,
        aead_alg: "AES-256-GCM-SIV".to_string(),
        kek_id: format!("envfort:{TEST_PROFILE}"),
    };
    let aad_bytes = bincode::serialize(&aad).expect("serialize aad");

    let value_nonce = [0x11_u8; NONCE_SIZE];
    let wrap_nonce = [0x22_u8; NONCE_SIZE];
    let ciphertext = encrypt_with_key(&dek, &value_nonce, value.as_bytes(), &aad_bytes);
    let wrapped = encrypt_with_key(&kek, &wrap_nonce, &dek, &aad_bytes);

    let mut encrypted_dek = Vec::with_capacity(NONCE_SIZE + wrapped.len());
    encrypted_dek.extend_from_slice(&wrap_nonce);
    encrypted_dek.extend_from_slice(&wrapped);

    let now = 1_700_000_000_i64;
    conn.execute(
        "
        INSERT INTO secrets (
            profile, key_name, key_id, kek_id, version, aead_alg, nonce,
            encrypted_dek, ciphertext, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8, ?9, ?10)
        ",
        rusqlite::params![
            TEST_PROFILE,
            key_name,
            key_id,
            aad.kek_id,
            aad.aead_alg,
            value_nonce.to_vec(),
            encrypted_dek,
            ciphertext,
            now,
            now
        ],
    )
    .expect("insert seeded secret");
}

#[test]
fn help_exits_successfully() {
    let status = Command::new(env!("CARGO_BIN_EXE_envfort"))
        .arg("--help")
        .status()
        .expect("execute envfort --help");

    assert!(
        status.success(),
        "expected exit code 0 for --help, got status: {status:?}"
    );
}

#[test]
fn subcommand_help_exits_successfully() {
    let subcommands = [
        vec!["allowlist", "--help"],
        vec!["allowlist", "list", "--help"],
        vec!["allowlist", "add", "--help"],
        vec!["allowlist", "rm", "--help"],
        vec!["rotate-kek", "--help"],
        vec!["export", "--help"],
        vec!["import", "--help"],
        vec!["audit", "--help"],
        vec!["kdf", "--help"],
        vec!["kdf", "calibrate", "--help"],
        vec!["profile", "--help"],
        vec!["profile", "create", "--help"],
        vec!["profile", "list", "--help"],
        vec!["profile", "delete", "--help"],
        vec!["ui", "--help"],
    ];

    for args in subcommands {
        let status = Command::new(env!("CARGO_BIN_EXE_envfort"))
            .args(&args)
            .status()
            .expect("execute envfort subcommand --help");
        assert!(
            status.success(),
            "expected exit code 0 for args {args:?}, got status: {status:?}"
        );
    }
}

#[test]
fn allowlist_add_list_rm_flow_works() {
    let temp_home = new_temp_home("allowlist-cli-flow");
    let python_path = resolve_python3_path();

    let add_status = create_base_command(&temp_home)
        .args(["allowlist", "add", "python3"])
        .status()
        .expect("execute allowlist add");
    assert!(add_status.success(), "allowlist add failed: {add_status:?}");

    let list_output = create_base_command(&temp_home)
        .args(["allowlist", "list"])
        .output()
        .expect("execute allowlist list");
    assert!(list_output.status.success(), "allowlist list failed");
    let listed = String::from_utf8_lossy(&list_output.stdout);
    assert!(listed.contains(&python_path), "list output={listed}");
    assert!(listed.contains("sha256:"), "list output={listed}");

    let rm_status = create_base_command(&temp_home)
        .args(["allowlist", "rm", "python3"])
        .status()
        .expect("execute allowlist rm");
    assert!(rm_status.success(), "allowlist rm failed: {rm_status:?}");

    let list_after_output = create_base_command(&temp_home)
        .args(["allowlist", "list"])
        .output()
        .expect("execute allowlist list after rm");
    assert!(
        list_after_output.status.success(),
        "allowlist list after rm failed"
    );
    let listed_after = String::from_utf8_lossy(&list_after_output.stdout);
    assert!(
        !listed_after.contains(&python_path),
        "allowlist entry should be removed: {listed_after}"
    );

    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn kdf_calibrate_runs_successfully() {
    let temp_home = std::env::temp_dir().join(format!(
        "envfort-it-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_home).expect("create temp home");

    let status = Command::new(env!("CARGO_BIN_EXE_envfort"))
        .env("HOME", &temp_home)
        .args(["kdf", "calibrate", "--target-ms", "1"])
        .status()
        .expect("execute kdf calibrate");

    assert!(
        status.success(),
        "expected kdf calibrate to exit 0, got status: {status:?}"
    );

    let _ = std::fs::remove_dir_all(temp_home);
}

#[test]
fn run_inject_env_mode_works() {
    let temp_home = new_temp_home("inject-env");
    seed_secret_for_run(&temp_home, "MY_SECRET", "s3cr3t");
    allowlist_python3(&temp_home);

    let status = create_base_command(&temp_home)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--inject",
            "env",
            "--",
            "python3",
            "-c",
            "import os,sys; sys.exit(0 if os.environ.get('MY_SECRET')=='s3cr3t' else 1)",
        ])
        .status()
        .expect("execute run --inject env");

    assert!(status.success(), "env injection failed: {status:?}");
    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn run_inject_stdin_mode_works() {
    let temp_home = new_temp_home("inject-stdin");
    seed_secret_for_run(&temp_home, "MY_SECRET", "s3cr3t");
    allowlist_python3(&temp_home);

    let status = create_base_command(&temp_home)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--inject",
            "stdin",
            "--",
            "python3",
            "-c",
            "import sys; lines=sys.stdin.read().splitlines(); sys.exit(0 if 'MY_SECRET=s3cr3t' in lines else 1)",
        ])
        .status()
        .expect("execute run --inject stdin");

    assert!(status.success(), "stdin injection failed: {status:?}");
    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn run_inject_fd_mode_works() {
    let temp_home = new_temp_home("inject-fd");
    seed_secret_for_run(&temp_home, "MY_SECRET", "s3cr3t");
    allowlist_python3(&temp_home);

    let status = create_base_command(&temp_home)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--inject",
            "fd",
            "--",
            "python3",
            "-c",
            "import os,sys; fd=int(os.environ['ENVFORT_SECRET_FD']); data=os.read(fd,4096).decode(); sys.exit(0 if 'MY_SECRET=s3cr3t' in data.splitlines() else 1)",
        ])
        .status()
        .expect("execute run --inject fd");

    assert!(status.success(), "fd injection failed: {status:?}");
    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn run_llm_safe_defaults_to_fd() {
    let temp_home = new_temp_home("inject-llm-safe");
    seed_secret_for_run(&temp_home, "MY_SECRET", "s3cr3t");
    allowlist_python3(&temp_home);

    let status = create_base_command(&temp_home)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--llm-safe",
            "--",
            "python3",
            "-c",
            "import os,sys; fd=int(os.environ['ENVFORT_SECRET_FD']); data=os.read(fd,4096).decode(); sys.exit(0 if 'MY_SECRET=s3cr3t' in data.splitlines() else 1)",
        ])
        .status()
        .expect("execute run --llm-safe");

    assert!(status.success(), "llm-safe fd injection failed: {status:?}");
    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn run_inject_tmpfile_mode_works_and_cleans_up() {
    let temp_home = new_temp_home("inject-tmpfile");
    seed_secret_for_run(&temp_home, "MY_SECRET", "s3cr3t");
    allowlist_python3(&temp_home);
    let marker_file = temp_home.join("tmpfile-marker.txt");

    let status = create_base_command(&temp_home)
        .env("MARKER_FILE", &marker_file)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--inject",
            "tmpfile",
            "--",
            "python3",
            "-c",
            "import os,sys,pathlib; p=os.environ['ENVFORT_SECRET_FILE']; lines=open(p).read().splitlines(); pathlib.Path(os.environ['MARKER_FILE']).write_text(p); sys.exit(0 if 'MY_SECRET=s3cr3t' in lines else 1)",
        ])
        .status()
        .expect("execute run --inject tmpfile");

    assert!(status.success(), "tmpfile injection failed: {status:?}");

    let generated_path = fs::read_to_string(&marker_file).expect("read tmpfile marker");
    assert!(
        !Path::new(generated_path.trim()).exists(),
        "tmpfile should be removed after child exit"
    );

    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn run_inject_socket_mode_works_and_cleans_up() {
    let temp_home = new_temp_home("inject-socket");
    seed_secret_for_run(&temp_home, "MY_SECRET", "s3cr3t");
    allowlist_python3(&temp_home);
    let marker_file = temp_home.join("socket-marker.txt");

    let status = create_base_command(&temp_home)
        .env("MARKER_SOCKET", &marker_file)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--inject",
            "socket",
            "--",
            "python3",
            "-c",
            "import os,socket,sys,pathlib; p=os.environ['ENVFORT_SECRET_SOCKET']; s=socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); s.connect(p); data=s.recv(4096).decode(); s.close(); pathlib.Path(os.environ['MARKER_SOCKET']).write_text(p); sys.exit(0 if 'MY_SECRET=s3cr3t' in data.splitlines() else 1)",
        ])
        .status()
        .expect("execute run --inject socket");

    assert!(status.success(), "socket injection failed: {status:?}");

    let generated_path = fs::read_to_string(&marker_file).expect("read socket marker");
    assert!(
        !Path::new(generated_path.trim()).exists(),
        "socket path should be removed after child exit"
    );

    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn run_ci_rejects_unknown_command() {
    let temp_home = new_temp_home("allowlist-ci-reject");
    seed_secret_for_run(&temp_home, "MY_SECRET", "s3cr3t");

    let output = create_base_command(&temp_home)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--ci",
            "--inject",
            "env",
            "--",
            "python3",
            "-c",
            "import os,sys; sys.exit(0 if os.environ.get('MY_SECRET')=='s3cr3t' else 1)",
        ])
        .output()
        .expect("execute run --ci");

    assert!(
        !output.status.success(),
        "ci mode must reject unknown command"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("allowlist"), "stderr={stderr}");
    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn run_ci_allows_allowlisted_command() {
    let temp_home = new_temp_home("allowlist-ci-allow");
    seed_secret_for_run(&temp_home, "MY_SECRET", "s3cr3t");
    allowlist_python3(&temp_home);

    let status = create_base_command(&temp_home)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--ci",
            "--inject",
            "env",
            "--",
            "python3",
            "-c",
            "import os,sys; sys.exit(0 if os.environ.get('MY_SECRET')=='s3cr3t' else 1)",
        ])
        .status()
        .expect("execute run --ci allowlisted");

    assert!(
        status.success(),
        "ci allowlisted command failed: {status:?}"
    );
    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn run_interactive_approval_adds_allowlist() {
    let temp_home = new_temp_home("allowlist-interactive");
    seed_secret_for_run(&temp_home, "MY_SECRET", "s3cr3t");
    let python_path = resolve_python3_path();

    let mut child = create_base_command(&temp_home)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--inject",
            "env",
            "--",
            "python3",
            "-c",
            "import os,sys; sys.exit(0 if os.environ.get('MY_SECRET')=='s3cr3t' else 1)",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn run command");

    let mut stdin = child.stdin.take().expect("stdin handle");
    stdin.write_all(b"y\n").expect("write interactive approval");

    let output = child.wait_with_output().expect("wait for child");
    assert!(
        output.status.success(),
        "interactive approval command failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let config_path = temp_home.join(".envfort").join("config.toml");
    let config = fs::read_to_string(&config_path).expect("read config");
    assert!(config.contains(&python_path), "config missing command path");
    assert!(config.contains("sha256:"), "config missing command hash");
    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn run_llm_safe_rejects_unknown_command() {
    let temp_home = new_temp_home("allowlist-llm-safe-reject");
    seed_secret_for_run(&temp_home, "MY_SECRET", "s3cr3t");

    let output = create_base_command(&temp_home)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--llm-safe",
            "--",
            "python3",
            "-c",
            "import os,sys; sys.exit(0 if os.environ.get('MY_SECRET')=='s3cr3t' else 1)",
        ])
        .output()
        .expect("execute run --llm-safe");

    assert!(
        !output.status.success(),
        "llm-safe mode must reject unknown command"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("allowlist"), "stderr={stderr}");
    let _ = fs::remove_dir_all(temp_home);
}

#[test]
fn full_workflow_init_set_list_run() {
    let temp_home = new_temp_home("full-workflow");

    let init_status = create_base_command(&temp_home)
        .args(["init", "--profile", TEST_PROFILE])
        .status()
        .expect("execute init");
    assert!(init_status.success(), "init failed: {init_status:?}");

    let mut set_child = create_base_command(&temp_home)
        .args(["set", "API_TOKEN", "--profile", TEST_PROFILE])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn set");
    set_child
        .stdin
        .take()
        .expect("set stdin")
        .write_all(b"workflow-secret\n")
        .expect("write secret");
    let set_output = set_child.wait_with_output().expect("wait set");
    assert!(
        set_output.status.success(),
        "set failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&set_output.stdout),
        String::from_utf8_lossy(&set_output.stderr)
    );

    let list_output = create_base_command(&temp_home)
        .args(["list", "--profile", TEST_PROFILE])
        .output()
        .expect("execute list");
    assert!(list_output.status.success(), "list failed");
    let listed = String::from_utf8_lossy(&list_output.stdout);
    assert!(listed.contains("API_TOKEN"), "list output={listed}");

    allowlist_python3(&temp_home);
    let run_status = create_base_command(&temp_home)
        .args([
            "run",
            "--profile",
            TEST_PROFILE,
            "--ci",
            "--inject",
            "env",
            "--",
            "python3",
            "-c",
            "import os,sys; sys.exit(0 if os.environ.get('API_TOKEN')=='workflow-secret' else 1)",
        ])
        .status()
        .expect("execute run");
    assert!(run_status.success(), "run failed: {run_status:?}");

    let _ = fs::remove_dir_all(temp_home);
}
