# RVN

A Rust-based TUI for managing Python virtual environments.

## What it does

RVN helps you manage existing virtual environments with a simple terminal interface. You can:

- list available venvs recursively scanned from execution dir
- activate a selected venv
- deactivate the current environment
- delete venvs
- display basic venv info

## Highlights

- **Rust-powered** for speed and reliability
- **Terminal user interface** designed for interactive use
- **Easy switching between multiple environments**

## Navigation

- **Use arrows or 'j/k' for up/down navigation**.
- **Press enter or 'a' to activate selected environment.**
- **Inside virtual environment use 'exit' to quit back to RVN TUI**
- **Press 'd' to flag selected environment for deletion, on confirm page prompt 'y/Y' or 'n/N' for confirmation**
- **Press 'q' to exit the TUI**

- Tooltips inside TUI coming soon...  

## Installation

- To build locally, make sure Rust is installed,cd into project root then run 

```console
cargo build --release
```

This builds the executable under /target/release/

Or use the installer script, which pulls the latest release from GitHub

```console
curl -sSL https://raw.githubusercontent.com/balint0513/rvn/main/install.sh | bash
```

## Status

Work in progress...
