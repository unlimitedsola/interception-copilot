use crate::registry;
use crate::system::{Architecture, SystemInfo};
use std::fs;
use std::io;
use std::path::Path;

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
    pub fn class_key(&self) -> &'static windows_sys::core::PCWSTR {
        match self {
            Self::Keyboard => &crate::registry::KEYBOARD_CLASS_KEY,
            Self::Mouse => &crate::registry::MOUSE_CLASS_KEY,
        }
    }
}

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

fn install_driver(system_info: &SystemInfo, driver_type: DriverType) -> Result<(), InstallError> {
    // Get embedded driver data directly
    let driver_data = get_embedded_driver_data(driver_type, system_info)?;

    // Target filename and path
    let target_filename = format!("{}.sys", driver_type.service_name());
    let target_path = Path::new(DRIVERS_PATH).join(&target_filename);

    // Write driver file to system directory
    fs::write(&target_path, driver_data)?;

    // Install registry service using the function-based API
    registry::install_service(driver_type).map_err(InstallError::RegistryError)?;

    Ok(())
}

fn uninstall_driver(driver_type: DriverType) -> Result<(), InstallError> {
    // Remove registry entries using the function-based API
    registry::uninstall_service(driver_type).map_err(InstallError::RegistryError)?;

    // Remove driver file from system directory
    let target_filename = format!("{}.sys", driver_type.service_name());
    let target_path = Path::new(DRIVERS_PATH).join(&target_filename);

    if target_path.exists() {
        fs::remove_file(&target_path)?;
    }

    Ok(())
}
