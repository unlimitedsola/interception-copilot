use crate::registry::RegistryManager;
use crate::system::SystemInfo;
use std::fs;
use std::io;
use std::path::Path;

const DRIVERS_PATH: &str = r"C:\Windows\System32\drivers";

#[derive(Debug, Clone, Copy)]
pub enum DriverType {
    Keyboard,
    Mouse,
}

impl DriverType {
    pub fn file_prefix(&self) -> &'static str {
        match self {
            DriverType::Keyboard => "KBDNT",
            DriverType::Mouse => "MOUNT",
        }
    }

    pub fn service_name(&self) -> &'static str {
        match self {
            DriverType::Keyboard => "keyboard",
            DriverType::Mouse => "mouse",
        }
    }
}

// Embedded driver files organized by type and system parameters
macro_rules! embed_driver {
    ($name:literal) => {
        include_bytes!(concat!("../drivers/", $name)) as &'static [u8]
    };
}

// Direct access to embedded drivers based on system parameters
fn get_embedded_driver_data(
    driver_type: DriverType,
    system_info: &SystemInfo,
) -> Result<&'static [u8], InstallError> {
    let prefix = system_info.get_driver_prefix();
    let arch = system_info.get_architecture_suffix();

    let driver_data = match (driver_type, prefix, arch) {
        // Keyboard drivers
        (DriverType::Keyboard, "51", "X86") => embed_driver!("KBDNT51X86.sys"),
        (DriverType::Keyboard, "52", "A64") => embed_driver!("KBDNT52A64.sys"),
        (DriverType::Keyboard, "52", "I64") => embed_driver!("KBDNT52I64.sys"),
        (DriverType::Keyboard, "52", "X86") => embed_driver!("KBDNT52X86.sys"),
        (DriverType::Keyboard, "60", "A64") => embed_driver!("KBDNT60A64.sys"),
        (DriverType::Keyboard, "60", "I64") => embed_driver!("KBDNT60I64.sys"),
        (DriverType::Keyboard, "60", "X86") => embed_driver!("KBDNT60X86.sys"),
        (DriverType::Keyboard, "61", "A64") => embed_driver!("KBDNT61A64.sys"),
        (DriverType::Keyboard, "61", "I64") => embed_driver!("KBDNT61I64.sys"),
        (DriverType::Keyboard, "61", "X86") => embed_driver!("KBDNT61X86.sys"),

        // Mouse drivers
        (DriverType::Mouse, "51", "X86") => embed_driver!("MOUNT51X86.sys"),
        (DriverType::Mouse, "52", "A64") => embed_driver!("MOUNT52A64.sys"),
        (DriverType::Mouse, "52", "I64") => embed_driver!("MOUNT52I64.sys"),
        (DriverType::Mouse, "52", "X86") => embed_driver!("MOUNT52X86.sys"),
        (DriverType::Mouse, "60", "A64") => embed_driver!("MOUNT60A64.sys"),
        (DriverType::Mouse, "60", "I64") => embed_driver!("MOUNT60I64.sys"),
        (DriverType::Mouse, "60", "X86") => embed_driver!("MOUNT60X86.sys"),
        (DriverType::Mouse, "61", "A64") => embed_driver!("MOUNT61A64.sys"),
        (DriverType::Mouse, "61", "I64") => embed_driver!("MOUNT61I64.sys"),
        (DriverType::Mouse, "61", "X86") => embed_driver!("MOUNT61X86.sys"),

        _ => {
            return Err(InstallError::DriverNotFound(format!(
                "No driver available for {driver_type:?} on Windows NT{prefix} {arch}"
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
    #[allow(dead_code)]
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

impl From<io::Error> for InstallError {
    fn from(err: io::Error) -> Self {
        InstallError::IoError(err)
    }
}

pub struct InterceptionInstaller {
    registry: RegistryManager,
}

impl Default for InterceptionInstaller {
    fn default() -> Self {
        Self::new()
    }
}

impl InterceptionInstaller {
    pub fn new() -> Self {
        Self {
            registry: RegistryManager::new(),
        }
    }

    pub fn install(&self) -> Result<(), InstallError> {
        println!("Detecting system configuration...");
        let system_info = SystemInfo::detect().map_err(InstallError::SystemDetectionFailed)?;

        println!(
            "System: Windows {} - {}",
            self.format_version(&system_info.version),
            self.format_architecture(&system_info.architecture)
        );

        // Install keyboard driver
        println!("Installing keyboard driver...");
        self.install_driver(&system_info, DriverType::Keyboard)?;

        // Install mouse driver
        println!("Installing mouse driver...");
        self.install_driver(&system_info, DriverType::Mouse)?;

        println!("Driver installation completed successfully.");
        println!();
        println!("IMPORTANT: You must reboot your system for the drivers to take effect.");

        Ok(())
    }

    pub fn uninstall(&self) -> Result<(), InstallError> {
        println!("Uninstalling Interception drivers...");

        // Uninstall keyboard driver
        println!("Removing keyboard driver...");
        self.uninstall_driver(DriverType::Keyboard)?;

        // Uninstall mouse driver
        println!("Removing mouse driver...");
        self.uninstall_driver(DriverType::Mouse)?;

        println!("Driver uninstallation completed successfully.");
        println!();
        println!("IMPORTANT: You must reboot your system for the changes to take effect.");

        Ok(())
    }

    fn install_driver(
        &self,
        system_info: &SystemInfo,
        driver_type: DriverType,
    ) -> Result<(), InstallError> {
        // Get embedded driver data directly
        let driver_data = get_embedded_driver_data(driver_type, system_info)?;

        // Target filename and path
        let target_filename = format!("{}.sys", driver_type.service_name());
        let target_path = Path::new(DRIVERS_PATH).join(&target_filename);

        // Write driver file to system directory
        fs::write(&target_path, driver_data)?;

        // Set up registry entries
        let driver_path = format!(r"\SystemRoot\system32\drivers\{target_filename}");

        match driver_type {
            DriverType::Keyboard => {
                self.registry
                    .install_keyboard_service(&driver_path)
                    .map_err(InstallError::RegistryError)?;
            }
            DriverType::Mouse => {
                self.registry
                    .install_mouse_service(&driver_path)
                    .map_err(InstallError::RegistryError)?;
            }
        }

        Ok(())
    }

    fn uninstall_driver(&self, driver_type: DriverType) -> Result<(), InstallError> {
        // Remove registry entries
        match driver_type {
            DriverType::Keyboard => {
                self.registry
                    .uninstall_keyboard_service()
                    .map_err(InstallError::RegistryError)?;
            }
            DriverType::Mouse => {
                self.registry
                    .uninstall_mouse_service()
                    .map_err(InstallError::RegistryError)?;
            }
        }

        // Remove driver file from system directory
        let target_filename = format!("{}.sys", driver_type.service_name());
        let target_path = Path::new(DRIVERS_PATH).join(&target_filename);

        if target_path.exists() {
            fs::remove_file(&target_path)?;
        }

        Ok(())
    }

    fn format_version(&self, version: &crate::system::WindowsVersion) -> &'static str {
        match version {
            crate::system::WindowsVersion::WindowsXP => "XP",
            crate::system::WindowsVersion::Windows2003 => "Server 2003",
            crate::system::WindowsVersion::WindowsVista => "Vista",
            crate::system::WindowsVersion::Windows7 => "7",
            crate::system::WindowsVersion::Windows8 => "8",
            crate::system::WindowsVersion::Windows81 => "8.1",
            crate::system::WindowsVersion::Windows10Plus => "10/11",
        }
    }

    fn format_architecture(&self, architecture: &crate::system::Architecture) -> &'static str {
        match architecture {
            crate::system::Architecture::X86 => "x86 (32-bit)",
            crate::system::Architecture::AMD64 => "x64 (64-bit)",
            crate::system::Architecture::IA64 => "IA-64",
        }
    }
}
