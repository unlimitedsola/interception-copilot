//! Precedence demonstration example using the Interception API
//!
//! This example demonstrates how to use the precedence feature to control
//! the order in which input events are processed from multiple devices.
//!
//! Precedence determines the priority order when multiple devices have input
//! available simultaneously. Higher precedence values get processed first.
//!
//! **Note**: This requires the Interception driver to be installed on Windows.

use interception::{Device, FILTER_KEY_ALL, FILTER_MOUSE_ALL, Interception};
use std::time::Duration;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Interception Precedence Demo ===");
    println!("This example demonstrates how precedence affects input processing order.");
    println!("Press Ctrl+C to exit\n");

    // Create the high-level Interception instance
    let mut interception = Interception::new()?;

    println!(
        "Created Interception instance with {} devices",
        interception.devices().len()
    );

    // First, let's examine the default precedence values for all devices
    println!("\n--- Default Precedence Values ---");
    let devices = interception.devices_mut();
    for (i, device) in devices.iter_mut().enumerate() {
        match device.get_precedence() {
            Ok(precedence) => {
                let device_type = if i < 10 { "keyboard" } else { "mouse" };
                let device_index = if i < 10 { i } else { i - 10 };
                println!("  Device {i} ({device_type} {device_index}): precedence = {precedence}");
            }
            Err(e) => {
                println!("  Device {i}: Error getting precedence - {e}");
            }
        }
    }

    // Demonstrate setting different precedence values for different devices
    println!("\n--- Setting Custom Precedence Values ---");
    let devices = interception.devices_mut();

    // Set higher precedence for first keyboard (device 0)
    if let Device::Keyboard(keyboard) = &mut devices[0] {
        match keyboard.set_precedence(100) {
            Ok(_) => println!("  Set keyboard 0 precedence to 100 (highest priority)"),
            Err(e) => println!("  Failed to set keyboard 0 precedence: {e}"),
        }
    }

    // Set medium precedence for second keyboard (device 1) if available
    if let Device::Keyboard(keyboard) = &mut devices[1] {
        match keyboard.set_precedence(50) {
            Ok(_) => println!("  Set keyboard 1 precedence to 50 (medium priority)"),
            Err(e) => println!("  Failed to set keyboard 1 precedence: {e}"),
        }
    }

    // Set lower precedence for first mouse (device 10)
    if let Device::Mouse(mouse) = &mut devices[10] {
        match mouse.set_precedence(10) {
            Ok(_) => println!("  Set mouse 0 precedence to 10 (lower priority)"),
            Err(e) => println!("  Failed to set mouse 0 precedence: {e}"),
        }
    }

    // Verify the precedence values were set correctly
    println!("\n--- Verifying Custom Precedence Values ---");
    let devices = interception.devices_mut();
    for i in [0, 1, 10] {
        if let Some(device) = devices.get_mut(i) {
            match device.get_precedence() {
                Ok(precedence) => {
                    let device_type = if i < 10 { "keyboard" } else { "mouse" };
                    let device_index = if i < 10 { i } else { i - 10 };
                    println!(
                        "  Device {i} ({device_type} {device_index}): precedence = {precedence}"
                    );
                }
                Err(e) => {
                    println!("  Device {i}: Error getting precedence - {e}");
                }
            }
        }
    }

    // Set up filters for input capture to demonstrate precedence in action
    println!("\n--- Setting Up Input Filters ---");
    let devices = interception.devices_mut();
    let mut active_keyboards = 0;
    let mut active_mice = 0;

    for (i, device) in devices.iter_mut().enumerate() {
        match device {
            Device::Keyboard(keyboard) => match keyboard.set_filter(FILTER_KEY_ALL) {
                Ok(_) => {
                    active_keyboards += 1;
                    println!("  Set filter for keyboard device {i}");
                }
                Err(e) => println!("  Failed to set filter for keyboard device {i}: {e}"),
            },
            Device::Mouse(mouse) => {
                // Only set filter for first mouse to reduce noise
                if i == 10 {
                    match mouse.set_filter(FILTER_MOUSE_ALL) {
                        Ok(_) => {
                            active_mice += 1;
                            println!("  Set filter for mouse device {i}");
                        }
                        Err(e) => println!("  Failed to set filter for mouse device {i}: {e}"),
                    }
                }
            }
        }
    }

    if active_keyboards == 0 && active_mice == 0 {
        println!("No devices could be configured. Is the Interception driver installed?");
        return Ok(());
    }

    println!(
        "\nConfigured {active_keyboards} keyboard device(s) and {active_mice} mouse device(s)"
    );

    // Demonstrate global precedence setting
    println!("\n--- Demonstrating Global Precedence Setting ---");
    println!("Setting all devices to precedence 25...");
    match interception.set_precedence(25) {
        Ok(_) => println!("Successfully set precedence to 25 for all devices"),
        Err(e) => println!("Failed to set global precedence: {e}"),
    }

    // Verify global precedence was set
    println!("Verifying global precedence setting:");
    let devices = interception.devices_mut();
    for i in [0, 1, 10] {
        if let Some(device) = devices.get_mut(i) {
            match device.get_precedence() {
                Ok(precedence) => {
                    let device_type = if i < 10 { "keyboard" } else { "mouse" };
                    let device_index = if i < 10 { i } else { i - 10 };
                    println!(
                        "  Device {i} ({device_type} {device_index}): precedence = {precedence}"
                    );
                }
                Err(e) => {
                    println!("  Device {i}: Error getting precedence - {e}");
                }
            }
        }
    }

    // Restore custom precedence values for demonstration
    println!("\n--- Restoring Custom Precedence for Demo ---");
    let devices = interception.devices_mut();

    if let Device::Keyboard(keyboard) = &mut devices[0] {
        keyboard.set_precedence(100).ok();
        println!("  Restored keyboard 0 to precedence 100 (highest)");
    }

    if let Device::Keyboard(keyboard) = &mut devices[1] {
        keyboard.set_precedence(50).ok();
        println!("  Restored keyboard 1 to precedence 50 (medium)");
    }

    if let Device::Mouse(mouse) = &mut devices[10] {
        mouse.set_precedence(10).ok();
        println!("  Restored mouse 0 to precedence 10 (lowest)");
    }

    // Main event loop demonstrating precedence in action
    println!("\n--- Live Precedence Demonstration ---");
    println!("Start typing on different keyboards and moving the mouse.");
    println!("Notice how events from higher precedence devices are processed first.");
    println!("Device precedence order: keyboard 0 (100) > keyboard 1 (50) > mouse 0 (10)");
    println!("Press Escape key to exit...\n");

    let mut event_count = 0;
    loop {
        // Wait for any device to have input available
        match interception.wait(Some(Duration::from_millis(100))) {
            Ok(device) => {
                event_count += 1;

                match device {
                    Device::Keyboard(keyboard) => {
                        match keyboard.receive(10) {
                            Ok(strokes) => {
                                if !strokes.is_empty() {
                                    let precedence = keyboard.get_precedence().unwrap_or(-1);

                                    for stroke in &strokes {
                                        let key_action = if stroke.state & 0x01 != 0 {
                                            "UP"
                                        } else {
                                            "DOWN"
                                        };

                                        println!(
                                            "Event #{}: KEYBOARD (precedence: {}) - Key {}: 0x{:02X}",
                                            event_count, precedence, key_action, stroke.code
                                        );

                                        // Exit on Escape key
                                        if stroke.code == 0x01 && (stroke.state & 0x01) == 0 {
                                            println!("\nEscape key pressed. Exiting demo...");
                                            return Ok(());
                                        }
                                    }

                                    // Send the strokes back so they still work normally
                                    keyboard.send(&strokes)?;
                                }
                            }
                            Err(e) => eprintln!("Error receiving keyboard strokes: {e}"),
                        }
                    }
                    Device::Mouse(mouse) => {
                        match mouse.receive(10) {
                            Ok(strokes) => {
                                if !strokes.is_empty() {
                                    let precedence = mouse.get_precedence().unwrap_or(-1);

                                    for stroke in &strokes {
                                        println!(
                                            "Event #{}: MOUSE (precedence: {}) - pos=({}, {}), state=0x{:04X}",
                                            event_count,
                                            precedence,
                                            stroke.x,
                                            stroke.y,
                                            stroke.state
                                        );
                                    }

                                    // Send the strokes back so they still work normally
                                    mouse.send(&strokes)?;
                                }
                            }
                            Err(e) => eprintln!("Error receiving mouse strokes: {e}"),
                        }
                    }
                }
            }
            Err(interception::InterceptionError::Wait(interception::WaitError::WaitTimeout)) => {
                // Timeout - continue loop
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
