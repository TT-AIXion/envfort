mod cli;
mod crypto;
mod error;
mod keychain;
mod storage;
mod ui;

use std::fs;
use std::io::{self, ErrorKind, Write};
use std::os::fd::RawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;
use std::time::Instant;

use argon2::{Algorithm, Argon2, Params, Version};
use rpassword::prompt_password;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::cli::{
    AuditArgs, Commands, ExportArgs, ImportArgs, InitArgs, InjectMode, KdfArgs, KdfCalibrateArgs,
    KdfCommands, ListArgs, ProfileCommands, RemoveArgs, RotateKekArgs, RunArgs, SetArgs, UiArgs,
    parse_cli,
};
use crate::crypto::{
    AadData, KEK, KEY_SIZE, NONCE_SIZE, decrypt_value, encrypt_value, generate_dek, unwrap_dek,
    wrap_dek,
};
use crate::error::{CliError, CryptoError, KeychainError};
use crate::keychain::{KeychainBackend, OsKeychain, get_backend};
use crate::storage::{SecretRecord, VaultDb};

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedBackupFile {
    version: u32,
    profile: String,
    created_at: i64,
    key_id: String,
    aead_alg: String,
    kek_id: String,
    value_nonce: Vec<u8>,
    ciphertext: Vec<u8>,
    wrap_nonce: Vec<u8>,
    wrapped_dek: Vec<u8>,
}

