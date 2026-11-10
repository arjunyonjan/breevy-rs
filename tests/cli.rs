use std::process::Command;

fn sample() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/sample.bvy")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_breevy-rs"))
        .arg(sample())
        .args(args)
        .output()
        .expect("failed to run breevy-rs")
}

#[test]
fn lists_every_snippet() {
    let out = run(&["--list"]);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 8);
    assert!(stdout.contains("addr"));
}

#[test]
fn matches_abbreviation_with_trigger() {
    let out = run(&["--match", "addr", "--trigger"]);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.starts_with("match: addr\n"), "got: {stdout}");
    assert!(stdout.contains("expansion:\n"));
}

#[test]
fn unknown_abbreviation_exits_failure() {
    let out = run(&["--match", "zzz", "--trigger"]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.trim(), "no match");
}
