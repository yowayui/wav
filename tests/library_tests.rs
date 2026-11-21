// Integration tests run against the compiled binary via std::process::Command.
// Substantive integration tests added in Phase D when library scanning is complete.

#[test]
fn binary_prints_help() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wav"))
        .arg("--help")
        .output()
        .expect("failed to run wav");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wav"),
        "help output should mention binary name"
    );
}

#[test]
fn binary_prints_version() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wav"))
        .arg("--version")
        .output()
        .expect("failed to run wav");
    assert!(output.status.success());
}
