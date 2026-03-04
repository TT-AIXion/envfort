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
