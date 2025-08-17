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

use std::fs;
use std::io;
use std::mem::size_of;
use std::path::Path;
use std::ptr;
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, FALSE};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, REG_DWORD, REG_MULTI_SZ, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteKeyW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW,
};
use windows_sys::Win32::System::SystemInformation::{
    GetSystemInfo, GetVersionExW, OSVERSIONINFOW, PROCESSOR_ARCHITECTURE_AMD64,
    PROCESSOR_ARCHITECTURE_IA64, PROCESSOR_ARCHITECTURE_INTEL, SYSTEM_INFO,
};
use windows_sys::core::PCWSTR;
use windows_sys::w;

mod registry;

// Constants
const DRIVERS_PATH: &str = r"C:\Windows\System32\drivers";
const SERVICES_KEY: &str = r"SYSTEM\CurrentControlSet\Services";
const KEYBOARD_CLASS_KEY: PCWSTR =
    w!(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e96b-e325-11ce-bfc1-08002be10318}");
const MOUSE_CLASS_KEY: PCWSTR =
    w!(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e96f-e325-11ce-bfc1-08002be10318}");

// Public Types

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
    pub fn service_name(&self) -> &'static str {
        match self {
            Self::Keyboard => "keyboard",
            Self::Mouse => "mouse",
        }
    }

    /// Returns the display name shown in Windows services
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Keyboard => "Keyboard Upper Filter Driver",
            Self::Mouse => "Mouse Upper Filter Driver",
        }
    }

    /// Returns the Windows registry class key for this driver type
    pub fn class_key(&self) -> &'static PCWSTR {
        match self {
            Self::Keyboard => &KEYBOARD_CLASS_KEY,
            Self::Mouse => &MOUSE_CLASS_KEY,
        }
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
    pub fn detect() -> Result<Self, String> {
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
    SystemDetectionFailed(String),
    IoError(io::Error),
    RegistryError(String),
    DriverNotFound(String),
    PermissionDenied,
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::SystemDetectionFailed(msg) => {
                write!(f, "System detection failed: {msg}")
            }
            InstallError::IoError(err) => write!(f, "I/O error: {err}"),
            InstallError::RegistryError(msg) => write!(f, "Registry error: {msg}"),
            InstallError::DriverNotFound(msg) => write!(f, "Driver file not found: {msg}"),
            InstallError::PermissionDenied => {
                write!(f, "Permission denied - administrator privileges required")
            }
        }
    }
}

impl std::error::Error for InstallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
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

// Public API Functions

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
        install_driver(&system_info, driver_type)?;
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
        uninstall_driver(driver_type)?;
    }

    println!("Driver uninstallation completed successfully.");
    println!();
    println!("IMPORTANT: You must reboot your system for the changes to take effect.");

    Ok(())
}

// Private Implementation Functions

// Embedded driver files organized by type and system parameters
macro_rules! embed_driver {
    ($name:literal) => {
        include_bytes!(concat!("../drivers/", $name, ".sys")).as_slice()
    };
}

fn get_embedded_driver_data(
    driver_type: DriverType,
    system_info: &SystemInfo,
) -> Result<&'static [u8], InstallError> {
    let driver_data = match (
        driver_type,
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
                "No driver available for {driver_type:?} on {:?} {:?}",
                system_info.version, system_info.architecture
            )));
        }
    };

    Ok(driver_data)
}

fn install_driver(system_info: &SystemInfo, driver_type: DriverType) -> Result<(), InstallError> {
    // Get embedded driver data directly
    let driver_data = get_embedded_driver_data(driver_type, system_info)?;

    // Target filename and path
    let target_filename = format!("{}.sys", driver_type.service_name());
    let target_path = Path::new(DRIVERS_PATH).join(&target_filename);

    // Write driver file to system directory
    fs::write(&target_path, driver_data)?;

    // Install registry service
    install_service(driver_type).map_err(InstallError::RegistryError)?;

    Ok(())
}

fn uninstall_driver(driver_type: DriverType) -> Result<(), InstallError> {
    // Remove registry entries
    uninstall_service(driver_type).map_err(InstallError::RegistryError)?;

    // Remove driver file from system directory
    let target_filename = format!("{}.sys", driver_type.service_name());
    let target_path = Path::new(DRIVERS_PATH).join(&target_filename);

    if target_path.exists() {
        fs::remove_file(&target_path)?;
    }

    Ok(())
}

// System Detection Functions

fn get_windows_version() -> Result<WindowsNTVersion, String> {
    unsafe {
        let mut version_info = OSVERSIONINFOW {
            dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };

        if GetVersionExW(&mut version_info) == FALSE {
            return Err("Failed to get Windows version".to_string());
        }

        Ok(WindowsNTVersion {
            major: version_info.dwMajorVersion,
            minor: version_info.dwMinorVersion,
        })
    }
}

