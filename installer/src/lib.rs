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

use crate::registry::Key;
use std::error::Error;
use std::fmt::Display;
use std::mem::size_of;
use std::path::Path;
use std::{fmt, fs, io, ptr};
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, FALSE};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, REG_MULTI_SZ, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows_sys::Win32::System::Services::{
    SERVICE_DEMAND_START, SERVICE_ERROR_NORMAL, SERVICE_KERNEL_DRIVER,
};
use windows_sys::Win32::System::SystemInformation::{
    GetSystemInfo, GetVersionExW, OSVERSIONINFOW, PROCESSOR_ARCHITECTURE_AMD64,
    PROCESSOR_ARCHITECTURE_IA64, PROCESSOR_ARCHITECTURE_INTEL, SYSTEM_INFO,
};
use windows_sys::core::PCWSTR;
use windows_sys::w;

mod registry;

// Embedded driver files organized by type and system parameters
macro_rules! embed_driver {
    ($name:literal) => {
        include_bytes!(concat!("../drivers/", $name, ".sys")).as_slice()
    };
}

const DRIVERS_PATH: &str = r"C:\Windows\System32\drivers";

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
    pub const fn service_name(self) -> &'static str {
        match self {
            Self::Keyboard => "keyboard",
            Self::Mouse => "mouse",
        }
    }

    /// Returns the display name shown in Windows services
    pub const fn display_name(self) -> PCWSTR {
        match self {
            Self::Keyboard => w!("Keyboard Upper Filter Driver"),
            Self::Mouse => w!("Mouse Upper Filter Driver"),
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
        self.uninstall_service()
            .map_err(InstallError::RegistryError)?;

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
            key.set_raw(w!("DisplayName"), REG_SZ, &[])?; //todo
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
        unsafe {
            let mut key: HKEY = ptr::null_mut();

            let result = RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                self.class_key(),
                0,
                KEY_ALL_ACCESS,
                &mut key,
            );

            if result != ERROR_SUCCESS {
                todo!()
            }

            // Get current UpperFilters value
            let mut filters = get_upper_filters(key)?;

            // Add our filter if not already present
            let svc_name = self.service_name();
            if !filters.iter().any(|f| f == svc_name) {
                filters.push(svc_name.to_string());
                set_upper_filters(key, &filters)?;
            }

            RegCloseKey(key);
        }

        Ok(())
    }

    fn remove_class_filter(self) -> Result<(), registry::Error> {
        unsafe {
            let mut key: HKEY = ptr::null_mut();

            let result = RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                self.class_key(),
                0,
                KEY_ALL_ACCESS,
                &mut key,
            );

            if result != ERROR_SUCCESS {
                todo!()
            }

            // Get current UpperFilters value
            let mut filters = get_upper_filters(key)?;

            // Remove our filter
            filters.retain(|f| f != self.service_name());

            if filters.is_empty() {
                // Delete the UpperFilters value if no filters remain
                RegDeleteValueW(key, w!("UpperFilters"));
            } else {
                set_upper_filters(key, &filters)?;
            }

            RegCloseKey(key);
        }

        Ok(())
    }

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
                return Err(InstallError::DriverNotFound(format!(
                    "No driver available for {self:?} on {:?} {:?}",
                    system_info.version, system_info.architecture
                )));
            }
        };
        Ok(driver_data)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WindowsNTVersion {
    pub major: u32,
    pub minor: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum Architecture {
    X86,
    AMD64,
    IA64,
}

#[derive(Debug)]
pub struct SystemInfo {
    pub version: WindowsNTVersion,
    pub architecture: Architecture,
}

impl SystemInfo {
    pub fn detect() -> Result<Self, &'static str> {
        let version = get_windows_version()?;
        let architecture = get_architecture()?;

        Ok(SystemInfo {
            version,
            architecture,
        })
    }
}

#[derive(Debug)]
pub enum InstallError {
    SystemDetectionFailed(&'static str),
    IoError(io::Error),
    RegistryError(registry::Error),
    DriverNotFound(String),
    PermissionDenied,
}

impl Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallError::SystemDetectionFailed(msg) => {
                write!(f, "System detection failed: {msg}")
            }
            InstallError::IoError(err) => write!(f, "I/O error: {err}"),
            InstallError::RegistryError(err) => write!(f, "Registry error: {err}"),
            InstallError::DriverNotFound(msg) => write!(f, "Driver file not found: {msg}"),
            InstallError::PermissionDenied => {
                write!(f, "Permission denied - administrator privileges required")
            }
        }
    }
}

