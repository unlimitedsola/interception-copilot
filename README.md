# Interception

A Rust port of the [Interception library](https://github.com/oblitum/Interception) for intercepting keyboard and mouse input on Windows systems.

## Overview

Interception provides a low-level interface for intercepting and modifying keyboard and mouse input at the driver level on Windows. This Rust implementation maintains full compatibility with the original C library while providing modern safety guarantees.

## Features

- **Low-level input interception** - Intercept keyboard and mouse events at the driver level
- **Real-time input modification** - Modify or block input events in real-time
- **Safe Rust API** - Memory-safe wrapper around Windows APIs
- **Driver installer included** - Self-contained installer for required Windows drivers

## Components

### Main Library

The core library provides functions for:
- Setting up device contexts and filters
- Receiving keyboard and mouse events
- Sending modified events back to the system
- Managing device priorities and filters

### Driver Installer

A comprehensive installer utility that handles driver installation and setup automatically:

```bash
# Install Interception drivers
interception-installer install

# Remove Interception drivers  
interception-installer uninstall
```

## Requirements

- Windows system (Windows XP or later)
- Administrator privileges for driver installation
- System reboot required after driver installation

## Installation

1. Download the latest release
2. Run the installer with administrator privileges:
   ```bash
   interception-installer install
   ```
3. Reboot your system
4. You can now use applications that depend on Interception

## Usage Example

```rust
use interception::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = InterceptionContext::new()?;
    
    // Set up keyboard filter
    context.set_filter(
        Device::Keyboard(1),
        Filter::KeyFilter::All
    )?;
    
    loop {
        if let Some(device) = context.wait()? {
            let stroke = context.receive(device)?;
            
            // Process the input stroke
            // ... your input processing logic here ...
            
            // Send the stroke back to the system
            context.send(device, stroke)?;
        }
    }
}
```

## License

This project is dual-licensed under LGPL-3.0-only for non-commercial use. See the original [Interception project](https://github.com/oblitum/Interception) for commercial licensing options.

## Reference

Based on the original [Interception library](https://github.com/oblitum/Interception) by Francisco Lopes.