fn get_architecture() -> Result<Architecture, String> {
    unsafe {
        let mut system_info = SYSTEM_INFO::default();
        GetSystemInfo(&mut system_info);

        let architecture = match system_info.Anonymous.Anonymous.wProcessorArchitecture {
            PROCESSOR_ARCHITECTURE_INTEL => Architecture::X86,
            PROCESSOR_ARCHITECTURE_AMD64 => Architecture::AMD64,
            PROCESSOR_ARCHITECTURE_IA64 => Architecture::IA64,
            _ => return Err("Unsupported processor architecture".to_string()),
        };

        Ok(architecture)
    }
}

// Registry Service Functions

fn install_service(driver_type: DriverType) -> Result<(), String> {
    create_service(driver_type.service_name(), driver_type.display_name())?;
    add_class_filter(*driver_type.class_key(), driver_type.service_name())?;
    Ok(())
}

fn uninstall_service(driver_type: DriverType) -> Result<(), String> {
    remove_class_filter(*driver_type.class_key(), driver_type.service_name())?;
    delete_service(driver_type.service_name())?;
    Ok(())
}

fn create_service(service_name: &str, display_name: &str) -> Result<(), String> {
    let service_key = format!("{SERVICES_KEY}\\{service_name}");

    unsafe {
        let mut key: HKEY = ptr::null_mut();
        let service_key_wide = to_wide_string(&service_key);

        let result = RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            service_key_wide.as_ptr(),
            0,
            ptr::null(), // lpClass - can be null (input parameter)
            0,           // dwOptions
            KEY_ALL_ACCESS,
            ptr::null(), // lpSecurityAttributes - can be null (input parameter)
            &mut key,
            ptr::null_mut(), // lpdwDisposition - can be null (output parameter)
        );

        if result != ERROR_SUCCESS {
            return Err(format!("Failed to create service key: {result}"));
        }

        // Set DisplayName
        let display_name_wide = to_wide_string(display_name);
        RegSetValueExW(
            key,
            w!("DisplayName"),
            0,
            REG_SZ,
            display_name_wide.as_ptr() as *const u8,
            (display_name_wide.len() * 2) as u32,
        );

        // Set Type (kernel driver)
        RegSetValueExW(
            key,
            w!("Type"),
            0,
            REG_DWORD,
            1u32.to_le_bytes().as_ptr(),
            4,
        );

        // Set ErrorControl (normal)
        RegSetValueExW(
            key,
            w!("ErrorControl"),
            0,
            REG_DWORD,
            1u32.to_le_bytes().as_ptr(),
            4,
        );

        // Set Start (manual start)
        RegSetValueExW(
            key,
            w!("Start"),
            0,
            REG_DWORD,
            3u32.to_le_bytes().as_ptr(),
            4,
        );

        RegCloseKey(key);
    }

    Ok(())
}

fn delete_service(service_name: &str) -> Result<(), String> {
    let service_key = format!("{SERVICES_KEY}\\{service_name}");

    unsafe {
        let service_key_wide = to_wide_string(&service_key);
        let result = RegDeleteKeyW(HKEY_LOCAL_MACHINE, service_key_wide.as_ptr());

        if result != ERROR_SUCCESS {
            return Err(format!("Failed to delete service key: {result}"));
        }
    }

    Ok(())
}

fn add_class_filter(class_key: PCWSTR, filter_name: &str) -> Result<(), String> {
    unsafe {
        let mut key: HKEY = ptr::null_mut();

        let result = RegOpenKeyExW(HKEY_LOCAL_MACHINE, class_key, 0, KEY_ALL_ACCESS, &mut key);

        if result != ERROR_SUCCESS {
            return Err(format!("Failed to open class key: {result}"));
        }

        // Get current UpperFilters value
        let mut filters = get_upper_filters(key)?;

        // Add our filter if not already present
        if !filters.contains(&filter_name.to_string()) {
            filters.push(filter_name.to_string());
            set_upper_filters(key, &filters)?;
        }

        RegCloseKey(key);
    }

    Ok(())
}

fn remove_class_filter(class_key: PCWSTR, filter_name: &str) -> Result<(), String> {
    unsafe {
        let mut key: HKEY = ptr::null_mut();

        let result = RegOpenKeyExW(HKEY_LOCAL_MACHINE, class_key, 0, KEY_ALL_ACCESS, &mut key);

        if result != ERROR_SUCCESS {
            return Err(format!("Failed to open class key: {result}"));
        }

        // Get current UpperFilters value
        let mut filters = get_upper_filters(key)?;

        // Remove our filter
        filters.retain(|f| f != filter_name);

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

fn get_upper_filters(key: HKEY) -> Result<Vec<String>, String> {
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
            return Err(format!("Failed to read UpperFilters: {result}"));
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

fn set_upper_filters(key: HKEY, filters: &[String]) -> Result<(), String> {
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
            return Err(format!("Failed to set UpperFilters: {result}"));
        }
    }

    Ok(())
}

fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
