//! Keyboard event logger example using the high-level Interception API
//!
//! This example demonstrates how to use the high-level Interception API
//! to capture keyboard events efficiently using event-driven waiting.
//!
//! **Note**: This requires the Interception driver to be installed on Windows.

use interception::{Device, FILTER_KEY_ALL, Interception};
use std::time::Duration;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting keyboard event logger using high-level API...");
    println!("Press Ctrl+C to exit");

    // Create the high-level Interception instance which manages all devices
    let mut interception = Interception::new()?;

    println!(
        "Created Interception instance with {} devices",
        interception.devices().len()
    );

    // Set filter to capture all keyboard events on keyboard devices
    let devices = interception.devices_mut();
    let mut active_keyboards = 0;
    for (i, device) in devices.iter_mut().enumerate() {
        if let Device::Keyboard(keyboard) = device {
            match keyboard.set_filter(FILTER_KEY_ALL) {
                Ok(_) => {
                    active_keyboards += 1;
                    println!("Set filter for keyboard device {i}");
                }
                Err(e) => eprintln!("Failed to set filter for keyboard device {i}: {e}"),
            }
        }
    }

    if active_keyboards == 0 {
        println!("No keyboard devices could be configured. Is the Interception driver installed?");
        return Ok(());
    }

    println!("Configured {active_keyboards} keyboard device(s)");

    // Print hardware IDs for all devices on startup
    println!("Device hardware IDs:");
    let devices = interception.devices_mut();
    for (i, device) in devices.iter_mut().enumerate() {
        match device.get_hardware_id() {
            Ok(hardware_id) => {
                let device_type = if i < 10 { "keyboard" } else { "mouse" };
                let device_index = if i < 10 { i } else { i - 10 };
                // Convert bytes to UTF-16 string if possible, otherwise hex dump
                let hardware_str = if hardware_id.len() >= 2 && hardware_id.len() % 2 == 0 {
                    let u16_chars: Vec<u16> = hardware_id
                        .chunks_exact(2)
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect();
                    String::from_utf16_lossy(&u16_chars)
                        .trim_end_matches('\0')
                        .to_string()
                } else {
                    format!(
                        "0x{}",
                        hardware_id
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    )
                };
                println!("  Device {i} ({device_type} {device_index}): {hardware_str}");
            }
            Err(e) => {
                let device_type = if i < 10 { "keyboard" } else { "mouse" };
                let device_index = if i < 10 { i } else { i - 10 };
                println!(
                    "  Device {i} ({device_type} {device_index}): Error getting hardware ID - {e}"
                );
            }
        }
    }

    // Main event loop using efficient waiting
    loop {
        // Wait for any device to have input available (with 100ms timeout)
        match interception.wait(Some(Duration::from_millis(100))) {
            Ok(device) => {
                // Check if this is a keyboard device
                if let Device::Keyboard(keyboard) = device {
                    // Try to receive keyboard strokes from the device
                    match keyboard.receive(10) {
                        Ok(strokes) => {
                            if !strokes.is_empty() {
                                for stroke in &strokes {
                                    let key_action = if stroke.state & 0x01 != 0 {
                                        "UP"
                                    } else {
                                        "DOWN"
                                    };
                                    println!(
                                        "Key {}: {} (code: 0x{:02X}, state: 0x{:02X}, info: 0x{:08X})",
                                        key_action,
                                        stroke.code,
                                        stroke.code,
                                        stroke.state,
                                        stroke.information
                                    );
                                }

                                // Send the strokes back so they still work normally
                                keyboard.send(&strokes)?;
                            }
                        }
                        Err(e) => eprintln!("Error receiving strokes: {e}"),
                    }
                }
                // If it's a mouse device, we ignore it in this keyboard-only example
            }
            Err(interception::InterceptionError::Wait(interception::WaitError::WaitTimeout)) => {
                // Timeout - continue loop (this allows for graceful shutdown on Ctrl+C)
                continue;
            }
            Err(e) => {
                eprintln!("Error waiting for input: {e}");
                return Err(e.into());
            }
        }
    }
}

#[cfg(not(windows))]
fn main() {
    println!("This example only works on Windows with the Interception driver installed.");
    println!("Current platform is not Windows.");
}
