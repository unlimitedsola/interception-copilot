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
//! use interception_installer::{install, uninstall};
//!
//! // Install drivers
//! match install() {
//!     Ok(()) => println!("Installation completed successfully"),
//!     Err(e) => eprintln!("Installation failed: {}", e),
//! }
//!
//! // Uninstall drivers
//! match uninstall() {
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

use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::{fmt, fs, io};
use windows_sys::Win32::System::Registry::{KEY_ALL_ACCESS, REG_OPTION_NON_VOLATILE};
use windows_sys::Win32::System::Services::{
    SERVICE_DEMAND_START, SERVICE_ERROR_NORMAL, SERVICE_KERNEL_DRIVER,
};
use windows_sys::core::PCWSTR;
use windows_sys::w;

use crate::registry::{Key, Value};
use crate::sysinfo::{Architecture, SystemInfo};
use crate::wcstr::WCStr;

mod registry;
mod sysinfo;
mod wcstr;

/// Represents the type of input device driver
#[derive(Debug, Clone, Copy)]
pub enum DriverType {
    /// Keyboard input driver
    Keyboard,
    /// Mouse input driver
    Mouse,
}

/// All available driver types for iteration
pub const ALL_DRIVER_TYPES: &[DriverType] = &[DriverType::Keyboard, DriverType::Mouse];

impl DriverType {
    /// Returns the service name used in the Windows registry
    pub const fn service_name(self) -> &'static WCStr {
        match self {
            Self::Keyboard => wcstr!("keyboard"),
            Self::Mouse => wcstr!("mouse"),
        }
    }

    /// Returns the display name shown in Windows services
    pub const fn display_name(self) -> &'static WCStr {
        match self {
            Self::Keyboard => wcstr!("Keyboard Upper Filter Driver"),
            Self::Mouse => wcstr!("Mouse Upper Filter Driver"),
        }
    }

    /// Returns the Windows registry filter driver class key for this driver type
    pub const fn class_key(self) -> PCWSTR {
        match self {
            Self::Keyboard => {
                w!(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e96b-e325-11ce-bfc1-08002be10318}")
            }
            Self::Mouse => {
                w!(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e96f-e325-11ce-bfc1-08002be10318}")
            }
        }
    }

    /// Returns the Windows registry service key for this driver type
    pub const fn service_key(self) -> PCWSTR {
        match self {
            Self::Keyboard => w!(r"SYSTEM\CurrentControlSet\Services\keyboard"),
            Self::Mouse => w!(r"SYSTEM\CurrentControlSet\Services\mouse"),
        }
    }
}

const DRIVERS_PATH: &str = r"C:\Windows\System32\drivers";

impl DriverType {
    fn install_driver(self, system_info: &SystemInfo) -> Result<(), InstallError> {
        // Get embedded driver data directly
        let driver_data = self.get_driver_binary(system_info)?;

        // Target filename and path
        let target_filename = format!("{}.sys", self.service_name());
        let target_path = Path::new(DRIVERS_PATH).join(&target_filename);

        // Write driver file to system directory
        fs::write(&target_path, driver_data)?;

        // Install registry service
        self.install_service()?;

        Ok(())
    }

    fn uninstall_driver(self) -> Result<(), InstallError> {
        // Remove registry entries
        self.uninstall_service().map_err(InstallError::Registry)?;

        // Remove driver file from system directory
        let target_filename = format!("{}.sys", self.service_name());
        let target_path = Path::new(DRIVERS_PATH).join(&target_filename);

        if target_path.exists() {
            fs::remove_file(&target_path)?;
        }

        Ok(())
    }

    fn install_service(self) -> Result<(), registry::Error> {
        self.create_service()?;
        self.add_class_filter()?;
        Ok(())
    }

    fn uninstall_service(self) -> Result<(), registry::Error> {
        self.remove_class_filter()?;
        self.delete_service()?;
        Ok(())
    }

