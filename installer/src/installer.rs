use crate::registry::RegistryManager;
use crate::system::SystemInfo;
use std::fs;
use std::io;
use std::path::Path;

const DRIVERS_PATH: &str = r"C:\Windows\System32\drivers";

// Embedded driver files
macro_rules! embed_driver {
    ($name:literal) => {
        ($name, include_bytes!(concat!("../drivers/", $name)))
    };
}

// All driver files embedded in the binary
const EMBEDDED_DRIVERS: &[(&str, &[u8])] = &[
    embed_driver!("KBDNT51X86.sys"),
    embed_driver!("KBDNT52A64.sys"),
    embed_driver!("KBDNT52I64.sys"),
    embed_driver!("KBDNT52X86.sys"),
    embed_driver!("KBDNT60A64.sys"),
    embed_driver!("KBDNT60I64.sys"),
    embed_driver!("KBDNT60X86.sys"),
    embed_driver!("KBDNT61A64.sys"),
    embed_driver!("KBDNT61I64.sys"),
    embed_driver!("KBDNT61X86.sys"),
    embed_driver!("MOUNT51X86.sys"),
    embed_driver!("MOUNT52A64.sys"),
    embed_driver!("MOUNT52I64.sys"),
    embed_driver!("MOUNT52X86.sys"),
    embed_driver!("MOUNT60A64.sys"),
    embed_driver!("MOUNT60I64.sys"),
    embed_driver!("MOUNT60X86.sys"),
    embed_driver!("MOUNT61A64.sys"),
    embed_driver!("MOUNT61I64.sys"),
    embed_driver!("MOUNT61X86.sys"),
];

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
        self.install_driver(&system_info, "keyboard", "KBDNT")?;

        // Install mouse driver
        println!("Installing mouse driver...");
        self.install_driver(&system_info, "mouse", "MOUNT")?;

        println!("Driver installation completed successfully.");
        println!();
        println!("IMPORTANT: You must reboot your system for the drivers to take effect.");

        Ok(())
    }

    pub fn uninstall(&self) -> Result<(), InstallError> {
        println!("Uninstalling Interception drivers...");

        // Uninstall keyboard driver
        println!("Removing keyboard driver...");
        self.uninstall_driver("keyboard", "KBDNT")?;

        // Uninstall mouse driver
        println!("Removing mouse driver...");
        self.uninstall_driver("mouse", "MOUNT")?;

        println!("Driver uninstallation completed successfully.");
        println!();
        println!("IMPORTANT: You must reboot your system for the changes to take effect.");

        Ok(())
    }

    fn install_driver(
        &self,
        system_info: &SystemInfo,
        driver_type: &str,
        file_prefix: &str,
    ) -> Result<(), InstallError> {
        // Determine source and target file names
        let source_filename = format!(
            "{}{}{}.sys",
            file_prefix,
            system_info.get_driver_prefix(),
            system_info.get_architecture_suffix()
        );

        let target_filename = format!("{driver_type}.sys");
        let target_path = Path::new(DRIVERS_PATH).join(&target_filename);

        // Find embedded driver data
        let driver_data = self.get_embedded_driver(&source_filename)?;

        // Write driver file to system directory
        fs::write(&target_path, driver_data)?;

        // Set up registry entries
        let driver_path = format!(r"\SystemRoot\system32\drivers\{target_filename}");

        match driver_type {
            "keyboard" => {
                self.registry
                    .install_keyboard_service(&driver_path)
                    .map_err(InstallError::RegistryError)?;
            }
            "mouse" => {
                self.registry
                    .install_mouse_service(&driver_path)
                    .map_err(InstallError::RegistryError)?;
            }
            _ => {
                return Err(InstallError::DriverNotFound(format!(
                    "Unknown driver type: {driver_type}"
                )));
            }
        }

        Ok(())
    }

    fn uninstall_driver(&self, driver_type: &str, _file_prefix: &str) -> Result<(), InstallError> {
        // Remove registry entries
        match driver_type {
            "keyboard" => {
                self.registry
                    .uninstall_keyboard_service()
                    .map_err(InstallError::RegistryError)?;
            }
            "mouse" => {
                self.registry
                    .uninstall_mouse_service()
                    .map_err(InstallError::RegistryError)?;
            }
            _ => {
                return Err(InstallError::DriverNotFound(format!(
                    "Unknown driver type: {driver_type}"
                )));
            }
        }

        // Remove driver file from system directory
        let target_filename = format!("{driver_type}.sys");
        let target_path = Path::new(DRIVERS_PATH).join(&target_filename);

        if target_path.exists() {
            fs::remove_file(&target_path)?;
        }

        Ok(())
    }

    fn get_embedded_driver(&self, filename: &str) -> Result<&'static [u8], InstallError> {
        EMBEDDED_DRIVERS
            .iter()
            .find(|(name, _)| *name == filename)
            .map(|(_, data)| *data)
            .ok_or_else(|| {
                InstallError::DriverNotFound(format!(
                    "Driver file not found in embedded data: {filename}. Available drivers: {}",
                    EMBEDDED_DRIVERS
                        .iter()
                        .map(|(name, _)| *name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
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
