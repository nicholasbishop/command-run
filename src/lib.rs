#![deny(missing_docs)]

//! Utility for running a command in a subprocess.
//!
//! The [`Command`] type is a wrapper around the [`std::process::Command`]
//! type that adds a few convenient features:
//!
//! - Print and/or log the command before running it
//! - Optionally return an error if the command is not successful
//! - The command can be formatted as a command-line string
//! - The [`Command`] type can be cloned and its fields are public
//!
//! [`Command`]: struct.Command.html
//! [`std::process::Command`]: https://doc.rust-lang.org/std/process/struct.Command.html

use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::{fmt, io, process};

/// Type of error.
#[derive(Debug)]
pub enum ErrorKind {
    /// The command failed to launch (e.g. if the program does not
    /// exist).
    Launch(io::Error),

    /// The command exited non-zero or due to a signal.
    Exit(process::ExitStatus),
}

/// Error returned by [`Command::run`].
///
/// [`Command::run`]: struct.Command.html#method.run
#[derive(Debug)]
pub struct Error {
    /// The command that caused the error.
    pub command: Command,

    /// The type of error.
    pub kind: ErrorKind,
}

impl Error {
    /// Check if the error kind is `Launch`.
    pub fn is_launch_error(&self) -> bool {
        matches!(self.kind, ErrorKind::Launch(_))
    }

    /// Check if the error kind is `Exit`.
    pub fn is_exit_error(&self) -> bool {
        matches!(self.kind, ErrorKind::Exit(_))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match &self.kind {
            ErrorKind::Launch(err) => write!(
                f,
                "failed to launch '{}': {}",
                self.command.command_line_lossy(),
                err
            ),
            ErrorKind::Exit(err) => write!(
                f,
                "command '{}' failed: {}",
                self.command.command_line_lossy(),
                err
            ),
        }
    }
}

impl std::error::Error for Error {}

/// The output of a finished process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Output {
    /// The status (exit code) of the process.
    pub status: process::ExitStatus,

    /// The data that the process wrote to stdout.
    pub stdout: Vec<u8>,

    /// The data that the process wrote to stderr.
    pub stderr: Vec<u8>,
}

impl Output {
    /// Get stdout as a string.
    pub fn stdout_string_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.stdout)
    }

    /// Get stderr as a string.
    pub fn stderr_string_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.stderr)
    }
}

impl From<process::Output> for Output {
    fn from(o: process::Output) -> Output {
        Output {
            status: o.status,
            stdout: o.stdout,
            stderr: o.stderr,
        }
    }
}

/// A command to run in a subprocess and options for how it is run.
///
/// Some notable trait implementations:
/// - Derives `Clone`, `Debug`, `Eq`, and `PartialEq`
/// - `Default` (see docstrings for each field for what the
///   corresponding default is)
/// - `From<&Command> for std::process::Command` to convert to a
///   [`std::process::Command`]
///
/// [`std::process::Command`]: https://doc.rust-lang.org/std/process/struct.Command.html
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Command {
    /// Program path.
    ///
    /// The path can be just a file name, in which case the `$PATH` is
    /// searched.
    pub program: PathBuf,

    /// Arguments passed to the program.
    pub args: Vec<OsString>,

    /// Directory from which to run the program.
    ///
    /// If not set (the default), the current working directory is
    /// used.
    pub dir: Option<PathBuf>,

    /// If `true`, log the command before running it. The default is
    /// false. This does nothing if the "logging" feature is not
    /// enabled.
    pub log_command: bool,

    /// If `true` (the default), print the command to stdout before
    /// running it.
    pub print_command: bool,

    /// If `true` (the default), check if the command exited
    /// successfully and return an error if not.
    pub check: bool,

    /// If `true`, capture the stdout and stderr of the
    /// command. The default is `false`.
    pub capture: bool,

    /// If `false` (the default), inherit environment variables from the
    /// current process.
    pub clear_env: bool,

    /// Add or update environment variables in the child process.
    pub env: HashMap<OsString, OsString>,
}

impl Command {
    /// Make a new Command with the given program.
    ///
    /// All other fields are set to the defaults.
    pub fn new<S: AsRef<OsStr>>(program: S) -> Command {
        Command {
            program: program.as_ref().into(),
            ..Default::default()
        }
    }

    /// Make a new Command with the given program and args.
    ///
    /// All other fields are set to the defaults.
    pub fn with_args<I, S1, S2>(program: S1, args: I) -> Command
    where
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>,
        I: IntoIterator<Item = S2>,
    {
        Command {
            program: program.as_ref().into(),
            args: args.into_iter().map(|arg| arg.as_ref().into()).collect(),
            ..Default::default()
        }
    }

