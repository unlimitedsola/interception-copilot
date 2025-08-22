use std::env;

use interception_installer::{install, uninstall};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "install" => {
            install()?;
            println!("Installation completed successfully.");
        }
        "uninstall" => {
            uninstall()?;
            println!("Uninstallation completed successfully.");
        }
        _ => {
            print_usage();
        }
    }
    Ok(())
}

fn print_usage() {
    println!("Interception Driver Installer");
    println!("Usage: interception-installer <command>");
    println!();
    println!("Commands:");
    println!("  install     Install the Interception drivers");
    println!("  uninstall   Uninstall the Interception drivers");
    println!();
    println!("Note: This tool requires administrator privileges.");
}
