//! Typed mouse event capture using the new idiomatic API
//!
//! This example demonstrates how to use the new MouseDevice type
//! to capture and send mouse events in a type-safe manner.

use interception_copilot::{
    FILTER_MOUSE_ALL, MOUSE_LEFT_BUTTON_DOWN, MOUSE_LEFT_BUTTON_UP, MouseDevice, MouseStroke,
};

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Typed mouse event capture example");
    println!("Move your mouse and click buttons to see events");
    println!("Press Ctrl+C to exit");

    // Create mouse devices for first 3 mice
    let mut mice: Vec<MouseDevice> = Vec::new();
    for i in 0..3 {
        if let Ok(mouse) = MouseDevice::new(i) {
            mice.push(mouse);
        }
    }

    if mice.is_empty() {
        println!("No mouse devices could be created. Is the Interception driver installed?");
        return Ok(());
    }

    println!("Created {} mouse device(s)", mice.len());

    // Set filter to capture all mouse events on each device
    for mouse in &mice {
        mouse.set_filter(FILTER_MOUSE_ALL)?;
    }

    let mut event_count = 0;
    let mut last_synthetic = std::time::Instant::now();

    // Main event loop
    loop {
        // Check each mouse device for input
        for (device_index, mouse) in mice.iter().enumerate() {
            // Try to receive mouse strokes from the device
            match mouse.receive(10) {
                Ok(strokes) => {
                    if !strokes.is_empty() {
                        for stroke in &strokes {
                            println!(
                                "Mouse {} event #{}: pos=({}, {}), state=0x{:04X}, flags=0x{:04X}, rolling={}",
                                device_index,
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

        // Send synthetic mouse click every 10 seconds as a demo
        if last_synthetic.elapsed() > std::time::Duration::from_secs(10) && !mice.is_empty() {
            println!("Sending synthetic mouse click...");

            let click_down = MouseStroke::button_down(MOUSE_LEFT_BUTTON_DOWN);
            let click_up = MouseStroke::button_up(MOUSE_LEFT_BUTTON_UP);

            // Send to first mouse device
            let first_mouse = &mice[0];
            if let Err(e) = first_mouse.send(&[click_down]) {
                eprintln!("Error sending click down: {e}");
            } else {
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Err(e) = first_mouse.send(&[click_up]) {
                    eprintln!("Error sending click up: {e}");
                } else {
                    println!("Synthetic click sent to mouse 0!");
                }
            }

            last_synthetic = std::time::Instant::now();
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
