#![deny(missing_docs)]

//! Utility for running a command in a subprocess.
//!
//! The [`Command`] type is a wrapper around the [`std::process::Command`]
//! type that adds a few convenient features:
//!
//! - Print or log the command before running it
//! - Optionally return an error if the command is not successful
//! - The command can be formatted as a command-line string
//! - The [`Command`] type can be cloned and its fields are public

use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::{fmt, io, process};

/// Type of error.
#[derive(Debug)]
pub enum ErrorKind {
    /// An error occurred in the calls used to run the command. For
    /// example, this variant is used if the program does not exist.
    Run(io::Error),

    /// The command exited non-zero or due to a signal.
    Exit(process::ExitStatus),
}

/// Error returned by [`Command::run`].
#[derive(Debug)]
pub struct Error {
    /// The command that caused the error.
    pub command: Command,

    /// The type of error.
    pub kind: ErrorKind,
}

impl Error {
    /// Check if the error kind is `Run`.
    pub fn is_run_error(&self) -> bool {
        matches!(self.kind, ErrorKind::Run(_))
    }

    /// Check if the error kind is `Exit`.
    pub fn is_exit_error(&self) -> bool {
        matches!(self.kind, ErrorKind::Exit(_))
    }
}

/// Internal trait for converting an io::Error to an Error.
trait IntoError<T> {
    fn into_run_error(self, command: &Command) -> Result<T, Error>;
}

impl<T> IntoError<T> for Result<T, io::Error> {
    fn into_run_error(self, command: &Command) -> Result<T, Error> {
        self.map_err(|err| Error {
            command: command.clone(),
            kind: ErrorKind::Run(err),
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match &self.kind {
            ErrorKind::Run(err) => write!(
                f,
                "failed to run '{}': {}",
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

fn combine_output(mut cmd: process::Command) -> Result<Output, io::Error> {
    let (mut reader, writer) = os_pipe::pipe()?;
    let writer_clone = writer.try_clone()?;
    cmd.stdout(writer);
    cmd.stderr(writer_clone);

    let mut handle = cmd.spawn()?;

    drop(cmd);

    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    let status = handle.wait()?;

    Ok(Output {
        stdout: output,
        stderr: Vec::new(),
        status,
    })
}

/// Where log messages go.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogTo {
    /// Print to stdout.
    Stdout,

    /// Use the standard `log` crate.
    #[cfg(feature = "logging")]
    Log,
}

/// A command to run in a subprocess and options for how it is run.
///
/// Some notable trait implementations:
/// - Derives `Clone`, `Debug`, `Eq`, and `PartialEq`
/// - `Default` (see docstrings for each field for what the
///   corresponding default is)
/// - `From<&Command> for std::process::Command` to convert to a
///   [`std::process::Command`]
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

    /// Where log messages go. The default is stdout.
    pub log_to: LogTo,

    /// If `true` (the default), log the command before running it.
    pub log_command: bool,

    /// If `true`, log the output if the command exits non-zero or due
    /// to a signal. This does nothing is `capture` is `false` or if
    /// `check` is `false`. The default is `false`.
    pub log_output_on_error: bool,

    /// If `true` (the default), check if the command exited
    /// successfully and return an error if not.
    pub check: bool,

    /// If `true`, capture the stdout and stderr of the
    /// command. The default is `false`.
    pub capture: bool,

    /// If `true`, send stderr to stdout; the `stderr` field in
    /// `Output` will be empty. The default is `false.`
    pub combine_output: bool,

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

    /// Set `capture` to `true`.
    pub fn enable_capture(&mut self) -> &mut Self {
        self.capture = true;
        self
    }

    /// Set `combine_output` to `true`.
    pub fn combine_output(&mut self) -> &mut Self {
        self.combine_output = true;
        self
    }

    /// Set the directory from which to run the program.
    pub fn set_dir<S: AsRef<OsStr>>(&mut self, dir: S) -> &mut Self {
        self.dir = Some(dir.as_ref().into());
        self
    }

    /// Set `check` to `false`.
    pub fn disable_check(&mut self) -> &mut Self {
        self.check = false;
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
    /// If `log_command` is `true` then the command line is logged
    /// before running it. If the command fails the error is not
    /// logged or printed, but the resulting error type implements
    /// `Display` and can be used for this purpose.
    pub fn run(&self) -> Result<Output, Error> {
        let cmd_str = self.command_line_lossy();
        if self.log_command {
            match self.log_to {
                LogTo::Stdout => println!("{}", cmd_str),

                #[cfg(feature = "logging")]
                LogTo::Log => log::info!("{}", cmd_str),
            }
        }

        let mut cmd: process::Command = self.into();
        let out = if self.capture {
            if self.combine_output {
                combine_output(cmd).into_run_error(self)?
            } else {
                cmd.output().into_run_error(self)?.into()
            }
        } else {
            let status = cmd.status().into_run_error(self)?;
            Output {
                stdout: Vec::new(),
                stderr: Vec::new(),
                status,
            }
        };
        if self.check && !out.status.success() {
            if self.capture && self.log_output_on_error {
                let mut msg =
                    format!("command '{}' failed: {}", cmd_str, out.status);
                if self.combine_output {
                    msg = format!(
                        "{}\noutput:\n{}",
                        msg,
                        out.stdout_string_lossy()
                    );
                } else {
                    msg = format!(
                        "{}\nstdout:\n{}\nstderr:\n{}",
                        msg,
                        out.stdout_string_lossy(),
                        out.stderr_string_lossy()
                    );
                }
                match self.log_to {
                    LogTo::Stdout => println!("{}", msg),

                    #[cfg(feature = "logging")]
                    LogTo::Log => log::error!("{}", msg),
                }
            }

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
    /// alphanumeric or in the set `/-,:.=`, the component is
    /// quoted with `'` (single quotes). This is both too aggressive
    /// (unnecessarily quoting things that don't need to be quoted)
    /// and incorrect (e.g. a single quote will itself be quoted with
    /// a single quote). This method is mostly intended for logging
    /// though, and it should work reasonably well for that.
    pub fn command_line_lossy(&self) -> String {
        fn convert_word<S: AsRef<OsStr>>(word: S) -> String {
            fn char_requires_quoting(c: char) -> bool {
                if c.is_ascii_alphanumeric() {
                    return false;
                }
                let allowed_chars = "/-,:.=";
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
            log_to: LogTo::Stdout,
            log_command: true,
            log_output_on_error: false,
            check: true,
            capture: false,
            combine_output: false,
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
