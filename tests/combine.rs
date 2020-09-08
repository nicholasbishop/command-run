use command_run::Command;
use std::fs;
use tempfile::TempDir;

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
