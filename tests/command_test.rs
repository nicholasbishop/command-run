#[cfg(feature = "logging")]
mod capture_logger {
    use log::{Level, LevelFilter, Metadata, Record};
    use once_cell::sync::OnceCell;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct CapturedLogs {
        logs: Arc<Mutex<Vec<(Level, String)>>>,
    }

    struct Logger {}

    impl log::Log for Logger {
        fn enabled(&self, _: &Metadata) -> bool {
            true
        }

        fn log(&self, record: &Record) {
            CAPTURED_LOGS
                .get()
                .unwrap()
                .logs
                .lock()
                .unwrap()
                .push((record.level(), record.args().to_string()));
        }

        fn flush(&self) {}
    }

    // TODO: can these be combined?
    static LOGGER: Logger = Logger {};
    static CAPTURED_LOGS: OnceCell<CapturedLogs> = OnceCell::new();

    pub fn init() {
        CAPTURED_LOGS.set(CapturedLogs::default()).unwrap();
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(LevelFilter::Info))
            .unwrap();
    }

    pub fn get_logs() -> Vec<(Level, String)> {
        CAPTURED_LOGS.get().unwrap().logs.lock().unwrap().clone()
    }

    pub fn clear_logs() {
        CAPTURED_LOGS.get().unwrap().logs.lock().unwrap().clear();
    }
}

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
fn test_args() -> Result<(), anyhow::Error> {
    let out = Command::with_args("echo", &["hello", "world"])
        .enable_capture()
        .run()?;
    assert_eq!(out.stdout, b"hello world\n");
    Ok(())
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
        Command::with_args("a", &["-_/,:.="]).command_line_lossy(),
        "a -_/,:.="
    );
}

struct TestProg {
    command: Command,

    // Allow dead_code when the "logging" feature is off
    #[allow(dead_code)]
    tmpdir: TempDir,
}

impl TestProg {
    fn new() -> Result<TestProg, anyhow::Error> {
        let tmpdir = TempDir::new()?;

        // Write the test code to a temporary directory
        let code = include_str!("testprog.rs");
        let code_path = tmpdir.path().join("testprog.rs");
        fs::write(&code_path, code)?;

        // Build the test program
        let prog_path = tmpdir.path().join("testprog");
        Command::new("rustc")
            .add_arg("-o")
            .add_args(&[&prog_path, &code_path])
            .run()?;

        Ok(TestProg {
            command: Command::new(&prog_path),
            tmpdir,
        })
    }

    // Allow dead_code when the "logging" feature is off
    #[allow(dead_code)]
    fn path(&self) -> String {
        self.tmpdir.path().join("testprog").display().to_string()
    }
}

#[test]
fn test_combine_output() -> Result<(), anyhow::Error> {
    let mut testprog = TestProg::new()?;
    testprog.command.capture = true;
    testprog.command.combine_output = true;
    testprog.command.check = false;

    let output = testprog.command.run().unwrap();
    assert_eq!(output.stdout_string_lossy(), "test-stdout\ntest-stderr\n");
    assert_eq!(output.stderr_string_lossy(), "");

    Ok(())
}

#[cfg(feature = "logging")]
#[test]
fn test_log() -> Result<(), anyhow::Error> {
    use command_run::LogTo;
    use log::Level;

    capture_logger::init();

    let mut testprog = TestProg::new()?;
    testprog.command.capture = true;
    testprog.command.log_command = true;
    testprog.command.log_output_on_error = true;
    testprog.command.log_to = LogTo::Log;

    assert!(testprog.command.run().unwrap_err().is_exit_error());

    assert_eq!(
        capture_logger::get_logs(),
        vec![
            (Level::Info, testprog.path()),
            (
                Level::Error,
                format!(
                    "command '{}' failed: exit status: 1
stdout:
test-stdout

stderr:
test-stderr
",
                    testprog.path()
                )
            )
        ]
    );

    // Re-run the command with combined output
    capture_logger::clear_logs();
    testprog.command.combine_output = true;
    assert!(testprog.command.run().unwrap_err().is_exit_error());
    assert_eq!(
        capture_logger::get_logs(),
        vec![
            (Level::Info, testprog.path()),
            (
                Level::Error,
                format!(
                    "command '{}' failed: exit status: 1
output:
test-stdout
test-stderr
",
                    testprog.path()
                )
            )
        ]
    );

    Ok(())
}
