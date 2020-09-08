use command_run::{Command, LogTo};
use log::{Level, LevelFilter, Metadata, Record};
use once_cell::sync::OnceCell;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
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

struct TestProg {
    command: Command,
    tmpdir: TempDir,
}

impl TestProg {
    fn new() -> TestProg {
        let tmpdir = TempDir::new().unwrap();

        // Write the test code to a temporary directory
        let code = include_str!("testprog.rs");
        let code_path = tmpdir.path().join("testprog.rs");
        fs::write(&code_path, code).unwrap();

        // Build the test program
        let prog_path = tmpdir.path().join("testprog");
        Command::new("rustc")
            .add_arg("-o")
            .add_args(&[&prog_path, &code_path])
            .run()
            .unwrap();

        TestProg {
            command: Command::new(&prog_path),
            tmpdir,
        }
    }
}

#[test]
fn test_combine_output() {
    let mut testprog = TestProg::new();
    testprog.command.capture = true;
    testprog.command.combine_output = true;

    let output = testprog.command.run().unwrap();
    assert_eq!(output.stdout_string_lossy(), "test-stdout\ntest-stderr\n");
    assert_eq!(output.stderr_string_lossy(), "");
}

#[derive(Debug, Default)]
struct CapturedLogs {
    logs: Arc<Mutex<Vec<(Level, String)>>>,
}

impl CapturedLogs {
    fn records(&self) -> Vec<(Level, String)> {
        self.logs.lock().unwrap().clone()
    }
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

static LOGGER: Logger = Logger {};
static CAPTURED_LOGS: OnceCell<CapturedLogs> = OnceCell::new();

#[cfg(feature = "logging")]
#[test]
fn test_log() {
    CAPTURED_LOGS.set(CapturedLogs::default()).unwrap();
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Info))
        .unwrap();

    let mut testprog = TestProg::new();
    testprog.command.capture = true;
    testprog.command.log_command = true;
    testprog.command.log_output_on_error = true;
    testprog.command.log_to = LogTo::Log;

    let _output = testprog.command.run().unwrap();

    assert_eq!(
        CAPTURED_LOGS.get().unwrap().records(),
        vec![(
            Level::Info,
            testprog
                .tmpdir
                .path()
                .join("testprog")
                .display()
                .to_string()
        )]
    );

    // TODO: need to make this program (optionally?) exit non-zero
}