impl Error for InstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            InstallError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for InstallError {
    fn from(err: io::Error) -> Self {
        InstallError::IoError(err)
    }
}

impl From<registry::Error> for InstallError {
    fn from(err: registry::Error) -> Self {
        InstallError::RegistryError(err)
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
    let system_info = SystemInfo::detect().map_err(InstallError::SystemDetectionFailed)?;

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

fn get_windows_version() -> Result<WindowsNTVersion, &'static str> {
    unsafe {
        let mut version_info = OSVERSIONINFOW {
            dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };

        if GetVersionExW(&mut version_info) == FALSE {
            return Err("Failed to get Windows version");
        }

        Ok(WindowsNTVersion {
            major: version_info.dwMajorVersion,
            minor: version_info.dwMinorVersion,
        })
    }
}

fn get_architecture() -> Result<Architecture, &'static str> {
    unsafe {
        let mut system_info = SYSTEM_INFO::default();
        GetSystemInfo(&mut system_info);

        let architecture = match system_info.Anonymous.Anonymous.wProcessorArchitecture {
            PROCESSOR_ARCHITECTURE_INTEL => Architecture::X86,
            PROCESSOR_ARCHITECTURE_AMD64 => Architecture::AMD64,
            PROCESSOR_ARCHITECTURE_IA64 => Architecture::IA64,
            _ => return Err("Unsupported processor architecture"),
        };

        Ok(architecture)
    }
}

fn get_upper_filters(key: HKEY) -> Result<Vec<String>, registry::Error> {
    unsafe {
        let mut buffer_size = 0u32;
        let mut data_type = 0u32;

        // Get the size of the data
        let result = RegQueryValueExW(
            key,
            w!("UpperFilters"),
            ptr::null(), // lpReserved - must be null (input parameter)
            &mut data_type,
            ptr::null_mut(), // lpData - can be null when querying size (output parameter)
            &mut buffer_size,
        );

        if result != ERROR_SUCCESS || data_type != REG_MULTI_SZ {
            // No existing UpperFilters or wrong type, return empty vector
            return Ok(Vec::new());
        }

        let mut buffer = vec![0u8; buffer_size as usize];
        let result = RegQueryValueExW(
            key,
            w!("UpperFilters"),
            ptr::null(), // lpReserved - must be null (input parameter)
            &mut data_type,
            buffer.as_mut_ptr(),
            &mut buffer_size,
        );

        if result != ERROR_SUCCESS {
            todo!()
        }

        // Convert buffer to Vec<String>
        let wide_chars = buffer.len() / 2;
        let wide_slice = std::slice::from_raw_parts(buffer.as_ptr() as *const u16, wide_chars);

        let mut filters = Vec::new();
        let mut start = 0;

        for (i, &ch) in wide_slice.iter().enumerate() {
            if ch == 0 {
                if i > start {
                    let filter_slice = &wide_slice[start..i];
                    if let Ok(filter) = String::from_utf16(filter_slice)
                        && !filter.is_empty()
                    {
                        filters.push(filter);
                    }
                }
                start = i + 1;
                if start >= wide_slice.len() || wide_slice[start] == 0 {
                    break;
                }
            }
        }

        Ok(filters)
    }
}

fn set_upper_filters(key: HKEY, filters: &[String]) -> Result<(), registry::Error> {
    // Convert to wide multi-string format
    let mut wide_data = Vec::new();

    for filter in filters {
        let wide_filter = to_wide_string(filter);
        wide_data.extend_from_slice(&wide_filter[..wide_filter.len() - 1]); // exclude null terminator
        wide_data.push(0); // add separator
    }
    wide_data.push(0); // add final null terminator

    unsafe {
        let result = RegSetValueExW(
            key,
            w!("UpperFilters"),
            0,
            REG_MULTI_SZ,
            wide_data.as_ptr() as *const u8,
            (wide_data.len() * 2) as u32,
        );

        if result != ERROR_SUCCESS {
            todo!();
        }
    }

    Ok(())
}

fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
