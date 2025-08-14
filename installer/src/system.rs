use windows_sys::Win32::Foundation::FALSE;
use windows_sys::Win32::System::SystemInformation::{
    GetSystemInfo, GetVersionExW, OSVERSIONINFOW, SYSTEM_INFO,
};

#[derive(Debug, Clone, Copy)]
pub enum WindowsNTVersion {
    NT51, // Windows XP
    NT52, // Windows 2003
    NT60, // Windows Vista
    NT61, // Windows 7+
}

#[derive(Debug, Clone, Copy)]
pub enum ProcessorArchitecture {
    X86,
    A64,
    I64,
}

#[derive(Debug)]
pub struct SystemInfo {
    pub version: WindowsNTVersion,
    pub architecture: ProcessorArchitecture,
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

fn get_windows_version() -> Result<WindowsNTVersion, String> {
    unsafe {
        let mut version_info = OSVERSIONINFOW {
            dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
            dwMajorVersion: 0,
            dwMinorVersion: 0,
            dwBuildNumber: 0,
            dwPlatformId: 0,
            szCSDVersion: [0; 128],
        };

        if GetVersionExW(&mut version_info) == FALSE {
            return Err("Failed to get Windows version".to_string());
        }

        let version = match (version_info.dwMajorVersion, version_info.dwMinorVersion) {
            (5, 1) => WindowsNTVersion::NT51,  // Windows XP
            (5, 2) => WindowsNTVersion::NT52,  // Windows 2003
            (6, 0) => WindowsNTVersion::NT60,  // Windows Vista
            (6, 1) => WindowsNTVersion::NT61,  // Windows 7
            (6, 2) => WindowsNTVersion::NT61,  // Windows 8 - use NT61 drivers
            (6, 3) => WindowsNTVersion::NT61,  // Windows 8.1 - use NT61 drivers
            (10, _) => WindowsNTVersion::NT61, // Windows 10+ - use NT61 drivers
            _ => WindowsNTVersion::NT61,       // Default to NT61 for unknown versions
        };

        Ok(version)
    }
}

fn get_architecture() -> Result<ProcessorArchitecture, String> {
    unsafe {
        let mut system_info = std::mem::zeroed::<SYSTEM_INFO>();
        GetSystemInfo(&mut system_info);

        const PROCESSOR_ARCHITECTURE_INTEL: u16 = 0;
        const PROCESSOR_ARCHITECTURE_AMD64: u16 = 9;
        const PROCESSOR_ARCHITECTURE_IA64: u16 = 6;

        let architecture = match system_info.Anonymous.Anonymous.wProcessorArchitecture {
            PROCESSOR_ARCHITECTURE_INTEL => ProcessorArchitecture::X86,
            PROCESSOR_ARCHITECTURE_AMD64 => ProcessorArchitecture::A64,
            PROCESSOR_ARCHITECTURE_IA64 => ProcessorArchitecture::I64,
            _ => return Err("Unsupported processor architecture".to_string()),
        };

        Ok(architecture)
    }
}
