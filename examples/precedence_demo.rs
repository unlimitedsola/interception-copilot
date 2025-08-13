//! Precedence demonstration example using the Interception API
//!
//! This example demonstrates how to use device precedence values to control
//! the order in which devices are processed when multiple devices have input available.
//!
//! Device precedence is an integer value that determines the priority order for processing
//! when the `wait()` function needs to select from multiple devices with pending input.
//! Higher precedence values are processed first.
//!
//! **Note**: This requires the Interception driver to be installed on Windows.

use interception::{Device, FILTER_KEY_ALL, FILTER_MOUSE_ALL, Interception, Precedence};
use std::time::Duration;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Interception Device Precedence Demonstration ===");
    println!("This example shows how to get/set precedence values and their effects.");
    println!("Press Ctrl+C to exit\n");

    // Create the high-level Interception instance which manages all devices
    let mut interception = Interception::new()?;

    println!(
        "Created Interception instance with {} devices",
        interception.devices().len()
    );

    // Step 1: Display initial precedence values for all devices
    println!("\n--- Step 1: Initial Device Precedence Values ---");
    display_device_precedence(&mut interception)?;

    // Step 2: Configure some devices with filters and set different precedence values
    println!("\n--- Step 2: Setting Up Devices with Different Precedence ---");
    configure_devices_with_precedence(&mut interception)?;

    // Step 3: Display the updated precedence values
    println!("\n--- Step 3: Updated Device Precedence Values ---");
    display_device_precedence(&mut interception)?;

    // Step 4: Demonstrate precedence in action with device waiting
    println!("\n--- Step 4: Precedence in Action ---");
    println!("Move your mouse or press keys to see which devices get processed first");
    println!("Higher precedence devices will be selected first when multiple have input");

    demonstrate_precedence_order(&mut interception)?;

    Ok(())
}

/// Display precedence values for all devices
fn display_device_precedence(
    interception: &mut Interception,
) -> Result<(), Box<dyn std::error::Error>> {
    let devices = interception.devices_mut();

    for (i, device) in devices.iter_mut().enumerate() {
        let precedence_result = device.get_precedence();
        let hardware_id_result = device.get_hardware_id();

        let device_type = if i < 10 { "keyboard" } else { "mouse" };
        let device_index = if i < 10 { i } else { i - 10 };

        match (precedence_result, hardware_id_result) {
            (Ok(precedence), Ok(hardware_id)) => {
                // Truncate hardware ID for display
                let short_id = if hardware_id.len() > 40 {
                    format!("{}...", &hardware_id[..37])
                } else {
                    hardware_id
                };
                println!(
                    "  Device {i:2} ({device_type} {device_index}): precedence={precedence:3}, hardware_id=\"{short_id}\""
                );
            }
            (Ok(precedence), Err(_)) => {
                println!(
                    "  Device {i:2} ({device_type} {device_index}): precedence={precedence:3}, hardware_id=<unavailable>"
                );
            }
            (Err(prec_err), Ok(hardware_id)) => {
                let short_id = if hardware_id.len() > 40 {
                    format!("{}...", &hardware_id[..37])
                } else {
                    hardware_id
                };
                println!(
                    "  Device {i:2} ({device_type} {device_index}): precedence=<error: {prec_err}>, hardware_id=\"{short_id}\""
                );
            }
            (Err(prec_err), Err(_)) => {
                println!(
                    "  Device {i:2} ({device_type} {device_index}): precedence=<error: {prec_err}>, hardware_id=<unavailable>"
                );
            }
        }
    }

    Ok(())
}

