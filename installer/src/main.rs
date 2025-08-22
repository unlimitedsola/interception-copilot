use interception_installer::DriverType;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "install" => {
            for driver_type in DriverType::ALL {
                println!("Installing {driver_type:?} driver...");
                driver_type.install()?;
            }

            println!("Driver installation completed successfully.");
            println!();
            println!("IMPORTANT: You must reboot your system for the drivers to take effect.");
        }
        "uninstall" => {
            println!("Uninstalling Interception drivers...");

            for driver_type in DriverType::ALL {
                println!("Removing {driver_type:?} driver...");
                driver_type.uninstall()?;
            }

            println!("Driver uninstallation completed successfully.");
            println!();
            println!("IMPORTANT: You must reboot your system for the changes to take effect.");
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
