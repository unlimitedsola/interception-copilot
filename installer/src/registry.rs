//! # Safety
//!
//! The `PCWSTR` pointers in this module always assume that the pointer is valid for reads
//! up until and including the next `\0`. This is a common requirement for Windows API
//! functions that deal with wide strings.

use crate::str::WCStr;
use std::fmt::Display;
use std::num::NonZeroU32;
use std::{error, fmt, ptr, result};
use windows_sys::Win32::Foundation::WIN32_ERROR;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
    HKEY_USERS, REG_DWORD, REG_OPEN_CREATE_OPTIONS, REG_QWORD, REG_SAM_FLAGS, REG_SZ,
    REG_VALUE_TYPE, RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegDeleteValueW, RegOpenKeyExW,
    RegSetValueExW,
};
use windows_sys::core::PCWSTR;

#[repr(transparent)]
#[derive(Debug)]
pub struct Key(HKEY);

/// Predefined registry keys.
impl Key {
    /// The predefined `HKEY_CLASSES_ROOT` registry key.
    pub const CLASSES_ROOT: &'static Key = &Key(HKEY_CLASSES_ROOT);

    /// The predefined `HKEY_CURRENT_CONFIG` registry key.
    pub const CURRENT_CONFIG: &'static Key = &Key(HKEY_CURRENT_CONFIG);

    /// The predefined `HKEY_CURRENT_USER` registry key.
    pub const CURRENT_USER: &'static Key = &Key(HKEY_CURRENT_USER);

    /// The predefined `HKEY_LOCAL_MACHINE` registry key.
    pub const LOCAL_MACHINE: &'static Key = &Key(HKEY_LOCAL_MACHINE);

    /// The predefined `HKEY_USERS` registry key.
    pub const USERS: &'static Key = &Key(HKEY_USERS);
}

/// Constructors
impl Key {
    /// # Safety
    ///
    /// The `PCWSTR` pointer needs to be valid for reads up until and including the next `\0`.
    pub unsafe fn create(
        &self,
        path: PCWSTR,
        options: REG_OPEN_CREATE_OPTIONS,
        access: REG_SAM_FLAGS,
    ) -> Result<Self> {
        let mut handle = ptr::null_mut();
        let res = unsafe {
            RegCreateKeyExW(
                self.0,
                path,
                0,
                ptr::null(),
                options,
                access,
                ptr::null(),
                &mut handle,
                ptr::null_mut(),
            )
        };
        win32_result(res).map(|_| Key(handle))
    }

    /// # Safety
    ///
    /// The `PCWSTR` pointer needs to be valid for reads up until and including the next `\0`.
    pub unsafe fn open(
        &self,
        path: PCWSTR,
        options: REG_OPEN_CREATE_OPTIONS,
        access: REG_SAM_FLAGS,
    ) -> Result<Self> {
        let mut handle = ptr::null_mut();
        let res = unsafe { RegOpenKeyExW(self.0, path, options, access, &mut handle) };
        win32_result(res).map(|_| Key(handle))
    }
}

/// Setters
impl Key {
    /// # Safety
    ///
    /// The `PCWSTR` pointer needs to be valid for reads up until and including the next `\0`.
    pub unsafe fn set_raw(&self, name: PCWSTR, value_type: REG_VALUE_TYPE, value: &[u8]) -> Result {
        let res = unsafe {
            RegSetValueExW(
                self.0,
                name,
                0,
                value_type,
                value.as_ptr(),
                value.len() as u32,
            )
        };
        win32_result(res)
    }

    /// # Safety
    ///
    /// The `PCWSTR` pointer needs to be valid for reads up until and including the next `\0`.
    pub unsafe fn set<V: RegValue>(&self, name: PCWSTR, value: V) -> Result {
        unsafe { self.set_raw(name, V::VALUE_TYPE, value.as_bytes().as_ref()) }
    }
}

/// Delete operations
impl Key {
    /// # Safety
    ///
    /// The `PCWSTR` pointer needs to be valid for reads up until and including the next `\0`.
    pub unsafe fn delete_key(&self, path: PCWSTR) -> Result {
        let res = unsafe { RegDeleteKeyW(self.0, path) };
        win32_result(res)
    }

    /// # Safety
    ///
    /// The `PCWSTR` pointer needs to be valid for reads up until and including the next `\0`.
    pub unsafe fn delete_value(&self, name: PCWSTR) -> Result {
        let res = unsafe { RegDeleteValueW(self.0, name) };
        win32_result(res)
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        };
    }
}

pub trait RegValue: Sized {
    const VALUE_TYPE: REG_VALUE_TYPE;

    fn as_bytes(&self) -> impl AsRef<[u8]>;
}

impl RegValue for u32 {
    const VALUE_TYPE: REG_VALUE_TYPE = REG_DWORD;

    fn as_bytes(&self) -> impl AsRef<[u8]> {
        self.to_le_bytes()
    }
}

impl RegValue for u64 {
    const VALUE_TYPE: REG_VALUE_TYPE = REG_QWORD;

    fn as_bytes(&self) -> impl AsRef<[u8]> {
        self.to_le_bytes()
    }
}

impl RegValue for &WCStr {
    const VALUE_TYPE: REG_VALUE_TYPE = REG_SZ;

    fn as_bytes(&self) -> impl AsRef<[u8]> {
        WCStr::as_bytes(self)
    }
}

type Result<T = (), E = Error> = result::Result<T, E>;

const fn win32_result(result: WIN32_ERROR) -> Result {
    match NonZeroU32::new(result) {
        None => Ok(()),
        Some(code) => Err(Error(code)),
    }
}

#[derive(Debug)]
pub struct Error(NonZeroU32);

const _: () = {
    ["Result is niche optimized"][size_of::<Result>() - size_of::<WIN32_ERROR>()];
};

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Registry error: {}", self.0)
    }
}

impl error::Error for Error {}
