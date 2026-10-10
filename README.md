# rshell

A POSIX-inspired Unix shell written in Rust.

Built as a learning project to explore how shells work under the hood: lexing, pipelines, process management, and interactive line editing.

## Features

- **Command execution**: builtins and executables found via `$PATH`
- **Pipelines**: chain commands with `|`
- **Redirection**: `>`, `>>`, `2>`, `2>>` (and `1>`, `1>>`)
- **Background jobs**: run commands with `&` and track their status
- **Quoting and escaping**: single quotes, double quotes, and backslash escapes
- **Variable expansion**: `$VAR` and `${VAR}`
- **Tab completion**: builtins, `$PATH` commands, files and directories
- **Programmable completion**: external completion scripts per command, via `COMP_LINE` and `COMP_POINT`
- **Command history**: loaded from and saved to the file set in `$HISTFILE`

## Getting started

### Requirements

- A Unix-like system (Linux or macOS)
- **Windows users:** use [WSL](https://learn.microsoft.com/windows/wsl/install) (Windows Subsystem for Linux). This project was developed and tested on Windows through WSL, and it does not run natively on Windows.
- Rust 1.88 or newer ([install Rust](https://rustup.rs)), installed inside WSL if you are on Windows

### Build and run

```sh
git clone https://github.com/alxatnsv/rshell.git
cd rshell
cargo run --release
```

## Usage

```sh
$ echo "hello world" | tr a-z A-Z
HELLO WORLD
$ ls > files.txt
$ sleep 5 &
$ exit
```

## Project structure

| File                    | Purpose                                         |
| ----------------------- | ----------------------------------------------- |
| `main.rs`               | REPL loop and history handling                  |
| `lexer.rs`              | State-machine tokenizer (quotes, escapes, vars) |
| `pipeline.rs`           | Splits tokens into pipeline stages              |
| `io_context.rs`         | Parses redirection operators                    |
| `streams.rs`            | Input/output abstractions for commands          |
| `job.rs`                | Background job tracking                         |
| `helper.rs`             | Tab completion via `rustyline`                  |
| `completion_context.rs` | Parses the cursor position for completion       |
| `shell.rs`              | Builtins and command execution                  |

## Limitations

This is not a complete POSIX shell. Currently unsupported:

- Special variables such as `$?` and `$$`
- Command substitution and globbing
- Control flow (`if`, `for`, `while`) and scripting
- Redirection inside pipeline stages

## Acknowledgements

Started from the [CodeCrafters "Build Your Own Shell"](https://codecrafters.io/challenges/shell) challenge.