    /// Append a single argument.
    pub fn add_arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.args.push(arg.as_ref().into());
        self
    }

    /// Append two arguments.
    ///
    /// This is equivalent to calling `add_arg` twice; it is for the
    /// common case where the arguments have different types, e.g. a
    /// literal string for the first argument and a `Path` for the
    /// second argument.
    pub fn add_arg_pair<S1, S2>(&mut self, arg1: S1, arg2: S2) -> &mut Self
    where
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>,
    {
        self.add_arg(arg1);
        self.add_arg(arg2);
        self
    }

    /// Append multiple arguments.
    pub fn add_args<I, S>(&mut self, args: I) -> &mut Self
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = S>,
    {
        for arg in args {
            self.add_arg(arg);
        }
        self
    }

    /// Set capture to `true`.
    pub fn enable_capture(&mut self) -> &mut Self {
        self.capture = true;
        self
    }

    /// Run the command.
    ///
    /// If `capture` is `true`, the command's output (stdout and
    /// stderr) is returned along with the status. If not, the stdout
    /// and stderr are empty.
    ///
    /// If the command fails to start an error is returned. If check
    /// is set, an error is also returned if the command exits
    /// non-zero or due to a signal.
    ///
    /// If `log_command` and/or `print_command` is true then the
    /// command line is logged and/or printed before running it. If
    /// the command fails the error is not logged or printed, but the
    /// resulting error type implements `Display` and can be used for
    /// this purpose.
    pub fn run(&self) -> Result<Output, Error> {
        let cmd_str = self.command_line_lossy();
        #[cfg(feature = "logging")]
        if self.log_command {
            log::info!("{}", cmd_str);
        }
        if self.print_command {
            println!("{}", cmd_str);
        }

        let mut cmd: process::Command = self.into();
        let out = if self.capture {
            cmd.output()
                .map_err(|err| Error {
                    command: self.clone(),
                    kind: ErrorKind::Launch(err),
                })?
                .into()
        } else {
            let status = cmd.status().map_err(|err| Error {
                command: self.clone(),
                kind: ErrorKind::Launch(err),
            })?;
            Output {
                stdout: Vec::new(),
                stderr: Vec::new(),
                status,
            }
        };
        if self.check && !out.status.success() {
            return Err(Error {
                command: self.clone(),
                kind: ErrorKind::Exit(out.status),
            });
        }
        Ok(out)
    }

    /// Format as a space-separated command line.
    ///
    /// The program path and the arguments are converted to strings
    /// with [`String::from_utf8_lossy`].
    ///
    /// If any component contains characters that are not ASCII
    /// alphanumeric or in the set `/-,:.`, the component is
    /// quoted with `'` (single quotes). This is both too aggressive
    /// (unnecessarily quoting things that don't need to be quoted)
    /// and incorrect (e.g. a single quote will itself be quoted with
    /// a single quote). This method is mostly intended for logging
    /// though, and it should work reasonably well for that.
    ///
    /// [`String::from_utf8_lossy`]: https://doc.rust-lang.org/std/string/struct.String.html#method.from_utf8_lossy
    pub fn command_line_lossy(&self) -> String {
        fn convert_word<S: AsRef<OsStr>>(word: S) -> String {
            fn char_requires_quoting(c: char) -> bool {
                if c.is_ascii_alphanumeric() {
                    return false;
                }
                let allowed_chars = "/-,:.";
                !allowed_chars.contains(c)
            }

            let s =
                String::from_utf8_lossy(word.as_ref().as_bytes()).to_string();
            if s.chars().any(char_requires_quoting) {
                format!("'{}'", s)
            } else {
                s
            }
        }

        let mut out = convert_word(&self.program);
        for arg in &self.args {
            out.push(' ');
            out.push_str(&convert_word(arg));
        }
        out
    }
}

impl Default for Command {
    fn default() -> Self {
        Command {
            program: PathBuf::new(),
            args: Vec::new(),
            dir: None,
            log_command: false,
            print_command: true,
            check: true,
            capture: false,
            clear_env: false,
            env: HashMap::new(),
        }
    }
}

impl From<&Command> for process::Command {
    fn from(cmd: &Command) -> Self {
        let mut out = process::Command::new(&cmd.program);
        out.args(&cmd.args);
        if let Some(dir) = &cmd.dir {
            out.current_dir(dir);
        }
        if cmd.clear_env {
            out.env_clear();
        }
        out.envs(&cmd.env);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
            Command::with_args("test", &["hello", "world"])
                .command_line_lossy(),
            "test hello world"
        );

        assert_eq!(
            Command::with_args("a b", &["c d", "e"]).command_line_lossy(),
            "'a b' 'c d' e"
        );

        // Check that some special characters do not cause quoting
        assert_eq!(
            Command::with_args("a", &["-/,:"]).command_line_lossy(),
            "a -/,:"
        );
    }
}
