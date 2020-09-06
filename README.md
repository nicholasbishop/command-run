# command-run

[![crates.io](https://img.shields.io/crates/v/command-run.svg)](https://crates.io/crates/command-run)
[![Documentation](https://docs.rs/command-run/badge.svg)](https://docs.rs/command-run)

Rust library for running a command in a subprocess.

This library is a thin wrapper around the [`std::process::Command`]
type with a few additional convenient features:

- Print and/or log the command before running it
- Optionally return an error if the command is not successful
- The command can be formatted as a command-line string
- The [`Command`] type can be cloned and its fields are public

## Dependencies and features

Other than the standard library, this crate has only one dependency:
the [`log`] crate. That dependency can be disabled:

```toml
command-run = { version = "*", default-features = false }
```

## Example

```rust
// This will return an error if the command did not exit successfully
// (controlled with the `check` field).
let output = Command::with_args("echo", &["hello", "world"])
    .enable_capture()
    .run()?;
assert_eq!(output.stdout_string_lossy(), "hello world\n");
```

[`log`]: https://crates.io/crates/log
[`std::process::Command`]: https://doc.rust-lang.org/std/process/struct.Command.html
[`Command`]: https://docs.rs/command-run/latest/command_run/struct.Command.html
