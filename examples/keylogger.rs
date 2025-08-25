//! Keyboard event logger example using the high-level Interception API
//!
//! This example demonstrates how to use the high-level Interception API
//! to capture keyboard events efficiently using event-driven waiting.
//!
//! **Note**: This requires the Interception driver to be installed on Windows.

use interception::{Device, FILTER_KEY_ALL, Interception, KEY_UP, KeyStroke};
use std::ffi::OsString;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting keyboard event logger...");
    println!("Press Ctrl+C to exit");

    // Create the high-level Interception instance which manages all devices
    let mut interception = Interception::new()?;

    // Set filter to capture all keyboard events on keyboard devices
    for device in interception.devices_mut() {
        if let Device::Keyboard(keyboard) = device {
            keyboard.set_filter(FILTER_KEY_ALL)?;
        }
    }

    println!("Device hardware IDs:");
    for (i, device) in interception.devices_mut().iter_mut().enumerate() {
        let hw_id = device
            .get_hardware_id()
            .unwrap_or_else(|_| OsString::from("Unknown"));
        match device {
            Device::Keyboard(_) => {
                println!("  Keyboard {i}: {hw_id:?}");
            }
            Device::Mouse(_) => {
                println!("  Mouse {i}: {hw_id:?}");
            }
        }
    }

    let mut strokes = [KeyStroke::default(); 10];

    // Main event loop
    loop {
        // Wait for any device to have input available
        let device_index = interception.wait_index(None)?;
        let device = &mut interception.devices_mut()[device_index];

        let Device::Keyboard(keyboard) = device else {
            // If the device is not a keyboard, continue to the next iteration
            continue;
        };

        // Try to receive keyboard strokes from the device
        let strokes = keyboard.receive(&mut strokes)?;
        if !strokes.is_empty() {
            for stroke in strokes.iter() {
                let key_action = if stroke.state & KEY_UP != 0 {
                    "UP"
                } else {
                    "DOWN"
                };
                println!(
                    "{:02}: {:>3} {:<4} (code: 0x{:02X}, state: 0x{:02X}, info: 0x{:08X})",
                    device_index,
                    stroke.code,
                    key_action,
                    stroke.code,
                    stroke.state,
                    stroke.information,
                );
            }

            // Send the strokes back so they still work normally
            keyboard.send(strokes)?;
        }
    }
}

#[cfg(not(windows))]
fn main() {
    println!("This example only works on Windows with the Interception driver installed.");
    println!("Current platform is not Windows.");
}