/// Configure devices with filters and set different precedence values
fn configure_devices_with_precedence(
    interception: &mut Interception,
) -> Result<(), Box<dyn std::error::Error>> {
    let devices = interception.devices_mut();
    let mut configured_keyboards = 0;
    let mut configured_mice = 0;

    for (i, device) in devices.iter_mut().enumerate() {
        match device {
            Device::Keyboard(keyboard) => {
                // Configure up to 3 keyboard devices with different precedence values
                if configured_keyboards < 3 {
                    match keyboard.set_filter(FILTER_KEY_ALL) {
                        Ok(_) => {
                            // Set different precedence values: 10, 20, 30
                            let precedence: Precedence = (configured_keyboards + 1) * 10;
                            match keyboard.set_precedence(precedence) {
                                Ok(_) => {
                                    println!(
                                        "  ✓ Keyboard device {i} configured with precedence {precedence}"
                                    );
                                    configured_keyboards += 1;
                                }
                                Err(e) => {
                                    eprintln!(
                                        "  ✗ Failed to set precedence for keyboard device {i}: {e}"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("  ✗ Failed to set filter for keyboard device {i}: {e}");
                        }
                    }
                }
            }
            Device::Mouse(mouse) => {
                // Configure up to 2 mouse devices with different precedence values
                if configured_mice < 2 {
                    match mouse.set_filter(FILTER_MOUSE_ALL) {
                        Ok(_) => {
                            // Set different precedence values: 5, 15 (lower than keyboards)
                            let precedence: Precedence = (configured_mice * 10) + 5;
                            match mouse.set_precedence(precedence) {
                                Ok(_) => {
                                    println!(
                                        "  ✓ Mouse device {i} configured with precedence {precedence}"
                                    );
                                    configured_mice += 1;
                                }
                                Err(e) => {
                                    eprintln!(
                                        "  ✗ Failed to set precedence for mouse device {i}: {e}"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("  ✗ Failed to set filter for mouse device {i}: {e}");
                        }
                    }
                }
            }
        }
    }

    println!(
        "  Configured {configured_keyboards} keyboard device(s) and {configured_mice} mouse device(s)"
    );

    if configured_keyboards == 0 && configured_mice == 0 {
        println!(
            "  Warning: No devices could be configured. Is the Interception driver installed?"
        );
    }

    Ok(())
}

/// Demonstrate precedence order by showing which device gets selected first
fn demonstrate_precedence_order(
    interception: &mut Interception,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_count = 0;
    let max_events = 20; // Limit to prevent infinite loop in demo

    println!("Waiting for up to {max_events} input events to demonstrate precedence order...");
    println!("(Devices with higher precedence values will be processed first)\n");

    while event_count < max_events {
        // Wait for any device to have input available (with timeout)
        match interception.wait(Some(Duration::from_millis(500))) {
            Ok(device) => {
                event_count += 1;

                // Get device information first, before getting precedence
                let precedence = device.get_precedence().unwrap_or(-1);

                match device {
                    Device::Keyboard(keyboard) => {
                        // Try to receive and forward keyboard input
                        match keyboard.receive(1) {
                            Ok(strokes) => {
                                if !strokes.is_empty() {
                                    let stroke = &strokes[0];
                                    let key_action = if stroke.state & 0x01 != 0 {
                                        "UP"
                                    } else {
                                        "DOWN"
                                    };

                                    println!(
                                        "Event #{event_count:2}: Keyboard device (precedence={precedence:2}) - Key {key_action} 0x{:02X}",
                                        stroke.code
                                    );

                                    // Forward the input so it works normally
                                    keyboard.send(&strokes)?;
                                }
                            }
                            Err(e) => {
                                println!(
                                    "Event #{event_count:2}: Keyboard device (precedence={precedence:2}) - Error: {e}"
                                );
                            }
                        }
                    }
                    Device::Mouse(mouse) => {
                        // Try to receive and forward mouse input
                        match mouse.receive(1) {
                            Ok(strokes) => {
                                if !strokes.is_empty() {
                                    let stroke = &strokes[0];
                                    println!(
                                        "Event #{event_count:2}: Mouse device (precedence={precedence:2}) - Mouse pos=({}, {}), state=0x{:04X}",
                                        stroke.x, stroke.y, stroke.state
                                    );

                                    // Forward the input so it works normally
                                    mouse.send(&strokes)?;
                                }
                            }
                            Err(e) => {
                                println!(
                                    "Event #{event_count:2}: Mouse device (precedence={precedence:2}) - Error: {e}"
                                );
                            }
                        }
                    }
                }
            }
            Err(interception::InterceptionError::Wait(interception::WaitError::WaitTimeout)) => {
                // Timeout - show a progress indicator
                print!(".");
                std::io::Write::flush(&mut std::io::stdout()).ok();
            }
            Err(e) => {
                eprintln!("\nError waiting for input: {e}");
                return Err(e.into());
            }
        }
    }

    println!("\n\nDemo complete! Observed {event_count} input events.");
    println!("Notice how devices with higher precedence values were consistently");
    println!("selected first when multiple devices had input available simultaneously.");

    Ok(())
}

/// Demonstrate setting precedence on all devices at once
fn _demonstrate_global_precedence_setting(
    interception: &mut Interception,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Bonus: Setting Global Precedence ---");
    println!("Setting precedence to 100 for all devices...");

    match interception.set_precedence(100) {
        Ok(_) => {
            println!("✓ Successfully set precedence to 100 for all devices");

            // Verify the change
            println!("\nVerifying global precedence setting:");
            display_device_precedence(interception)?;
        }
        Err(e) => {
            eprintln!("✗ Failed to set global precedence: {e}");
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn main() {
    println!("=== Interception Device Precedence Demonstration ===");
    println!("This example only works on Windows with the Interception driver installed.");
    println!("Current platform is not Windows.");
    println!("\nThe precedence functionality allows controlling the order in which devices");
    println!("are processed when multiple devices have input available simultaneously.");
    println!("Higher precedence values are processed first.");
}
