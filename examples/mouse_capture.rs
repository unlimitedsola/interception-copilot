//! Mouse event capture using the high-level Interception API
//!
//! This example demonstrates how to use the high-level Interception API
//! to capture and send mouse events efficiently using event-driven waiting.

use interception::{
    Device, FILTER_MOUSE_ALL, Interception, MOUSE_LEFT_BUTTON_DOWN, MOUSE_LEFT_BUTTON_UP,
    MOUSE_MOVE_RELATIVE, MouseStroke,
};
use std::time::{Duration, Instant};

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Mouse event capture using high-level API");
    println!("Move your mouse and click buttons to see events");
    println!("Press Ctrl+C to exit");

    // Create the high-level Interception instance which manages all devices
    let mut interception = Interception::new()?;

    println!(
        "Created Interception instance with {} devices",
        interception.devices().len()
    );

    // Set filter to capture all mouse events on mouse devices
    let devices = interception.devices_mut();
    let mut active_mice = 0;
    for (i, device) in devices.iter_mut().enumerate() {
        if let Device::Mouse(mouse) = device {
            match mouse.set_filter(FILTER_MOUSE_ALL) {
                Ok(_) => {
                    active_mice += 1;
                    println!("Set filter for mouse device {i}");
                }
                Err(e) => eprintln!("Failed to set filter for mouse device {i}: {e}"),
            }
        }
    }

    if active_mice == 0 {
        println!("No mouse devices could be configured. Is the Interception driver installed?");
        return Ok(());
    }

    println!("Configured {active_mice} mouse device(s)");

    // Print hardware IDs for all devices on startup
    println!("Device hardware IDs:");
    let devices = interception.devices_mut();
    for (i, device) in devices.iter_mut().enumerate() {
        match device.get_hardware_id() {
            Ok(hardware_id) => {
                let device_type = if i < 10 { "keyboard" } else { "mouse" };
                let device_index = if i < 10 { i } else { i - 10 };
                println!("  Device {i} ({device_type} {device_index}): {hardware_id}");
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

    let mut event_count = 0;
    let mut last_synthetic = Instant::now();

    // Main event loop using efficient waiting
    loop {
        // Wait for any device to have input available (with 100ms timeout)
        match interception.wait(Some(Duration::from_millis(100))) {
            Ok(device) => {
                // Check if this is a mouse device
                if let Device::Mouse(mouse) = device {
                    // Try to receive mouse strokes from the device
                    match mouse.receive(10) {
                        Ok(strokes) => {
                            if !strokes.is_empty() {
                                for stroke in &strokes {
                                    println!(
                                        "Mouse event #{}: pos=({}, {}), state=0x{:04X}, flags=0x{:04X}, rolling={}",
                                        event_count,
                                        stroke.x,
                                        stroke.y,
                                        stroke.state,
                                        stroke.flags,
                                        stroke.rolling
                                    );
                                    event_count += 1;
                                }

                                // Send the strokes back so they still work normally
                                mouse.send(&strokes)?;
                            }
                        }
                        Err(e) => eprintln!("Error receiving strokes: {e}"),
                    }
                }
                // If it's a keyboard device, we ignore it in this mouse-only example
            }
            Err(interception::InterceptionError::Wait(interception::WaitError::WaitTimeout)) => {
                // Timeout - check if we should send synthetic events
            }
            Err(e) => {
                eprintln!("Error waiting for input: {e}");
                return Err(e.into());
            }
        }

        // Send synthetic mouse click every 10 seconds as a demo
        if last_synthetic.elapsed() > Duration::from_secs(10) {
            println!("Sending synthetic mouse click...");

            // Send to first mouse device we can find
            let devices = interception.devices_mut();
            let mut sent = false;
            for device in devices.iter_mut() {
                if let Device::Mouse(mouse) = device {
                    let click_down =
                        MouseStroke::new(MOUSE_MOVE_RELATIVE, MOUSE_LEFT_BUTTON_DOWN, 0, 0, 0, 0);
                    let click_up =
                        MouseStroke::new(MOUSE_MOVE_RELATIVE, MOUSE_LEFT_BUTTON_UP, 0, 0, 0, 0);

                    if let Err(e) = mouse.send(&[click_down]) {
                        eprintln!("Error sending click down: {e}");
                    } else {
                        std::thread::sleep(Duration::from_millis(50));
                        if let Err(e) = mouse.send(&[click_up]) {
                            eprintln!("Error sending click up: {e}");
                        } else {
                            println!("Synthetic click sent to mouse device!");
                            sent = true;
                            break;
                        }
                    }
                }
            }

            if !sent {
                eprintln!("No active mouse devices found for synthetic click");
            }

            last_synthetic = Instant::now();
        }
    }
}

#[cfg(not(windows))]
fn main() {
    println!("This example only works on Windows with the Interception driver installed.");
    println!("Current platform is not Windows.");
}
