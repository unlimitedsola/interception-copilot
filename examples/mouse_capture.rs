//! Mouse event capture using the high-level Interception API
//!
//! This example demonstrates how to use the high-level Interception API
//! to capture and send mouse events efficiently using event-driven waiting.
//!
//! **Note**: This requires the Interception driver to be installed on Windows.

use interception::{Device, FILTER_MOUSE_ALL, Interception, MouseStroke};

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Mouse event capture using high-level API");
    println!("Move your mouse and click buttons to see events");
    println!("Press Ctrl+C to exit");

    // Create the high-level Interception instance which manages all devices
    let mut interception = Interception::new()?;

    // Set filter to capture all keyboard events on keyboard devices
    for device in interception.devices_mut() {
        if let Device::Mouse(mouse) = device {
            mouse.set_filter(FILTER_MOUSE_ALL)?;
        }
    }

    println!("Device hardware IDs:");
    for (i, device) in interception.devices_mut().iter_mut().enumerate() {
        let hw_id = device
            .get_hardware_id()
            .unwrap_or_else(|_| String::from("Unknown"));
        match device {
            Device::Keyboard(_) => {
                println!("  Keyboard {i}: {hw_id}");
            }
            Device::Mouse(_) => {
                println!("  Mouse {i}: {hw_id}");
            }
        }
    }

    let mut strokes = [MouseStroke::default(); 10];

    // Main event loop
    loop {
        // Wait for any device to have input available
        let device_index = interception.wait_index(None)?;
        let device = &mut interception.devices_mut()[device_index];

        let Device::Mouse(mouse) = device else {
            // If the device is not a keyboard, continue to the next iteration
            continue;
        };

        // Try to receive keyboard strokes from the device
        let strokes = mouse.receive(&mut strokes)?;
        if !strokes.is_empty() {
            for stroke in strokes.iter() {
                println!(
                    "{:02}: pos=({}, {}), state=0x{:04X}, flags=0x{:04X}, rolling={}",
                    device_index, stroke.x, stroke.y, stroke.state, stroke.flags, stroke.rolling
                );
            }

            // Send the strokes back so they still work normally
            mouse.send(strokes)?;
        }
    }
}

#[cfg(not(windows))]
fn main() {
    println!("This example only works on Windows with the Interception driver installed.");
    println!("Current platform is not Windows.");
}
