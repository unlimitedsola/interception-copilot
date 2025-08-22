use std::env;
use std::process;

use interception_installer::{install, uninstall};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "install" => {
            if let Err(e) = install() {
                eprintln!("Installation failed: {e:?}");
                process::exit(1);
            }
            println!("Installation completed successfully.");
        }
        "uninstall" => {
            if let Err(e) = uninstall() {
                eprintln!("Uninstallation failed: {e:?}");
                process::exit(1);
            }
            println!("Uninstallation completed successfully.");
        }
        _ => {
            print_usage();
            process::exit(1);
        }
    }
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
