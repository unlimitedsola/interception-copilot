//! Mouse event capture and replay example
//! 
//! This example shows how to capture mouse movements and button clicks,
//! and demonstrates how to send synthetic mouse events.

use interception_copilot::{
    Context, InterceptionFilter, MouseStroke, Stroke, MouseState,
    is_mouse_device, mouse,
};

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Mouse event capture example");
    println!("Move your mouse and click buttons to see events");
    println!("Press Ctrl+C to exit");
    
    // Create an interception context
    let context = Context::new()?;
    
    // Set filter to capture all mouse events
    context.set_filter(is_mouse_device, InterceptionFilter::MOUSE_ALL)?;
    
    let mut event_count = 0;
    
    // Main event loop
    loop {
        // Wait for any device to have input available
        if let Some(device) = context.wait_with_timeout(1000) {
            if is_mouse_device(device) {
                // Receive mouse strokes from the device
                match context.receive(device, 10) {
                    Ok(strokes) => {
                        for stroke in &strokes {
                            unsafe {
                                let mouse_stroke = stroke.mouse;
                                println!("Mouse event #{}: pos=({}, {}), state=0x{:04X}, flags=0x{:04X}, rolling={}", 
                                    event_count, 
                                    mouse_stroke.x, mouse_stroke.y,
                                    mouse_stroke.state, mouse_stroke.flags, 
                                    mouse_stroke.rolling);
                            }
                            event_count += 1;
                        }
                        
                        // Send the strokes back so they still work normally
                        context.send(device, &strokes)?;
                    },
                    Err(e) => eprintln!("Error receiving strokes: {}", e),
                }
            }
        } else {
            // Timeout occurred - maybe send a synthetic mouse event as demo
            if event_count > 0 && event_count % 10 == 0 {
                println!("Sending synthetic mouse click...");
                
                let click_down = Stroke::from(MouseStroke::button_down(MouseState::LeftButtonDown));
                let click_up = Stroke::from(MouseStroke::button_up(MouseState::LeftButtonUp));
                
                // Send to first mouse device
                if let Some(first_mouse) = (0..10).map(mouse).find(|&d| is_mouse_device(d)) {
                    let _ = context.send(first_mouse, &[click_down]);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    let _ = context.send(first_mouse, &[click_up]);
                    println!("Synthetic click sent!");
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