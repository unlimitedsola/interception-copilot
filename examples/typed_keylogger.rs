//! Typed keyboard event logger example using the new idiomatic API
//!
//! This example demonstrates how to use the new KeyboardDevice type
//! to capture keyboard events in a type-safe manner.
//!
//! **Note**: This requires the Interception driver to be installed on Windows.

use interception::{FILTER_KEY_ALL, KeyboardDevice};

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting typed keyboard event logger...");
    println!("Press Ctrl+C to exit");

    // Create keyboard devices for first 3 keyboards
    let mut keyboards: Vec<KeyboardDevice> = Vec::new();
    for i in 0..3 {
        if let Ok(kb) = KeyboardDevice::new(i) {
            keyboards.push(kb);
        }
    }

    if keyboards.is_empty() {
        println!("No keyboard devices could be created. Is the Interception driver installed?");
        return Ok(());
    }

    println!("Created {} keyboard device(s)", keyboards.len());

    // Set filter to capture all keyboard events on each device
    for kb in &keyboards {
        kb.set_filter(FILTER_KEY_ALL)?;
    }

    // Main event loop
    loop {
        // Check each keyboard device for input
        for (device_index, keyboard) in keyboards.iter().enumerate() {
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
                                "Keyboard {} - Key {}: {} (state: 0x{:02X}, info: 0x{:08X})",
                                device_index,
                                key_action,
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

        // Small delay to avoid busy loop
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(not(windows))]
fn main() {
    println!("This example only works on Windows with the Interception driver installed.");
    println!("Current platform is not Windows.");
}
