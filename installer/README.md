# Interception Driver Installer

This crate provides a command-line installer for the Interception library drivers on Windows systems.

## Features

- Automatic Windows version and architecture detection
- Driver file extraction and installation to system directory
- Registry service configuration for keyboard and mouse drivers
- Device class filter setup (UpperFilters registry entries)
- Complete uninstallation support
- Administrator privilege validation

## Usage

**Important**: This installer requires administrator privileges to modify system files and registry entries.

### Installation

```cmd
interception-installer install
```

### Uninstallation

```cmd
interception-installer uninstall
```

## How It Works

### Installation Process

1. **System Detection**: Detects Windows version (XP through Windows 10/11) and architecture (x86, x64, IA-64)
2. **Driver Selection**: Chooses the appropriate driver files based on system configuration:
   - Keyboard: `KBDNT<version><arch>.sys` → `keyboard.sys`
   - Mouse: `MOUNT<version><arch>.sys` → `mouse.sys`
3. **File Installation**: Copies driver files to `C:\Windows\System32\drivers\`
4. **Service Registration**: Creates Windows services in the registry:
   - Service entries in `HKLM\SYSTEM\CurrentControlSet\Services\`
   - Sets appropriate service parameters (Type, ErrorControl, Start, ImagePath)
5. **Class Filter Setup**: Adds drivers to device class UpperFilters:
   - Keyboard class: `{4d36e96b-e325-11ce-bfc1-08002be10318}`
   - Mouse class: `{4d36e96f-e325-11ce-bfc1-08002be10318}`

### Uninstallation Process

1. **Registry Cleanup**: Removes service entries and class filters
2. **File Removal**: Deletes driver files from system directory

## Supported Windows Versions

The installer automatically detects and supports:

- Windows XP (5.1) - uses driver version 51
- Windows Server 2003 (5.2) - uses driver version 52
- Windows Vista (6.0) - uses driver version 60
- Windows 7 (6.1) - uses driver version 61
- Windows 8/8.1/10/11 - uses driver version 61 (Windows 7 drivers)

## Architecture Support

- x86 (32-bit)
- x64/AMD64 (64-bit)
- IA-64 (Itanium)

## Error Handling

The installer provides detailed error messages for:

- System detection failures
- Missing driver files
- File system access issues
- Registry modification problems
- Permission denied scenarios

## Reboot Requirement

**Important**: After installation or uninstallation, a system reboot is required for the changes to take effect.

## Building

```bash
# Cross-compile for Windows from Linux
cargo build --target x86_64-pc-windows-gnu --release
```

## Dependencies

- `windows-sys` - Windows API bindings for registry and system operations