fn main() {
    if let Err(err) = run_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_main() -> Result<(), CliError> {
    disable_core_dumps()?;
    dispatch_main()
}

fn dispatch_main() -> Result<(), CliError> {
    let cli = parse_cli();

    match cli.command {
        Commands::Init(args) => cmd_init(&args)?,
        Commands::Set(args) => cmd_set(&args)?,
        Commands::List(args) => cmd_list(&args)?,
        Commands::Run(args) => cmd_run(&args)?,
        Commands::Rm(args) => cmd_rm(&args)?,
        Commands::RotateKek(args) => cmd_rotate_kek(&args)?,
        Commands::Export(args) => cmd_export(&args)?,
        Commands::Import(args) => cmd_import(&args)?,
        Commands::Audit(args) => cmd_audit(&args)?,
        Commands::Kdf(args) => cmd_kdf(args)?,
        Commands::Profile(args) => cmd_profile(args.command)?,
        Commands::Ui(args) => cmd_ui(&args)?,
    }

    Ok(())
}

fn cmd_init(args: &InitArgs) -> Result<(), CliError> {
    let data_dir = envfort_data_dir()?;
    ensure_secure_dir(&data_dir)?;

    let db_path = resolve_db_path(&data_dir, &args.db);
    let db = VaultDb::init_db(&db_path)?;
    db.create_profile(&args.profile)?;
    db.set_meta("current_profile", &args.profile)?;

    let kek = generate_random_kek()?;
    let keychain = OsKeychain;
    keychain.store_kek(&args.profile, &kek)?;

    println!(
        "initialized profile={} db={}",
        args.profile,
        db_path.display()
    );
    Ok(())
}

fn cmd_set(args: &SetArgs) -> Result<(), CliError> {
    let db = open_default_db()?;
    db.create_profile(&args.profile)?;

    let backend = get_backend();
    let kek = backend.retrieve_kek(&args.profile)?;

    let prompt = format!("value for {}: ", args.key);
    let secret_value = Zeroizing::new(prompt_password(prompt)?);
    if secret_value.is_empty() {
        return Err(CliError::InvalidArgument(
            "secret value is empty".to_string(),
        ));
    }

    let dek = generate_dek()?;
    let key_id = Uuid::new_v4().to_string();
    let aad = AadData {
        profile_id: args.profile.clone(),
        key_id: key_id.clone(),
        record_version: 1,
        aead_alg: "AES-256-GCM-SIV".to_string(),
        kek_id: format!("envfort:{}", args.profile),
    };

    let (value_nonce, ciphertext) = encrypt_value(&dek, secret_value.as_bytes(), &aad)?;
    let (wrap_nonce, wrapped_dek) = wrap_dek(&kek, &dek, &aad)?;
    let mut encrypted_dek = Vec::with_capacity(wrap_nonce.len() + wrapped_dek.len());
    encrypted_dek.extend_from_slice(&wrap_nonce);
    encrypted_dek.extend_from_slice(&wrapped_dek);

    let record = SecretRecord {
        profile: args.profile.clone(),
        key_name: args.key.clone(),
        key_id,
        kek_id: aad.kek_id.clone(),
        version: i64::from(aad.record_version),
        aead_alg: aad.aead_alg.clone(),
        nonce: value_nonce,
        encrypted_dek,
        ciphertext,
    };

    db.set_secret(&record)?;
    db.log_audit("set", Some(&args.key), Some(&args.profile), None)?;
    println!("stored key={} profile={}", args.key, args.profile);
    Ok(())
}

fn cmd_list(args: &ListArgs) -> Result<(), CliError> {
    let db = open_default_db()?;
    let names = db.list_secrets(&args.profile)?;
    for name in names {
        println!("{name}");
    }
    Ok(())
}

fn cmd_run(args: &RunArgs) -> Result<(), CliError> {
    let (program, command_args) = args
        .command
        .split_first()
        .ok_or_else(|| CliError::InvalidArgument("run command requires a program".to_string()))?;

    let db = open_default_db()?;
    let backend = get_backend();
    let kek = backend.retrieve_kek(&args.profile)?;
    let records = db.list_secret_records(&args.profile)?;
    let inject_mode = resolve_inject_mode(args);

    let mut env_secrets: Vec<(String, Zeroizing<String>)> = Vec::with_capacity(records.len());
    for record in records {
        if record.encrypted_dek.len() < NONCE_SIZE {
            return Err(CliError::InvalidArgument(format!(
                "stored wrapped DEK for key {} is too short",
                record.key_name
            )));
        }

        let record_version = u32::try_from(record.version).map_err(|_| {
            CliError::InvalidArgument(format!(
                "invalid record version {} for key {}",
                record.version, record.key_name
            ))
        })?;

        let aad = AadData {
            profile_id: record.profile.clone(),
            key_id: record.key_id.clone(),
            record_version,
            aead_alg: record.aead_alg.clone(),
            kek_id: record.kek_id.clone(),
        };

        let (wrap_nonce, wrapped_dek) = record.encrypted_dek.split_at(NONCE_SIZE);
        let dek = unwrap_dek(&kek, wrap_nonce, wrapped_dek, &aad)?;
        let plaintext = decrypt_value(&dek, &record.nonce, &record.ciphertext, &aad)?;

        let value = match String::from_utf8(plaintext) {
            Ok(value) => Zeroizing::new(value),
            Err(err) => {
                let mut bytes = err.into_bytes();
                bytes.zeroize();
                return Err(CliError::InvalidArgument(format!(
                    "secret {} is not valid UTF-8",
                    record.key_name
                )));
            }
        };

        env_secrets.push((record.key_name, value));
    }

    let status = match inject_mode {
        InjectMode::Env => {
            let mut child = Command::new(program);
            child.args(command_args);
            for (key, value) in &env_secrets {
                child.env(key, value.as_str());
            }
            child.status()?
        }
        InjectMode::Stdin => {
            let payload = build_secret_payload(&env_secrets);
            let mut child = Command::new(program);
            child.args(command_args);
            child.stdin(Stdio::piped());
            let mut child = child.spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(payload.as_bytes())?;
            }
            child.wait()?
        }
        InjectMode::Fd => {
            let payload = build_secret_payload(&env_secrets);
            let (read_fd, write_fd) = create_pipe()?;

            let mut child = Command::new(program);
            child
                .args(command_args)
                .env("ENVFORT_SECRET_FD", read_fd.to_string());

            let mut child = child.spawn()?;
            write_all_to_fd(write_fd, payload.as_bytes())?;
            close_fd(write_fd)?;

            let status = child.wait()?;
            close_fd(read_fd)?;
            status
        }
        InjectMode::Socket => {
            let payload = build_secret_payload(&env_secrets);
            let socket_path = build_socket_path()?;
            if socket_path.exists() {
                fs::remove_file(&socket_path)?;
            }

            let listener = UnixListener::bind(&socket_path)?;
            listener.set_nonblocking(true)?;

            let stop = Arc::new(AtomicBool::new(false));
            let stop_for_thread = Arc::clone(&stop);
            let socket_path_for_thread = socket_path.clone();
            let payload_bytes = payload.into_bytes();

            let server_thread = thread::spawn(move || -> io::Result<()> {
                loop {
                    match listener.accept() {
                        Ok((mut stream, _addr)) => {
                            stream.write_all(&payload_bytes)?;
                            stream.flush()?;
                            break;
                        }
                        Err(err) if err.kind() == ErrorKind::WouldBlock => {
                            if stop_for_thread.load(Ordering::Relaxed) {
                                break;
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(err) => return Err(err),
                    }
                }
                if socket_path_for_thread.exists() {
                    let _ = fs::remove_file(&socket_path_for_thread);
                }
                Ok(())
            });

            let mut child = Command::new(program);
            child
                .args(command_args)
                .env("ENVFORT_SECRET_SOCKET", &socket_path);
            let child_status = child.status();

            stop.store(true, Ordering::Relaxed);
            let server_result = server_thread
                .join()
                .map_err(|_| CliError::Io(io::Error::other("socket thread panicked")))?;
            if socket_path.exists() {
                let _ = fs::remove_file(&socket_path);
            }
            server_result?;
            child_status?
        }
        InjectMode::Tmpfile => {
            let payload = build_secret_payload(&env_secrets);
            let runtime_dir = ensure_runtime_secret_dir()?;
            let file_path = runtime_dir.join(format!("envfort-secrets-{}.txt", Uuid::new_v4()));

            fs::write(&file_path, payload.as_bytes())?;
            fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600))?;

            let mut child = Command::new(program);
            child
                .args(command_args)
                .env("ENVFORT_SECRET_FILE", &file_path);

            let status = child.status();
            let cleanup_result = fs::remove_file(&file_path);

            let status = status?;
            if let Err(err) = cleanup_result {
                return Err(CliError::Io(err));
            }
            status
        }
    };
    drop(env_secrets);

    if status.success() {
        return Ok(());
    }

    let code = status.code().unwrap_or(1);
    std::process::exit(code);
}

