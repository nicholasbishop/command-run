# command-run

Rust library for running a command in a subprocess.

This library is a thin wrapper around the `std::process::Command`
type with a few additional convenient features:

- Print and/or log the command before running it
- Optionally return an error if the command is not successful
- The command can be formatted as a command-line string
- The `Command` type can be cloned

Example:

```rust
let cmd = Command::new("my-command");
// This will return an error if the command did not exit successfully
// (controlled with the `check` field). The output is captured by
// default (controlled by the `capture` field).
let output = cmd.run()?;
```
