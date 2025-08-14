/// Example of using the installer programmatically
///
/// This example shows how to use the InterceptionInstaller directly
/// without the CLI interface, useful for embedding in other applications.
use interception_installer::InterceptionInstaller;

fn main() {
    println!("Interception Driver Installer - Programmatic Example");
    println!("===================================================");

    // Create installer instance
    let installer = InterceptionInstaller::new();

    // Example: Install drivers
    match installer.install() {
        Ok(()) => {
            println!("✓ Installation completed successfully");
            println!("ℹ Please reboot your system for drivers to take effect");
        }
        Err(e) => {
            eprintln!("✗ Installation failed: {e}");
            std::process::exit(1);
        }
    }

    // Example usage for uninstall would be:
    // match installer.uninstall() {
    //     Ok(()) => println!("✓ Uninstallation completed successfully"),
    //     Err(e) => eprintln!("✗ Uninstallation failed: {e}"),
    // }
}