fn create_pipe() -> Result<(RawFd, RawFd), CliError> {
    let mut fds = [0_i32; 2];
    // SAFETY: `fds` points to valid memory for two file descriptors.
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if rc != 0 {
        return Err(CliError::Io(io::Error::last_os_error()));
    }
    Ok((fds[0], fds[1]))
}

fn write_all_to_fd(fd: RawFd, mut data: &[u8]) -> Result<(), CliError> {
    while !data.is_empty() {
        // SAFETY: `data` pointer/length are valid for read, `fd` comes from `pipe`.
        let written = unsafe { libc::write(fd, data.as_ptr().cast(), data.len()) };
        if written < 0 {
            return Err(CliError::Io(io::Error::last_os_error()));
        }

        let written = usize::try_from(written)
            .map_err(|_| CliError::Io(io::Error::other("negative write result")))?;
        data = &data[written..];
    }
    Ok(())
}

fn close_fd(fd: RawFd) -> Result<(), CliError> {
    // SAFETY: closing a valid fd from `pipe`.
    let rc = unsafe { libc::close(fd) };
    if rc == 0 {
        return Ok(());
    }
    Err(CliError::Io(io::Error::last_os_error()))
}

fn resolve_inject_mode(args: &RunArgs) -> InjectMode {
    match args.inject {
        Some(mode) => mode,
        None if args.llm_safe => InjectMode::Fd,
        None => InjectMode::Env,
    }
}

fn build_secret_payload(env_secrets: &[(String, Zeroizing<String>)]) -> String {
    let mut payload = String::new();
    for (key, value) in env_secrets {
        payload.push_str(key);
        payload.push('=');
        payload.push_str(value);
        payload.push('\n');
    }
    payload
}

fn cmd_rm(args: &RemoveArgs) -> Result<(), CliError> {
    let confirm = confirm_prompt(&format!(
        "delete key '{}' from profile '{}'?",
        args.key, args.profile
    ))?;
    if !confirm {
        println!("aborted");
        return Ok(());
    }

    let db = open_default_db()?;
    let deleted = db.delete_secret(&args.profile, &args.key)?;
    if deleted {
        db.log_audit("rm", Some(&args.key), Some(&args.profile), None)?;
        println!("deleted key={} profile={}", args.key, args.profile);
    } else {
        println!("not found key={} profile={}", args.key, args.profile);
    }
    Ok(())
}

