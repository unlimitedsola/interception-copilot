use windows_sys::Win32::Foundation::FALSE;
use windows_sys::Win32::System::SystemInformation::{
    GetSystemInfo, GetVersionExW, OSVERSIONINFOW, SYSTEM_INFO,
};

#[derive(Debug, Clone, Copy)]
pub enum WindowsVersion {
    WindowsXP,     // 5.1
    Windows2003,   // 5.2
    WindowsVista,  // 6.0
    Windows7,      // 6.1
    Windows8,      // 6.2
    Windows81,     // 6.3
    Windows10Plus, // 10.0+
}

#[derive(Debug, Clone, Copy)]
pub enum Architecture {
    X86,
    AMD64,
    IA64,
}

#[derive(Debug)]
pub struct SystemInfo {
    pub version: WindowsVersion,
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

    pub fn get_driver_prefix(&self) -> &'static str {
        match self.version {
            WindowsVersion::WindowsXP => "51",
            WindowsVersion::Windows2003 => "52",
            WindowsVersion::WindowsVista => "60",
            WindowsVersion::Windows7 => "61",
            WindowsVersion::Windows8 => "61", // Use 61 (Windows 7) drivers for Windows 8+
            WindowsVersion::Windows81 => "61", // Use 61 (Windows 7) drivers for Windows 8+
            WindowsVersion::Windows10Plus => "61", // Use 61 (Windows 7) drivers for Windows 10+
        }
    }

    pub fn get_architecture_suffix(&self) -> &'static str {
        match self.architecture {
            Architecture::X86 => "X86",
            Architecture::AMD64 => "A64",
            Architecture::IA64 => "I64",
        }
    }
}

fn get_windows_version() -> Result<WindowsVersion, String> {
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
            (5, 1) => WindowsVersion::WindowsXP,
            (5, 2) => WindowsVersion::Windows2003,
            (6, 0) => WindowsVersion::WindowsVista,
            (6, 1) => WindowsVersion::Windows7,
            (6, 2) => WindowsVersion::Windows8,
            (6, 3) => WindowsVersion::Windows81,
            (10, _) => WindowsVersion::Windows10Plus,
            _ => WindowsVersion::Windows10Plus, // Default to newest for unknown versions
        };

        Ok(version)
    }
}

fn get_architecture() -> Result<Architecture, String> {
    unsafe {
        let mut system_info = std::mem::zeroed::<SYSTEM_INFO>();
        GetSystemInfo(&mut system_info);

        const PROCESSOR_ARCHITECTURE_INTEL: u16 = 0;
        const PROCESSOR_ARCHITECTURE_AMD64: u16 = 9;
        const PROCESSOR_ARCHITECTURE_IA64: u16 = 6;

        let architecture = match system_info.Anonymous.Anonymous.wProcessorArchitecture {
            PROCESSOR_ARCHITECTURE_INTEL => Architecture::X86,
            PROCESSOR_ARCHITECTURE_AMD64 => Architecture::AMD64,
            PROCESSOR_ARCHITECTURE_IA64 => Architecture::IA64,
            _ => return Err("Unsupported processor architecture".to_string()),
        };

        Ok(architecture)
    }
}
