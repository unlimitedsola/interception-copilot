# Interception Copilot

Interception Copilot is a Rust port of the [Interception library](https://github.com/oblitum/Interception) using `windows-sys` with a safe API for intercepting keyboard and mouse input on Windows systems.

**ALWAYS reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.**

## Working Effectively

### Initial Setup - REQUIRED EVERY TIME

**CRITICAL**: This project is Windows-only but can be cross-compiled on Linux. You MUST install cross-compilation tools:

```bash
# Install MinGW-w64 cross-compiler (required for linking)
sudo apt update && sudo apt install -y gcc-mingw-w64-x86-64

# Install Windows target for Rust (required for compilation)  
rustup target add x86_64-pc-windows-gnu
```

**NEVER SKIP** these setup steps. All builds will fail without them.

### Building the Project

**NEVER CANCEL** build commands. Set timeouts appropriately:

```bash
# Debug build - takes ~1.5 seconds, NEVER CANCEL, set timeout to 60+ seconds
cargo build --target x86_64-pc-windows-gnu

# Release build - takes ~1.5 seconds, NEVER CANCEL, set timeout to 30+ seconds  
cargo build --target x86_64-pc-windows-gnu --release

# Build examples explicitly (if needed) - takes <1 second
cargo build --target x86_64-pc-windows-gnu --examples
```

### Code Quality and Validation

**ALWAYS** run these commands before finalizing changes. **NEVER CANCEL** any of them:

```bash
# Format check - takes <1 second
cargo fmt --all -- --check

# Linting - takes ~1.5 seconds, NEVER CANCEL, set timeout to 60+ seconds
cargo clippy --target x86_64-pc-windows-gnu --all-targets --all-features -- -D warnings

# Documentation generation - takes ~1 second
cargo doc --target x86_64-pc-windows-gnu --no-deps
```

### Testing Limitations - CRITICAL

**IMPORTANT**: Tests compile but CANNOT run on Linux because this library requires Windows-specific drivers and APIs.

```bash
# This will compile tests but execution will fail with "Exec format error"
cargo test --target x86_64-pc-windows-gnu
```

**DO NOT** attempt to fix test execution failures on Linux - they are expected. Tests can only run on Windows with the Interception driver installed.

## Reference Implementation

The `interception-c` folder contains the source code for the original C project which this Rust wrapper is based on.

**ALWAYS** use this as a reference for understanding the original implementation and functionality. The ported Rust code maintains the same functionality as the original C code, including the same memory layout for structs and ordering of fields.

### Key Reference Files
- `interception-c/library/interception.h` - Original C API definitions
- `interception-c/library/interception.c` - Original C implementation  
- `interception-c/samples/` - C example applications

## Code Structure

### Main Components
- `src/lib.rs` - Main library with safe Rust API wrapping Windows APIs
- `examples/basic_keylogger.rs` - Example keyboard event logger
- `examples/mouse_capture.rs` - Example mouse event capture and replay
- `interception-c/` - Original C implementation for reference

### Important Constants and Types
- Device limits: `INTERCEPTION_MAX_KEYBOARD` (10), `INTERCEPTION_MAX_MOUSE` (10)
- Key states: `KEY_DOWN`, `KEY_UP`, `KEY_E0`, `KEY_E1`
- Mouse states: `MOUSE_LEFT_BUTTON_DOWN`, `MOUSE_RIGHT_BUTTON_DOWN`, etc.
- Filters: `FILTER_KEY_ALL`, `FILTER_MOUSE_ALL`, etc.

## Validation Scenarios

### What You CAN Validate on Linux
- **Code compilation** for Windows target
- **Static analysis** with clippy
- **Code formatting** with rustfmt  
- **Documentation generation**
- **API consistency** with reference C implementation

### What You CANNOT Validate on Linux
- **Runtime functionality** - requires Windows + Interception driver + admin privileges
- **Hardware interaction** - needs real keyboard/mouse input
- **Driver communication** - needs Windows kernel driver installed

**ALWAYS** focus on compilation, static analysis, and API correctness. Do not attempt runtime testing.

## Code Style Requirements

- **DO NOT** use star imports (`use some_crate::*;`)
- **ALWAYS** use explicit imports
- **MAINTAIN** same struct memory layout as C reference implementation
- **FOLLOW** existing error handling patterns using `Result<T, InterceptionError>`

## Common Tasks Reference

### Repository Structure
```
.
├── Cargo.toml          # Project configuration
├── src/lib.rs          # Main library code  
├── examples/           # Usage examples
│   ├── basic_keylogger.rs
│   └── mouse_capture.rs
├── interception-c/     # Reference C implementation
│   ├── library/        # Core C library
│   └── samples/        # C example applications
└── target/             # Build artifacts (gitignored)
```

### Build Artifacts Location
- Debug binaries: `target/x86_64-pc-windows-gnu/debug/`
- Release binaries: `target/x86_64-pc-windows-gnu/release/`
- Examples: `target/x86_64-pc-windows-gnu/debug/examples/`
- Documentation: `target/x86_64-pc-windows-gnu/doc/`

### Dependencies Summary
- `windows-sys` v0.60.2 - Windows API bindings
- MinGW-w64 - Cross-compilation toolchain
- Rust x86_64-pc-windows-gnu target

## Troubleshooting

### "could not find `cc`" Error
```bash
sudo apt install -y gcc-mingw-w64-x86-64
```

### "error: Microsoft Visual C++ is required" 
This indicates you're not using the GNU target. Always use:
```bash
--target x86_64-pc-windows-gnu
```

### Tests Failing with "Exec format error"
This is **EXPECTED** on Linux. Windows executables cannot run on Linux.

### Missing Target Error
```bash
rustup target add x86_64-pc-windows-gnu
```

**FINAL REMINDER**: This is a Windows-specific hardware input interception library. All builds must target Windows, and runtime testing requires Windows environment with appropriate drivers and permissions.