fn cmd_rotate_kek(args: &RotateKekArgs) -> Result<(), CliError> {
    let db = open_default_db()?;
    let backend = get_backend();

    let old_kek = backend.retrieve_kek(&args.profile)?;
    let new_kek = generate_random_kek()?;
    let new_kek_id = format!("kek-{}", Uuid::new_v4());

    let rewrapped = db.rotate_profile_kek(&args.profile, &old_kek, &new_kek, &new_kek_id)?;
    if let Err(err) = backend.store_kek(&args.profile, &new_kek) {
        let rollback_kek_id = format!("rollback-kek-{}", Uuid::new_v4());
        let _ = db.rotate_profile_kek(&args.profile, &new_kek, &old_kek, &rollback_kek_id);
        return Err(err.into());
    }

    db.set_meta(&format!("kek_id:{}", args.profile), &new_kek_id)?;
    db.log_audit(
        "rotate-kek",
        None,
        Some(&args.profile),
        Some(&format!("rewrapped={rewrapped}")),
    )?;

    println!("rotated kek profile={} rewrapped={rewrapped}", args.profile);
    Ok(())
}

fn cmd_export(args: &ExportArgs) -> Result<(), CliError> {
    if !args.encrypted {
        return Err(CliError::InvalidArgument(
            "export requires --encrypted".to_string(),
        ));
    }

    let db = open_default_db()?;
    let backend = get_backend();
    let kek = backend.retrieve_kek(&args.profile)?;
    let db_path = default_db_path()?;
    let mut vault_bytes = fs::read(&db_path)?;

    let dek = generate_dek()?;
    let key_id = format!("backup-{}", Uuid::new_v4());
    let aad = AadData {
        profile_id: args.profile.clone(),
        key_id: key_id.clone(),
        record_version: 1,
        aead_alg: "AES-256-GCM-SIV".to_string(),
        kek_id: format!("backup:{}", args.profile),
    };
    let (value_nonce, ciphertext) = encrypt_value(&dek, &vault_bytes, &aad)?;
    vault_bytes.zeroize();

    let (wrap_nonce, wrapped_dek) = wrap_dek(&kek, &dek, &aad)?;
    let payload = EncryptedBackupFile {
        version: 1,
        profile: args.profile.clone(),
        created_at: chrono::Utc::now().timestamp(),
        key_id,
        aead_alg: aad.aead_alg.clone(),
        kek_id: aad.kek_id.clone(),
        value_nonce,
        ciphertext,
        wrap_nonce,
        wrapped_dek,
    };

    let encoded = bincode::serialize(&payload)
        .map_err(|err| CliError::InvalidArgument(format!("backup serialization failed: {err}")))?;
    let output_path = PathBuf::from(&args.output);
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, encoded)?;
    fs::set_permissions(&output_path, fs::Permissions::from_mode(0o600))?;

    db.log_audit(
        "export",
        None,
        Some(&args.profile),
        Some("encrypted backup"),
    )?;
    println!("exported encrypted backup to {}", output_path.display());
    Ok(())
}

fn cmd_import(args: &ImportArgs) -> Result<(), CliError> {
    if !args.encrypted {
        return Err(CliError::InvalidArgument(
            "import requires --encrypted".to_string(),
        ));
    }

    let encoded = fs::read(&args.path)?;
    let payload: EncryptedBackupFile = bincode::deserialize(&encoded).map_err(|err| {
        CliError::InvalidArgument(format!("backup deserialization failed: {err}"))
    })?;
    if payload.version != 1 {
        return Err(CliError::InvalidArgument(format!(
            "unsupported backup version {}",
            payload.version
        )));
    }

    let keychain_profile = args.profile.as_deref().unwrap_or(&payload.profile);
    let backend = get_backend();
    let kek = backend.retrieve_kek(keychain_profile)?;

    let aad = AadData {
        profile_id: payload.profile.clone(),
        key_id: payload.key_id.clone(),
        record_version: payload.version,
        aead_alg: payload.aead_alg.clone(),
        kek_id: payload.kek_id.clone(),
    };
    let dek = unwrap_dek(&kek, &payload.wrap_nonce, &payload.wrapped_dek, &aad)?;
    let mut vault_bytes = decrypt_value(&dek, &payload.value_nonce, &payload.ciphertext, &aad)?;

    let db_path = default_db_path()?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&db_path, &vault_bytes)?;
    fs::set_permissions(&db_path, fs::Permissions::from_mode(0o600))?;
    vault_bytes.zeroize();

    let db = VaultDb::init_db(&db_path)?;
    db.log_audit(
        "import",
        None,
        Some(&payload.profile),
        Some("encrypted backup"),
    )?;

    println!("imported encrypted backup from {}", args.path);
    Ok(())
}

