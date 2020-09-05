#![deny(missing_docs)]

//! Utility for running a command in a subprocess.
//!
//! The `Command` type is a wrapper around the `std::process::Command`
//! type that adds a few convenient features:
//!
//! - Print and/or log the command before running it
//! - Optionally return an error if the command is not successful
//! - The command can be formatted as a command-line string
//! - The `Command` type can be cloned

use log::info;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::{fmt, io, process};

/// Type of error.
#[derive(Debug)]
pub enum ErrorKind {
    /// The command failed to launch (e.g. if the executable does not
    /// exist.)
    Launch(io::Error),

    /// The command exited non-zero or due to a signal.
    Exit(process::ExitStatus),
}

/// Error returned by `Command::run`.
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

/// A command to run in a subprocess and options for how it is run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Command {
    /// Executable path.
    ///
    /// The path can be just a file name, in which case the $PATH is
    /// searched.
    pub executable: PathBuf,

    /// Arguments passed to the executable.
    pub args: Vec<OsString>,

    /// Directory from which to run the executable.
    ///
    /// If not set (the default), the current working directory is
    /// used.
    pub dir: Option<PathBuf>,

    /// If true, log the command before running it. The default is
    /// false.
    pub log_command: bool,

    /// If true (the default), print the command to stdout before
    /// running it.
    pub print_command: bool,

    /// If true (the default), check if the command exited
    /// successfully and return an error if not.
    pub check: bool,

    /// If true (the default), capture the stdout and stderr of the
    /// command.
    pub capture: bool,

    /// If false (the default), inherit environment variables from the
    /// current process.
    pub clear_env: bool,

    /// Add or update environment variables in the child process.
    pub env: HashMap<OsString, OsString>,
}

impl Command {
    /// Make a new Command with the given executable.
    ///
    /// All other fields are set to the defaults.
    pub fn new<S: AsRef<OsStr>>(executable: S) -> Command {
        Command {
            executable: executable.as_ref().into(),
            ..Default::default()
        }
    }

    /// Run the command.
    ///
    /// If capture is true, the command's output (stdout and stderr)
    /// is returned along with the status. If not, the stdout and
    /// stderr are empty.
    ///
    /// If the command fails to start an error is returned. If check
    /// is set, an error is also returned if the command exits
    /// non-zero or due to a signal.
    ///
    /// If log_command and/or print_command is true then the command
    /// line is logged and/or printed before running it. If the
    /// command fails the error is not logged or printed, but the
    /// resulting error type implements Display and can be used for
    /// this purpose.
    pub fn run(&self) -> Result<process::Output, Error> {
        let cmd_str = self.command_line_lossy();
        if self.log_command {
            info!("{}", cmd_str);
        }
        if self.print_command {
            println!("{}", cmd_str);
        }

        let mut cmd: process::Command = self.into();
        let out = if self.capture {
            cmd.output().map_err(|err| Error {
                command: self.clone(),
                kind: ErrorKind::Launch(err),
            })?
        } else {
            let status = cmd.status().map_err(|err| Error {
                command: self.clone(),
                kind: ErrorKind::Launch(err),
            })?;
            process::Output {
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
    pub fn command_line(&self) -> OsString {
        // TODO: add some quoting, e.g. if an arg has a space in it
        let mut out = OsString::new();
        out.push(&self.executable);

        for arg in &self.args {
            out.push(" ");
            out.push(arg);
        }
        out
    }

    /// Format as a space-separated command line and convert to a string.
    pub fn command_line_lossy(&self) -> String {
        String::from_utf8_lossy(self.command_line().as_bytes()).into()
    }
}

impl Default for Command {
    fn default() -> Self {
        Command {
            executable: PathBuf::new(),
            args: Vec::new(),
            dir: None,
            log_command: false,
            print_command: true,
            check: true,
            capture: true,
            clear_env: false,
            env: HashMap::new(),
        }
    }
}

impl From<&Command> for process::Command {
    fn from(cmd: &Command) -> Self {
        let mut out = process::Command::new(&cmd.executable);
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
        let mut cmd = Command {
            executable: Path::new("true").into(),
            ..Default::default()
        };
        assert!(cmd.run().is_ok());

        // Check, exit non-zero
        cmd.executable = Path::new("false").into();
        assert!(cmd.run().unwrap_err().is_exit_error());

        // No check
        cmd.check = false;
        assert!(cmd.run().is_ok());
    }
}
