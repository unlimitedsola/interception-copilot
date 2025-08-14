# Interception - Rust Port

This repository contains a Rust port of the [Interception library](https://github.com/oblitum/Interception) using `windows-sys` with a safe API for intercepting keyboard and mouse input on Windows systems.

## Repository Structure

- **`src/`** - Main Interception library (Rust port)
- **`installer/`** - Driver installer utility
- **`examples/`** - Usage examples for the main library
- **`interception-c/`** - Original C implementation (reference)

## Components

### Main Library (`src/`)

The main library provides a safe Rust API for intercepting keyboard and mouse input on Windows. It maintains the same functionality as the original C implementation while providing Rust safety guarantees.

### Driver Installer (`installer/`)

A comprehensive driver installer that handles:

- Automatic Windows version and architecture detection
- Driver file installation to system directories
- Windows registry service configuration
- Device class filter setup
- Complete uninstallation support

**Usage:**
```bash
# Install drivers
interception-installer install

# Uninstall drivers  
interception-installer uninstall
```

**Requirements:**
- Administrator privileges
- Windows system with compatible driver files
- System reboot after installation/uninstallation

## Building

This is a Windows-only library but can be cross-compiled on Linux:

```bash
# Install cross-compilation tools (Ubuntu/Debian)
sudo apt install gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu

# Build main library
cargo build --target x86_64-pc-windows-gnu

# Build installer
cargo build --target x86_64-pc-windows-gnu -p interception-installer

# Build release versions
cargo build --target x86_64-pc-windows-gnu --release
```

## Testing

**Important:** Tests compile but cannot run on Linux since this library requires Windows-specific drivers and APIs. Testing must be done on Windows systems with the Interception driver installed.

```bash
# Compile tests (Linux/Windows)
cargo test --target x86_64-pc-windows-gnu --no-run

# Run tests (Windows only, requires admin privileges)
cargo test --target x86_64-pc-windows-gnu
```

## Code Quality

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --target x86_64-pc-windows-gnu --all-targets --all-features -- -D warnings

# Generate documentation
cargo doc --target x86_64-pc-windows-gnu --no-deps
```

## Driver Files

The installer includes driver files for multiple Windows versions and architectures:

- **Windows XP** (5.1) - drivers with prefix `51`
- **Windows Server 2003** (5.2) - drivers with prefix `52` 
- **Windows Vista** (6.0) - drivers with prefix `60`
- **Windows 7** (6.1) - drivers with prefix `61`
- **Windows 8/8.1/10/11** - uses Windows 7 drivers (`61`)

Each version supports x86, x64, and IA-64 architectures where applicable.

## License

This project is dual-licensed under LGPL-3.0-only for non-commercial use. See the original [Interception project](https://github.com/oblitum/Interception) for commercial licensing options.

## Reference Implementation  

The `interception-c/` directory contains the original C implementation for reference. The Rust port maintains API compatibility and identical functionality.