fn cmd_audit(args: &AuditArgs) -> Result<(), CliError> {
    let db = open_default_db()?;
    let entries = db.list_audit_entries(args.tail)?;
    for entry in entries {
        let key_name = entry.key_name.unwrap_or_else(|| "-".to_string());
        let profile = entry.profile.unwrap_or_else(|| "-".to_string());
        let detail = entry.detail.unwrap_or_default();
        println!(
            "{} {} profile={} key={} {}",
            entry.timestamp, entry.action, profile, key_name, detail
        );
    }
    Ok(())
}

fn cmd_kdf(args: KdfArgs) -> Result<(), CliError> {
    match args.command {
        KdfCommands::Calibrate(calibrate_args) => cmd_kdf_calibrate(&calibrate_args),
    }
}

fn cmd_ui(args: &UiArgs) -> Result<(), CliError> {
    if args.timeout == 0 {
        return Err(CliError::InvalidArgument(
            "--timeout must be greater than 0".to_string(),
        ));
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(ui::run_ui(args.no_open, args.timeout))
}

fn cmd_kdf_calibrate(args: &KdfCalibrateArgs) -> Result<(), CliError> {
    if args.target_ms == 0 {
        return Err(CliError::InvalidArgument(
            "--target-ms must be greater than 0".to_string(),
        ));
    }

    let candidates_m = [8_192_u32, 16_384, 32_768, 65_536, 131_072];
    let candidates_t = [1_u32, 2, 3, 4];
    let p_cost = 4_u32;
    let passphrase = b"envfort-kdf-calibration-passphrase";
    let salt = b"0123456789abcdef";

    let mut best_under: Option<(u32, u32, u32, u64)> = None;
    let mut best_over: Option<(u32, u32, u32, u64)> = None;

    for m_cost in candidates_m {
        for t_cost in candidates_t {
            let params = Params::new(m_cost, t_cost, p_cost, Some(KEY_SIZE))
                .map_err(|err| CliError::Crypto(CryptoError::Argon2(err.to_string())))?;
            let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

            let mut output = [0_u8; KEY_SIZE];
            let started = Instant::now();
            argon2
                .hash_password_into(passphrase, salt, &mut output)
                .map_err(|err| CliError::Crypto(CryptoError::Argon2(err.to_string())))?;
            output.zeroize();

            let elapsed_ms = started.elapsed().as_millis() as u64;
            if elapsed_ms <= args.target_ms {
                match best_under {
                    Some((_, _, _, best_ms)) if best_ms >= elapsed_ms => {}
                    _ => best_under = Some((m_cost, t_cost, p_cost, elapsed_ms)),
                }
            } else {
                match best_over {
                    Some((_, _, _, best_ms)) if best_ms <= elapsed_ms => {}
                    _ => best_over = Some((m_cost, t_cost, p_cost, elapsed_ms)),
                }
            }
        }
    }

    let (m_cost, t_cost, p_cost, elapsed_ms) = best_under.or(best_over).ok_or_else(|| {
        CliError::InvalidArgument("failed to derive calibration result".to_string())
    })?;

    let db = open_default_db()?;
    let value = format!("argon2id:m={m_cost},t={t_cost},p={p_cost}");
    db.set_meta("kdf_params", &value)?;
    db.log_audit(
        "kdf-calibrate",
        None,
        None,
        Some(&format!("target_ms={}", args.target_ms)),
    )?;

    println!("recommended {value} measured_ms={elapsed_ms}");
    Ok(())
}

fn cmd_profile(command: ProfileCommands) -> Result<(), CliError> {
    let db = open_default_db()?;

    match command {
        ProfileCommands::Create(args) => {
            db.create_profile(&args.name)?;
            ensure_profile_kek_exists(&args.name)?;
            println!("created profile={}", args.name);
        }
        ProfileCommands::Delete(args) => {
            let confirm =
                confirm_prompt(&format!("delete profile '{}' and all secrets?", args.name))?;
            if !confirm {
                println!("aborted");
                return Ok(());
            }

            let deleted = db.delete_profile(&args.name)?;
            if deleted {
                let backend = get_backend();
                match backend.delete_kek(&args.name) {
                    Ok(()) | Err(KeychainError::MissingEntry(_)) => {}
                    Err(err) => return Err(err.into()),
                }
                println!("deleted profile={}", args.name);
            } else {
                println!("profile not found={}", args.name);
            }
        }
        ProfileCommands::List => {
            let profiles = db.list_profiles()?;
            for profile in profiles {
                println!("{profile}");
            }
        }
    }

    Ok(())
}

fn ensure_profile_kek_exists(profile: &str) -> Result<(), CliError> {
    let keychain = OsKeychain;
    match keychain.retrieve_kek(profile) {
        Ok(_) => Ok(()),
        Err(KeychainError::MissingEntry(_)) => {
            let kek = generate_random_kek()?;
            keychain.store_kek(profile, &kek)?;
            Ok(())
        }
        Err(err) => Err(err.into()),
    }
}

fn open_default_db() -> Result<VaultDb, CliError> {
    let data_dir = envfort_data_dir()?;
    ensure_secure_dir(&data_dir)?;
    let db_path = data_dir.join("vault.db");
    Ok(VaultDb::init_db(db_path)?)
}

fn default_db_path() -> Result<PathBuf, CliError> {
    let data_dir = envfort_data_dir()?;
    ensure_secure_dir(&data_dir)?;
    Ok(data_dir.join("vault.db"))
}

fn envfort_data_dir() -> Result<PathBuf, CliError> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| CliError::InvalidArgument("HOME is not set".to_string()))?;
    Ok(PathBuf::from(home).join(".envfort"))
}

