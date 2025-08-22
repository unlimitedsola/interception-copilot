use std::fmt::Display;
use std::num::NonZeroU32;
use std::{error, fmt, result};
use windows_sys::Win32::Foundation::{
    ERROR_INVALID_DATA, ERROR_MAPPED_ALIGNMENT, ERROR_UNSUPPORTED_TYPE, FALSE, WIN32_ERROR,
};
use windows_sys::Win32::System::SystemInformation::{
    GetSystemInfo, GetVersionExW, OSVERSIONINFOW, PROCESSOR_ARCHITECTURE_AMD64,
    PROCESSOR_ARCHITECTURE_IA64, PROCESSOR_ARCHITECTURE_INTEL, SYSTEM_INFO,
};

#[derive(Debug, Clone, Copy)]
pub struct SystemInfo {
    pub version: NTVersion,
    pub architecture: Architecture,
}

impl SystemInfo {
    pub fn detect() -> Result<Self> {
        let version = NTVersion::get()?;
        let architecture = Architecture::get()?;

        Ok(SystemInfo {
            version,
            architecture,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NTVersion {
    pub major: u32,
    pub minor: u32,
}

impl NTVersion {
    fn get() -> Result<Self> {
        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };

        if unsafe { GetVersionExW(&mut info) } == FALSE {
            return Err(Error::NTVersion);
        }

        Ok(NTVersion {
            major: info.dwMajorVersion,
            minor: info.dwMinorVersion,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Architecture {
    X86,
    AMD64,
    IA64,
}

impl Architecture {
    fn get() -> Result<Self> {
        let mut system_info = SYSTEM_INFO::default();
        unsafe { GetSystemInfo(&mut system_info) };

        let architecture = unsafe { system_info.Anonymous.Anonymous.wProcessorArchitecture };
        let architecture = match architecture {
            PROCESSOR_ARCHITECTURE_INTEL => Self::X86,
            PROCESSOR_ARCHITECTURE_AMD64 => Self::AMD64,
            PROCESSOR_ARCHITECTURE_IA64 => Self::IA64,
            _ => {
                return Err(Error::UnsupportedArchitecture);
            }
        };

        Ok(architecture)
    }
}

type Result<T = (), E = Error> = result::Result<T, E>;

#[derive(Debug, Copy, Clone)]
pub enum Error {
    NTVersion,
    UnsupportedArchitecture,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NTVersion => write!(f, "Failed to retrieve Windows version"),
            Self::UnsupportedArchitecture => write!(f, "Unsupported processor architecture"),
        }
    }
}

impl error::Error for Error {}
