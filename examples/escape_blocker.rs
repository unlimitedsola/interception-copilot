//! Escape key blocker example using the Interception API
//!
//! This example demonstrates how to block specific keyboard events (escape key)
//! while allowing other keys to pass through normally. It takes a precedence
//! value from the command line to demo interaction with multiple applications.
//!
//! Usage: escape_blocker [precedence]
//! - precedence: Optional integer value for processing priority (default: 0)
//!   Higher values get processed earlier in the event chain.
//!
//! **Note**: This requires the Interception driver to be installed on Windows.

use interception::{Device, FILTER_KEY_ALL, Interception, KEY_UP};
use std::env;

/// Escape key scan code
const SCANCODE_ESC: u16 = 0x01;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments for precedence
    let args: Vec<String> = env::args().collect();
    let precedence = if args.len() > 1 {
        args[1].parse::<i32>().unwrap_or_else(|_| {
            eprintln!("Invalid precedence value '{}', using default 0", args[1]);
            0
        })
    } else {
        0
    };

    println!("Starting escape key blocker with precedence: {precedence}");
    println!("This application will block all escape key events.");
    println!("Press Ctrl+C to exit");

    // Create the high-level Interception instance which manages all devices
    let mut interception = Interception::new()?;

    // Set precedence and filter for keyboard devices
    interception.set_precedence(precedence)?;
    for device in interception.devices_mut() {
        if let Device::Keyboard(keyboard) = device {
            keyboard.set_filter(FILTER_KEY_ALL)?;
        }
    }

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
        let strokes = keyboard.receive(10)?;
        if !strokes.is_empty() {
            // Separate escape key events from other key events
            let mut other_strokes = Vec::new();

            for stroke in &strokes {
                if stroke.code == SCANCODE_ESC {
                    // This is an escape key event - block it and log
                    let key_action = if stroke.state & KEY_UP != 0 {
                        "UP"
                    } else {
                        "DOWN"
                    };
                    println!(
                        "BLOCKED: Escape key {key_action} event (precedence: {precedence}, device: {device_index:02})"
                    );
                    // Note: we don't add this stroke to other_strokes, effectively blocking it
                } else {
                    // This is a non-escape key - allow it to pass through
                    other_strokes.push(stroke.clone());
                }
            }

            // Send back all non-escape key strokes so they work normally
            if !other_strokes.is_empty() {
                keyboard.send(&other_strokes)?;
            }
        }
    }
}

#[cfg(not(windows))]
fn main() {
    println!("This example only works on Windows with the Interception driver installed.");
    println!("Current platform is not Windows.");
}
