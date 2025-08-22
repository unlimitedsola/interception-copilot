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

use crate::registry::{Key, Value};
use crate::sysinfo::{Architecture, SystemInfo};
use crate::wcstr::WCStr;
use std::error::Error;
use std::ffi::OsString;
use std::fmt::Display;
use std::os::windows::ffi::OsStringExt;
use std::{fmt, fs, io, ptr};
use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_DELAY_UNTIL_REBOOT, MoveFileExW};
use windows_sys::Win32::System::Registry::{KEY_ALL_ACCESS, REG_OPTION_NON_VOLATILE};
use windows_sys::Win32::System::Services::{
    SERVICE_DEMAND_START, SERVICE_ERROR_NORMAL, SERVICE_KERNEL_DRIVER,
};
use windows_sys::core::PCWSTR;
use windows_sys::w;

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

    pub const fn driver_path(self) -> &'static WCStr {
        match self {
            Self::Keyboard => wcstr!(r"C:\Windows\System32\drivers\keyboard.sys"),
            Self::Mouse => wcstr!(r"C:\Windows\System32\drivers\mouse.sys"),
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

impl DriverType {
    fn install(self, system_info: &SystemInfo) -> Result<(), InstallError> {
        let driver_data = self.get_driver_binary(system_info)?;
        let path = OsString::from_wide(self.driver_path().as_wide());
        fs::write(path, driver_data)?;

        self.install_service()?;
        Ok(())
    }

    fn uninstall(self) -> Result<(), InstallError> {
        self.uninstall_service()?;

        // Remove the driver file on next reboot
        // Ignore errors, as the file may not exist
        let _ = unsafe {
            MoveFileExW(
                self.driver_path().as_ptr(),
                ptr::null(),
                MOVEFILE_DELAY_UNTIL_REBOOT,
            )
        };
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
        let res = unsafe { Key::LOCAL_MACHINE.delete_key(self.service_key()) };
        if let Err(err) = res
            && err == registry::Error::FILE_NOT_FOUND
        {
            // ignore if the key does not exist
            Ok(())
        } else {
            res
        }
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
        unsafe { key.set(w!("UpperFilters"), filters.as_slice())? };
        Ok(())
    }

    fn open_class_key(self) -> Result<Key, registry::Error> {
        unsafe {
            Key::LOCAL_MACHINE.open(self.class_key(), REG_OPTION_NON_VOLATILE, KEY_ALL_ACCESS)
        }
    }
}

impl DriverType {
    fn get_driver_binary(self, sys: &SystemInfo) -> Result<&'static [u8], InstallError> {
        struct DriverSet {
            keyboard: &'static [u8],
            mouse: &'static [u8],
        }

        macro_rules! embed_driver {
            ($name:expr) => {
                include_bytes!(concat!("drivers/", $name, ".sys")).as_slice()
            };
        }

        macro_rules! drivers {
            ($ver_arch:literal) => {
                &DriverSet {
                    keyboard: embed_driver!(concat!("KBD", $ver_arch)),
                    mouse: embed_driver!(concat!("MOU", $ver_arch)),
                }
            };
        }

        let driver_set = match (sys.version.major, sys.version.minor, sys.architecture) {
            // Windows 5.1 (XP)
            #[cfg(feature = "unsupported-platforms")]
            (5, 1, Architecture::X86) => drivers!("NT51X86"),
            // Windows 5.2 (2003)
            #[cfg(feature = "unsupported-platforms")]
            (5, 2, Architecture::AMD64) => drivers!("NT52A64"),
            #[cfg(feature = "unsupported-platforms")]
            (5, 2, Architecture::IA64) => drivers!("NT52I64"),
            #[cfg(feature = "unsupported-platforms")]
            (5, 2, Architecture::X86) => drivers!("NT52X86"),
            // Windows 6.0 (Vista)
            #[cfg(feature = "unsupported-platforms")]
            (6, 0, Architecture::AMD64) => drivers!("NT60A64"),
            #[cfg(feature = "unsupported-platforms")]
            (6, 0, Architecture::IA64) => drivers!("NT60I64"),
            #[cfg(feature = "unsupported-platforms")]
            (6, 0, Architecture::X86) => drivers!("NT60X86"),
            // Windows 6.1+ (7+)
            (6, 1.., Architecture::AMD64) | (10.., _, Architecture::AMD64) => drivers!("NT61A64"),
            #[cfg(feature = "unsupported-platforms")]
            (6, 1.., Architecture::IA64) | (10.., _, Architecture::IA64) => drivers!("NT61I64"),
            (6, 1.., Architecture::X86) | (10.., _, Architecture::X86) => drivers!("NT61X86"),
            _ => {
                return Err(InstallError::UnsupportedSystem(*sys));
            }
        };

        let driver_data = match self {
            DriverType::Keyboard => driver_set.keyboard,
            DriverType::Mouse => driver_set.mouse,
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
        driver_type.install(&system_info)?;
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
        driver_type.uninstall()?;
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
    UnsupportedSystem(SystemInfo),
}

impl Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SystemDetection(err) => {
                write!(f, "System detection failed: {err}")
            }
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Registry(err) => write!(f, "Registry error: {err}"),
            Self::UnsupportedSystem(sys) => {
                write!(f, "Unsupported system configuration: {sys:?}")
            }
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
