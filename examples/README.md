# Interception Examples

This directory contains examples demonstrating various features of the Interception library.

## Examples

### precedence_demo.rs
Demonstrates the precedence functionality of the Interception library. 

**Precedence** determines the order in which input events are processed when multiple devices have input available simultaneously. Higher precedence values are processed first.

Key features demonstrated:
- Setting custom precedence values for individual devices
- Getting precedence values from devices
- Setting precedence for all devices globally via the Interception instance
- Live demonstration showing how precedence affects input processing order

Usage: The example sets different precedence values for different input devices and shows how this affects the order of event processing during live input.

### keylogger.rs
A keyboard event logger demonstrating basic keyboard input capture using the high-level Interception API.

### mouse_capture.rs
A mouse event capture example demonstrating mouse input capture and synthetic event generation using the high-level Interception API.

## Building Examples

All examples are built for the Windows target since the Interception library is Windows-specific:

```bash
cargo build --target x86_64-pc-windows-gnu --examples
```

## Running Examples

These examples can only run on Windows with the Interception driver installed and require administrator privileges.

**Note**: The examples are cross-compiled for Windows from Linux but cannot be executed in the Linux environment. They need to be transferred to a Windows system with the appropriate driver and privileges to run.