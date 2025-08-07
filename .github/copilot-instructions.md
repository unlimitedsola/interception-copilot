## Reference Implementation

The `interception-c` folder contains the source code for the original project which this Rust wrapper is based on.

Use this as a reference for understanding the original implementation and functionality.

The ported Rust code should maintain the same functionality as the original C code, this including the same memory
layout for structs and ordering of fields. Proper tests should be written to ensure that the memory layout of the
structs matches the original C code.

## Windows Related Instructions

This project is Windows only, however, you will be working on a Linux machine.
To ensure you get correct error reports and diagnostics, you should follow these steps:

1. You should use a Rust toolchain that allows you to build for Windows on Linux.
   You can do this by installing the `x86_64-pc-windows-gnu` target.
2. You should use the `x86_64-pc-windows-gnu` toolchain for all your builds and tests.
   You can set this as the default toolchain by running:
   ```bash
   rustup default stable-x86_64-pc-windows-gnu
   ```

## Code Style

- Do not use star imports.

## Finalizing Code Changes

When you have completed your code changes, please ensure that you follow the steps below to maintain code quality and
consistency across the project.

Perform the following tasks before finalizing your work:

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all-targets --all-features`

Make sure to resolve any issues that arise from these commands. If you encounter warnings or errors, address them before
proceeding.
