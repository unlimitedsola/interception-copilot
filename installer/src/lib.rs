//! Interception Driver Installer Library
//!
//! This library provides functionality to install and uninstall Interception drivers
//! on Windows systems. It handles system detection, driver file management, registry
//! configuration, and device class filter setup.
//!
//! # Features
//!
//! - Automatic Windows version and architecture detection
//! - Driver file extraction and installation to system directory  
//! - Registry service configuration for keyboard and mouse drivers
//! - Device class filter setup (UpperFilters registry entries)
//! - Complete uninstallation support
//!
//! # Usage
//!
//! ```no_run
//! use interception_installer::InterceptionInstaller;
//!
//! let installer = InterceptionInstaller::new();
//!
//! // Install drivers
//! match installer.install() {
//!     Ok(()) => println!("Installation completed successfully"),
//!     Err(e) => eprintln!("Installation failed: {}", e),
//! }
//!
//! // Uninstall drivers  
//! match installer.uninstall() {
//!     Ok(()) => println!("Uninstallation completed successfully"),
//!     Err(e) => eprintln!("Uninstallation failed: {}", e),
//! }
//! ```
//!
//! # Important Notes
//!
//! - This library requires administrator privileges on Windows
//! - A system reboot is required after installation or uninstallation
//! - Only works on Windows systems with appropriate driver files available

pub mod installer;
pub mod registry;
pub mod system;

pub use installer::{InstallError, InterceptionInstaller};
pub use system::{ProcessorArchitecture, SystemInfo, WindowsNTVersion};