    fn create_service(self) -> Result<(), registry::Error> {
        unsafe {
            let key = Key::LOCAL_MACHINE.create(
                self.service_key(),
                REG_OPTION_NON_VOLATILE,
                KEY_ALL_ACCESS,
            )?;
            key.set(w!("DisplayName"), self.display_name())?;
            key.set(w!("Type"), SERVICE_KERNEL_DRIVER)?;
            key.set(w!("ErrorControl"), SERVICE_ERROR_NORMAL)?;
            key.set(w!("Start"), SERVICE_DEMAND_START)?;
        }
        Ok(())
    }

    fn delete_service(self) -> Result<(), registry::Error> {
        unsafe { Key::LOCAL_MACHINE.delete_key(self.service_key()) }
    }

    fn add_class_filter(self) -> Result<(), registry::Error> {
        let key = self.open_class_key()?;

        let filters = unsafe { key.get(w!("UpperFilters"))? };
        let Value::MultiString(mut filters) = filters else {
            return Err(registry::Error::UNSUPPORTED_TYPE);
        };

        // Add our filter if not already present
        let svc_name = self.service_name();
        if !filters.iter().any(|f| f.as_ref() == svc_name) {
            filters.push(svc_name.into());
            unsafe { key.set(w!("UpperFilters"), filters.as_slice())? };
        }
        Ok(())
    }

    fn remove_class_filter(self) -> Result<(), registry::Error> {
        let key = self.open_class_key()?;

        let filters = unsafe { key.get(w!("UpperFilters"))? };
        let Value::MultiString(mut filters) = filters else {
            return Err(registry::Error::UNSUPPORTED_TYPE);
        };

        // Add our filter if not already present
        let svc_name = self.service_name();
        filters.retain(|f| f.as_ref() != svc_name);
        if filters.is_empty() {
            // Delete the UpperFilters value if no filters remain
            unsafe { key.delete_value(w!("UpperFilters"))? };
        } else {
            unsafe { key.set(w!("UpperFilters"), filters.as_slice())? };
        }
        Ok(())
    }

    fn open_class_key(self) -> Result<Key, registry::Error> {
        unsafe {
            Key::LOCAL_MACHINE.open(self.class_key(), REG_OPTION_NON_VOLATILE, KEY_ALL_ACCESS)
        }
    }
}

// Embedded driver files organized by type and system parameters
macro_rules! embed_driver {
    ($name:literal) => {
        include_bytes!(concat!("../drivers/", $name, ".sys")).as_slice()
    };
}

