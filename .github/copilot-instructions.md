# Interception

Interception is a Rust port of the [Interception library](https://github.com/oblitum/Interception) using `windows-sys` with a safe API for intercepting keyboard and mouse input on Windows systems.

_Note: This repository is named "interception-copilot" to indicate it's a GitHub Copilot workspace, but the library itself is called "interception" - the same name as the original C implementation._

> Naming Policy: Outside of this `.github/copilot-instructions.md` file (and unavoidable occurrences in the repository name or remote URL), the word `copilot` MUST NOT appear in source code, documentation, commit messages, issue / PR titles or descriptions, binary names, or published crate metadata. Always refer to the library simply as `interception`. If you encounter an existing occurrence elsewhere, remove or rename it (except for the repository folder name itself) as a housekeeping fix.

**ALWAYS reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.**

## Maintaining and Improving These Instructions

### Correcting Outdated Information

**You are encouraged to correct any information in these instructions that is no longer accurate.** As the project evolves, dependencies may change, build processes may be updated, or new requirements may emerge. When you encounter information that doesn't match the current state of the project:

- Update version numbers, dependency names, or build commands that have changed
- Correct file paths, directory structures, or command syntax that no longer work
- Fix any outdated references to tools, APIs, or external resources
- Update timing estimates for builds, tests, or other operations if they no longer reflect reality

### Adding Important Information for Future Development

**When you discover important information that would be crucial for future work on this project, add it to these instructions.** This helps create a comprehensive knowledge base that benefits all future contributors:

- **New dependencies or setup requirements** that you had to discover through trial and error
- **Build gotchas or edge cases** that aren't immediately obvious but cause problems
- **API changes or compatibility issues** discovered while working with the Windows APIs
- **Performance considerations** or optimization techniques specific to this project
- **Testing approaches** or validation methods that prove useful
- **Cross-platform considerations** beyond the current Linux/Windows focus
- **Security considerations** related to low-level hardware access

### How to Update Instructions

When updating these instructions:

- Make changes directly to `.github/copilot-instructions.md`
- Keep the existing structure and formatting consistent
- Be specific and actionable in your additions
- Include code examples or commands where helpful
- Test any new build commands or procedures before adding them
- Consider the impact on both new and experienced developers working on the project

**Remember: These instructions are a living document that should evolve with the project.**

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
- `examples/keylogger.rs` - Example keyboard event logger using type-safe API
- `examples/mouse_capture.rs` - Example mouse event capture using type-safe API
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
- **NO BACKWARD COMPATIBILITY**: This library has not been released yet, so breaking changes are acceptable and backward compatibility is not maintained

## Common Tasks Reference

### Repository Structure

```
.
├── Cargo.toml          # Project configuration
├── src/lib.rs          # Main library code
├── examples/           # Usage examples
│   ├── keylogger.rs
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
