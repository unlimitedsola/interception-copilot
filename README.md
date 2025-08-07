# Interception Copilot

[![Crates.io](https://img.shields.io/crates/v/interception-copilot.svg)](https://crates.io/crates/interception-copilot)
[![Documentation](https://docs.rs/interception-copilot/badge.svg)](https://docs.rs/interception-copilot)
[![License](https://img.shields.io/crates/l/interception-copilot.svg)](https://github.com/unlimitedsola/interception-copilot/blob/main/LICENSE)

A Rust port of the [Interception library](https://github.com/oblitum/Interception) using `windows-sys` with a safe API for intercepting keyboard and mouse input on Windows.

The Interception library allows you to intercept and modify keyboard and mouse input at a low level on Windows systems. This Rust port provides both unsafe bindings to the original C API and safe wrappers for convenient use.

## Features

- 🦀 **Pure Rust** - No external C dependencies, uses `windows-sys`
- 🛡️ **Safe API** - Memory-safe wrappers around low-level Windows APIs
- 🎯 **Low-level access** - Direct access to keyboard and mouse input streams
- 📦 **Easy to use** - Simple, idiomatic Rust API
- 🔄 **Event filtering** - Granular control over which events to intercept
- 🖱️ **Complete coverage** - Full support for keyboards, mice, and their extended features

## Prerequisites

This library requires the **Interception driver** to be installed on the Windows system where your application will run.

1. Download the Interception driver from the [official repository](https://github.com/oblitum/Interception)
2. Run the installer as Administrator: `install-interception.exe /install`
3. Reboot your system

**Note**: The Interception driver is a low-level system component that requires Administrator privileges to install and run.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
interception-copilot = "0.1"
```

## Quick Start

```rust
use interception_copilot::{Context, InterceptionFilter, is_keyboard_device};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an interception context
    let context = Context::new()?;
    
    // Set filter to capture all keyboard input
    context.set_filter(is_keyboard_device, InterceptionFilter::KEY_ALL)?;
    
    // Wait for keyboard events and process them
    loop {
        if let Some(device) = context.wait() {
            // Receive strokes from the device
            let strokes = context.receive(device, 10)?;
            
            // Process the strokes...
            for stroke in &strokes {
                unsafe {
                    println!("Key event: code={}, state={}", 
                        stroke.key.code, stroke.key.state);
                }
            }
            
            // Send strokes back to continue normal operation
            context.send(device, &strokes)?;
        }
    }
}
```

## Examples

### Basic Keyboard Logger

```rust
use interception_copilot::{Context, InterceptionFilter, is_keyboard_device};

let context = Context::new()?;
context.set_filter(is_keyboard_device, InterceptionFilter::KEY_ALL)?;

loop {
    if let Some(device) = context.wait() {
        let strokes = context.receive(device, 10)?;
        
        for stroke in &strokes {
            unsafe {
                let key_stroke = stroke.key;
                let action = if key_stroke.state & 0x01 != 0 { "UP" } else { "DOWN" };
                println!("Key {}: code={}", action, key_stroke.code);
            }
        }
        
        // Forward the input
        context.send(device, &strokes)?;
    }
}
```

### Mouse Event Capture

```rust
use interception_copilot::{Context, InterceptionFilter, is_mouse_device};

let context = Context::new()?;
context.set_filter(is_mouse_device, InterceptionFilter::MOUSE_ALL)?;

loop {
    if let Some(device) = context.wait() {
        let strokes = context.receive(device, 10)?;
        
        for stroke in &strokes {
            unsafe {
                let mouse_stroke = stroke.mouse;
                println!("Mouse: pos=({}, {}), state=0x{:04X}", 
                    mouse_stroke.x, mouse_stroke.y, mouse_stroke.state);
            }
        }
        
        context.send(device, &strokes)?;
    }
}
```

### Sending Synthetic Input

```rust
use interception_copilot::{Context, KeyStroke, Stroke, keyboard};

let context = Context::new()?;

// Send a synthetic 'A' key press
let key_down = Stroke::from(KeyStroke::down(0x1E)); // 'A' key scan code
let key_up = Stroke::from(KeyStroke::up(0x1E));

context.send(keyboard(0), &[key_down])?;
std::thread::sleep(std::time::Duration::from_millis(50));
context.send(keyboard(0), &[key_up])?;
```

## API Overview

### Core Types

- **`Context`** - Main interception context for managing devices
- **`Device`** - Device identifier (keyboard or mouse)
- **`KeyStroke`** - Represents a keyboard event
- **`MouseStroke`** - Represents a mouse event  
- **`Stroke`** - Union type for keyboard or mouse events
- **`InterceptionFilter`** - Event filtering constants

### Key Functions

- **`Context::new()`** - Create a new interception context
- **`Context::wait()`** - Wait for input from any device
- **`Context::receive()`** - Receive input strokes from a device
- **`Context::send()`** - Send strokes to a device
- **`Context::set_filter()`** - Set event filters for devices

### Device Utilities

- **`keyboard(index)`** - Get keyboard device by index (0-9)
- **`mouse(index)`** - Get mouse device by index (0-9)
- **`is_keyboard_device(device)`** - Check if device is a keyboard
- **`is_mouse_device(device)`** - Check if device is a mouse

## Error Handling

The library provides comprehensive error handling through the `InterceptionError` enum:

```rust
use interception_copilot::{Context, InterceptionError};

match Context::new() {
    Ok(context) => {
        // Use the context...
    },
    Err(InterceptionError::CreateFile(code)) => {
        eprintln!("Failed to create device file: {}", code);
    },
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Filtering Events

You can filter which types of events to intercept:

```rust
// Capture only key down events
context.set_filter(is_keyboard_device, InterceptionFilter::KEY_DOWN)?;

// Capture only mouse button events (no movement)
context.set_filter(is_mouse_device, 
    InterceptionFilter::MOUSE_LEFT_BUTTON_DOWN | 
    InterceptionFilter::MOUSE_LEFT_BUTTON_UP)?;

// Capture everything
context.set_filter(|_| true, InterceptionFilter::KEY_ALL)?;
```

## Thread Safety

The `Context` struct is `Send` but not `Sync`. If you need to use it across multiple threads, wrap it in appropriate synchronization primitives like `Arc<Mutex<Context>>`.

## Performance Considerations

- The library performs memory allocation for stroke buffers on each receive/send operation
- For high-frequency applications, consider batch processing of strokes
- The underlying Windows APIs are efficient, but intercepting all events can impact system performance

## Limitations

- **Windows only** - This library only works on Windows systems
- **Driver required** - Requires the Interception driver to be installed
- **Administrator privileges** - Applications using this library typically need to run as Administrator
- **System-wide impact** - Intercepted events affect the entire system until forwarded

## Security Considerations

This library provides powerful low-level access to system input. Applications using it:

- Should be carefully audited for security vulnerabilities
- May be flagged by antivirus software as potentially unwanted
- Should implement proper access controls and logging
- Must handle user privacy and data protection appropriately

## License

This project is licensed under the GNU Lesser General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

This is the same license as the original Interception library to maintain compatibility.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. Make sure to:

1. Follow Rust coding conventions
2. Add tests for new functionality
3. Update documentation as needed
4. Test on Windows with the Interception driver installed

## Acknowledgments

- [Francisco Lopes (oblitum)](https://github.com/oblitum) for the original [Interception library](https://github.com/oblitum/Interception)
- The Rust community for `windows-sys` and related tooling
- Microsoft for the Windows API documentation

## Related Projects

- [Original Interception Library (C)](https://github.com/oblitum/Interception)
- [windows-rs](https://github.com/microsoft/windows-rs) - Rust for Windows
- [winapi-rs](https://github.com/retep998/winapi-rs) - Alternative Windows API bindings