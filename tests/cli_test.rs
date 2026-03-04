use std::process::Command;

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
