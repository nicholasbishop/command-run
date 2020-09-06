use command_run::{Command, Error};

#[test]
fn test_example() -> Result<(), Error> {
    // Begin readme example
    let mut cmd = Command::with_args("echo", &["hello", "world"]);
    cmd.capture = true;
    // This will return an error if the command did not exit successfully
    // (controlled with the `check` field).
    let output = cmd.run()?;
    assert_eq!(output.stdout_string_lossy(), "hello world\n");
    // End readme example
    Ok(())
}
