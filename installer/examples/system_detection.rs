/// Test driver file detection and system info
///
/// This example demonstrates the system detection capabilities
/// and shows what drivers would be selected for the current system.
use interception_installer::system::SystemInfo;

fn main() {
    println!("Interception Driver Installer - System Detection Test");
    println!("====================================================");

    // Test system detection
    match SystemInfo::detect() {
        Ok(info) => {
            println!("✓ System detection successful:");
            println!("  Version: {:?}", info.version);
            println!("  Architecture: {:?}", info.architecture);
            println!("  Driver prefix: {}", info.get_driver_prefix());
            println!("  Architecture suffix: {}", info.get_architecture_suffix());

            // Show what driver files would be needed
            let keyboard_driver = format!(
                "KBDNT{}{}.sys",
                info.get_driver_prefix(),
                info.get_architecture_suffix()
            );
            let mouse_driver = format!(
                "MOUNT{}{}.sys",
                info.get_driver_prefix(),
                info.get_architecture_suffix()
            );

            println!("\nRequired driver files:");
            println!("  Keyboard: {keyboard_driver}");
            println!("  Mouse: {mouse_driver}");
        }
        Err(e) => {
            eprintln!("✗ System detection failed: {e}");
            std::process::exit(1);
        }
    }
}
