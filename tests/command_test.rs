use command_run::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_check() {
    // Check, exit zero
    let mut cmd = Command::new("true");
    assert!(cmd.run().is_ok());

    // Check, exit non-zero
    cmd.program = Path::new("false").into();
    assert!(cmd.run().unwrap_err().is_exit_error());

    // No check
    cmd.check = false;
    assert!(cmd.run().is_ok());
}

#[test]
fn test_args() {
    let out = Command::with_args("echo", &["hello", "world"])
        .enable_capture()
        .run()
        .unwrap();
    assert_eq!(out.stdout, b"hello world\n");
}

#[test]
fn test_add_arg_variations() {
    let mut cmd = Command::new("a");
    cmd.add_arg("b");
    cmd.add_arg_pair("c", Path::new("d"));
    cmd.add_args(&["e", "f", "g"]);
    assert_eq!(cmd.command_line_lossy(), "a b c d e f g");
}

#[test]
fn test_command_line() {
    assert_eq!(Command::new("test").command_line_lossy(), "test");
    assert_eq!(
        Command::with_args("test", &["hello", "world"]).command_line_lossy(),
        "test hello world"
    );

    assert_eq!(
        Command::with_args("a b", &["c d", "e"]).command_line_lossy(),
        "'a b' 'c d' e"
    );

    // Check that some special characters do not cause quoting
    assert_eq!(
        Command::with_args("a", &["-/,:.="]).command_line_lossy(),
        "a -/,:.="
    );
}

#[test]
fn test_combine_output() {
    let tmpdir = TempDir::new().unwrap();

    // Build the test program
    let code = include_str!("testprog.rs");
    let code_path = tmpdir.path().join("testprog.rs");
    fs::write(&code_path, code).unwrap();

    let prog_path = tmpdir.path().join("testprog");
    Command::new("rustc")
        .add_arg("-o")
        .add_args(&[&prog_path, &code_path])
        .run()
        .unwrap();

    let output = Command::new(&prog_path)
        .combine_output()
        .enable_capture()
        .run()
        .unwrap();
    assert_eq!(output.stdout_string_lossy(), "test-stdout\ntest-stderr\n");
    assert_eq!(output.stderr_string_lossy(), "");
}
