use std::process::Command;

#[test]
fn playlist_list_cli() {
    let output = Command::new(env!("CARGO_BIN_EXE_wav"))
        .arg("playlist")
        .arg("list")
        .output()
        .expect("failed to run wav playlist list");
    assert!(output.status.success());
}
