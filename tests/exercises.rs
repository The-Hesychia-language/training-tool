//! Every exercise fails as it is given, and passes with its solution. Needs `hsy`.

use std::process::Command;

#[test]
fn exercises_fail_and_solutions_pass() {
    let output = Command::new(env!("CARGO_BIN_EXE_hsy-train"))
        .arg("verify")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("hsy-train runs");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{text}{}", String::from_utf8_lossy(&output.stderr));
}