impl DriverType {
    fn get_driver_binary(self, system_info: &SystemInfo) -> Result<&'static [u8], InstallError> {
        let driver_data = match (
            self,
            (system_info.version.major, system_info.version.minor),
            system_info.architecture,
        ) {
            // Keyboard drivers
            (DriverType::Keyboard, (5, 1), Architecture::X86) => embed_driver!("KBDNT51X86"),
            (DriverType::Keyboard, (5, 2), Architecture::AMD64) => embed_driver!("KBDNT52A64"),
            (DriverType::Keyboard, (5, 2), Architecture::IA64) => embed_driver!("KBDNT52I64"),
            (DriverType::Keyboard, (5, 2), Architecture::X86) => embed_driver!("KBDNT52X86"),
            (DriverType::Keyboard, (6, 0), Architecture::AMD64) => embed_driver!("KBDNT60A64"),
            (DriverType::Keyboard, (6, 0), Architecture::IA64) => embed_driver!("KBDNT60I64"),
            (DriverType::Keyboard, (6, 0), Architecture::X86) => embed_driver!("KBDNT60X86"),
            (DriverType::Keyboard, (6, 1), Architecture::AMD64) => embed_driver!("KBDNT61A64"),
            (DriverType::Keyboard, (6, 1), Architecture::IA64) => embed_driver!("KBDNT61I64"),
            (DriverType::Keyboard, (6, 1), Architecture::X86) => embed_driver!("KBDNT61X86"),
            // Mouse drivers
            (DriverType::Mouse, (5, 1), Architecture::X86) => embed_driver!("MOUNT51X86"),
            (DriverType::Mouse, (5, 2), Architecture::AMD64) => embed_driver!("MOUNT52A64"),
            (DriverType::Mouse, (5, 2), Architecture::IA64) => embed_driver!("MOUNT52I64"),
            (DriverType::Mouse, (5, 2), Architecture::X86) => embed_driver!("MOUNT52X86"),
            (DriverType::Mouse, (6, 0), Architecture::AMD64) => embed_driver!("MOUNT60A64"),
            (DriverType::Mouse, (6, 0), Architecture::IA64) => embed_driver!("MOUNT60I64"),
            (DriverType::Mouse, (6, 0), Architecture::X86) => embed_driver!("MOUNT60X86"),
            (DriverType::Mouse, (6, 1), Architecture::AMD64) => embed_driver!("MOUNT61A64"),
            (DriverType::Mouse, (6, 1), Architecture::IA64) => embed_driver!("MOUNT61I64"),
            (DriverType::Mouse, (6, 1), Architecture::X86) => embed_driver!("MOUNT61X86"),
            _ => {
                return Err(InstallError::Driver(format!(
                    "No driver available for {self:?} on {:?} {:?}",
                    system_info.version, system_info.architecture
                )));
            }
        };
        Ok(driver_data)
    }
}

/// Install all Interception drivers
///
/// This will:
/// 1. Detect the system configuration (Windows version and architecture)
/// 2. Install both keyboard and mouse drivers
/// 3. Configure registry entries and class filters
///
/// Returns an error if any step fails. A system reboot is required after successful installation.
pub fn install() -> Result<(), InstallError> {
    println!("Detecting system configuration...");
    let system_info = SystemInfo::detect()?;

    // Install all drivers
    for &driver_type in ALL_DRIVER_TYPES {
        println!("Installing {} driver...", driver_type.service_name());
        driver_type.install_driver(&system_info)?;
    }

    println!("Driver installation completed successfully.");
    println!();
    println!("IMPORTANT: You must reboot your system for the drivers to take effect.");

    Ok(())
}

/// Uninstall all Interception drivers
///
/// This will:
/// 1. Remove registry entries and class filters for both keyboard and mouse
/// 2. Delete driver files from the system directory
///
/// Returns an error if any step fails. A system reboot is required after successful uninstallation.
pub fn uninstall() -> Result<(), InstallError> {
    println!("Uninstalling Interception drivers...");

    // Uninstall all drivers
    for &driver_type in ALL_DRIVER_TYPES {
        println!("Removing {} driver...", driver_type.service_name());
        driver_type.uninstall_driver()?;
    }

    println!("Driver uninstallation completed successfully.");
    println!();
    println!("IMPORTANT: You must reboot your system for the changes to take effect.");

    Ok(())
}

#[derive(Debug)]
pub enum InstallError {
    SystemDetection(sysinfo::Error),
    Io(io::Error),
    Registry(registry::Error),
    Driver(String),
}

impl Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SystemDetection(err) => {
                write!(f, "System detection failed: {err}")
            }
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Registry(err) => write!(f, "Registry error: {err}"),
            Self::Driver(msg) => write!(f, "Driver file not found: {msg}"),
        }
    }
}

impl Error for InstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SystemDetection(err) => Some(err),
            Self::Io(err) => Some(err),
            Self::Registry(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for InstallError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<registry::Error> for InstallError {
    fn from(err: registry::Error) -> Self {
        Self::Registry(err)
    }
}

impl From<sysinfo::Error> for InstallError {
    fn from(err: sysinfo::Error) -> Self {
        Self::SystemDetection(err)
    }
}
