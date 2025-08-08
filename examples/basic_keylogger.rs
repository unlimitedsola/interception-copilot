//! Basic keyboard event logger example
//!
//! This example demonstrates how to use the interception-copilot library
//! to capture keyboard events. It will log all key presses and releases.
//!
//! **Note**: This requires the Interception driver to be installed on Windows.
//! This example will only work on Windows systems.

use interception_copilot::{Context, Filter, is_keyboard_device};

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting keyboard event logger...");
    println!("Press Ctrl+C to exit");

    // Create an interception context
    let context = Context::new()?;

    // Set filter to capture all keyboard events
    context.set_filter(is_keyboard_device, Filter::KEY_ALL)?;

    // Main event loop
    loop {
        // Wait for any device to have input available
        if let Some(device) = context.wait() {
            if is_keyboard_device(device) {
                // Receive keyboard strokes from the device
                match context.receive(device, 10) {
                    Ok(strokes) => {
                        for stroke in &strokes {
                            unsafe {
                                let key_stroke = stroke.key;
                                let key_action = if key_stroke.state & 0x01 != 0 {
                                    "UP"
                                } else {
                                    "DOWN"
                                };
                                println!(
                                    "Key {}: {} (state: 0x{:02X}, info: 0x{:08X})",
                                    key_action,
                                    key_stroke.code,
                                    key_stroke.state,
                                    key_stroke.information
                                );
                            }
                        }

                        // Send the strokes back so they still work normally
                        context.send(device, &strokes)?;
                    }
                    Err(e) => eprintln!("Error receiving strokes: {e}"),
                }
            }
        }
    }
}

#[cfg(not(windows))]
fn main() {
    println!("This example only works on Windows with the Interception driver installed.");
    println!("Current platform is not Windows.");
}
