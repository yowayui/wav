use std::process::Command;

#[test]
fn devices_cli_runs() {
    let output = Command::new(env!("CARGO_BIN_EXE_wav"))
        .arg("devices")
        .output()
        .expect("failed to run wav devices");
    assert!(output.status.success());
}
