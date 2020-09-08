# command-run

[![crates.io](https://img.shields.io/crates/v/command-run.svg)](https://crates.io/crates/command-run)
[![Documentation](https://docs.rs/command-run/badge.svg)](https://docs.rs/command-run)

Rust library for running a command in a subprocess.

This library is a thin wrapper around the [`std::process::Command`]
type with a few additional convenient features:

- Print or log the command before running it
- Optionally return an error if the command is not successful
- Optionally combine stdout and stderr
- Optionally print the command's output if the command fails
- The command can be formatted as a command-line string
- The [`Command`] type can be cloned and its fields are public

## Dependencies and features

- `log` - this is an optional dependency. It can be disabled by
  turning off the `logging` feature:

  ```toml
  command-run = { version = "*", default-features = false }
  ```

- `os_pipe` - this dependency is used to implement `combine_output`.
  
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
