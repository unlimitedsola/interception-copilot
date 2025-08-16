use windows_sys::Win32::Foundation::FALSE;
use windows_sys::Win32::System::SystemInformation::{
    GetSystemInfo, GetVersionExW, OSVERSIONINFOW, PROCESSOR_ARCHITECTURE_AMD64,
    PROCESSOR_ARCHITECTURE_IA64, PROCESSOR_ARCHITECTURE_INTEL, SYSTEM_INFO,
};

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