fn ensure_secure_dir(path: &Path) -> Result<(), CliError> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn resolve_db_path(data_dir: &Path, db_arg: &str) -> PathBuf {
    let db_path = PathBuf::from(db_arg);
    if db_path.is_absolute() {
        db_path
    } else {
        data_dir.join(db_path)
    }
}

fn confirm_prompt(message: &str) -> Result<bool, CliError> {
    print!("{message} [y/N]: ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim();
    Ok(answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"))
}

fn generate_random_kek() -> Result<KEK, CliError> {
    let mut raw = [0_u8; KEY_SIZE];
    getrandom::fill(&mut raw).map_err(|err| CliError::Io(io::Error::other(err.to_string())))?;
    let kek = KEK::from_slice(&raw)?;
    raw.zeroize();
    Ok(kek)
}

fn ensure_runtime_secret_dir() -> Result<PathBuf, CliError> {
    let base_dir = match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(path) => PathBuf::from(path).join("envfort"),
        None => envfort_data_dir()?.join("run"),
    };
    fs::create_dir_all(&base_dir)?;
    fs::set_permissions(&base_dir, fs::Permissions::from_mode(0o700))?;
    Ok(base_dir)
}

fn build_socket_path() -> Result<PathBuf, CliError> {
    let runtime_dir = ensure_runtime_secret_dir()?;
    let name = format!("ef-{}.sock", Uuid::new_v4().simple());
    let candidate = runtime_dir.join(&name);
    if candidate.to_string_lossy().len() < 96 {
        return Ok(candidate);
    }

    let fallback = std::env::temp_dir().join(name);
    if fallback.to_string_lossy().len() < 96 {
        return Ok(fallback);
    }

    Err(CliError::InvalidArgument(
        "could not allocate a short unix socket path".to_string(),
    ))
}

fn disable_core_dumps() -> Result<(), CliError> {
    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };

    // SAFETY: setrlimit is called with a valid pointer to a stack-allocated rlimit struct.
    let rc = unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) };
    if rc == 0 {
        return Ok(());
    }

    Err(CliError::Io(io::Error::last_os_error()))
}
