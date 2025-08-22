//! Minimalistic Windows Registry API wrapper.
//!
//! # Safety
//!
//! The `PCWSTR` pointers in this module always assume that the pointer is valid for reads
//! up until and including the next `\0`. This is a common requirement for Windows API
//! functions that deal with wide strings.

use crate::wcstr::{NotNulTerminatedError, WCStr};
use std::array::TryFromSliceError;
use std::fmt::Display;
use std::num::NonZeroU32;
use std::{error, fmt, ptr, result};
use windows_sys::Win32::Foundation::{
    ERROR_INVALID_DATA, ERROR_MAPPED_ALIGNMENT, ERROR_UNSUPPORTED_TYPE, WIN32_ERROR,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
    HKEY_USERS, REG_DWORD, REG_MULTI_SZ, REG_OPEN_CREATE_OPTIONS, REG_QWORD, REG_SAM_FLAGS, REG_SZ,
    REG_VALUE_TYPE, RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegDeleteValueW, RegOpenKeyExW,
    RegQueryValueExW, RegSetValueExW,
};
use windows_sys::core::PCWSTR;

#[repr(transparent)]
#[derive(Debug)]
pub struct Key(HKEY);

/// Predefined registry keys.
#[allow(dead_code)]
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

/// Getters
impl Key {
    /// # Safety
    ///
    /// The `PCWSTR` pointer needs to be valid for reads up until and including the next `\0`.
    pub unsafe fn get_raw(&self, name: PCWSTR) -> Result<(REG_VALUE_TYPE, Vec<u8>)> {
        let mut value_type = 0;
        let mut data_len = 0;

        // query size
        let res = unsafe {
            RegQueryValueExW(
                self.0,
                name,
                ptr::null(),
                &mut value_type,
                ptr::null_mut(),
                &mut data_len,
            )
        };
        win32_result(res)?;

        // allocate buffer and query again
        // on all windows platforms, the allocated buffer is guaranteed to be aligned
        // to the minimum alignment of a `u16`, thus we can safely use `u8` as the type
        // for the buffer for both `REG_SZ` and `REG_MULTI_SZ` values.
        let mut data = vec![0u8; data_len as usize];
        let res = unsafe {
            RegQueryValueExW(
                self.0,
                name,
                ptr::null(),
                &mut value_type,
                data.as_mut_ptr(),
                &mut data_len,
            )
        };
        data.truncate(data_len as usize);

        win32_result(res).map(|_| (value_type, data))
    }

    /// # Safety
    ///
    /// The `PCWSTR` pointer needs to be valid for reads up until and including the next `\0`.
    pub unsafe fn get(&self, name: PCWSTR) -> Result<Value> {
        let (value_type, data) = unsafe { self.get_raw(name) }?;
        Value::from_bytes(value_type, &data)
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
    pub unsafe fn set<V: IntoValue>(&self, name: PCWSTR, value: V) -> Result {
        unsafe { self.set_raw(name, V::VALUE_TYPE, value.into_bytes().as_ref()) }
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

pub trait IntoValue: Sized {
    const VALUE_TYPE: REG_VALUE_TYPE;

    fn into_bytes(self) -> impl AsRef<[u8]>;
}

impl IntoValue for u32 {
    const VALUE_TYPE: REG_VALUE_TYPE = REG_DWORD;

    fn into_bytes(self) -> impl AsRef<[u8]> {
        self.to_le_bytes()
    }
}

impl IntoValue for u64 {
    const VALUE_TYPE: REG_VALUE_TYPE = REG_QWORD;

    fn into_bytes(self) -> impl AsRef<[u8]> {
        self.to_le_bytes()
    }
}

impl IntoValue for &WCStr {
    const VALUE_TYPE: REG_VALUE_TYPE = REG_SZ;

    fn into_bytes(self) -> impl AsRef<[u8]> {
        WCStr::as_bytes(self)
    }
}

impl<S: AsRef<WCStr>> IntoValue for &[S] {
    const VALUE_TYPE: REG_VALUE_TYPE = REG_MULTI_SZ;

    fn into_bytes(self) -> impl AsRef<[u8]> {
        let mut bytes = Vec::new();
        for w_str in self {
            bytes.extend_from_slice(WCStr::as_bytes(w_str.as_ref()));
            bytes.extend_from_slice(&0u16.to_le_bytes()); // null terminator for each string
        }
        bytes.extend_from_slice(&0u16.to_le_bytes()); // final null terminator for the multi-string
        bytes
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Value {
    U32(u32),
    U64(u64),
    String(Box<WCStr>),
    MultiString(Vec<Box<WCStr>>),
}

impl Value {
    fn from_bytes(value_type: REG_VALUE_TYPE, bytes: &[u8]) -> Result<Self> {
        match value_type {
            REG_DWORD => Ok(Value::U32(u32::from_le_bytes(bytes.try_into()?))),
            REG_QWORD => Ok(Value::U64(u64::from_le_bytes(bytes.try_into()?))),
            REG_SZ => {
                let wide = to_u16_slice(bytes)?;
                Ok(Value::String(WCStr::try_from_slice(wide)?.into()))
            }
            REG_MULTI_SZ => {
                let wide = to_u16_slice(bytes)?;
                Ok(Value::MultiString(parse_multi_string(wide)?))
            }
            _ => Err(Error::UNSUPPORTED_TYPE),
        }
    }
}

fn to_u16_slice(bytes: &[u8]) -> Result<&[u16]> {
    match unsafe { bytes.align_to() } {
        ([], v, []) => Ok(v),
        _ => Err(Error::MAPPED_ALIGNMENT),
    }
}

fn parse_multi_string(wide: &[u16]) -> Result<Vec<Box<WCStr>>> {
    // Multi-string value is a sequence of wide strings, each terminated by a null character,
    // and the entire sequence is also terminated by an additional null character.
    if wide.is_empty() || wide.last().copied() != Some(0) {
        return Err(Error::INVALID_DATA);
    }

    let mut values = Vec::new();

    let mut start = 0;
    for (i, &c) in wide.iter().enumerate() {
        if c == 0 {
            if start < i {
                let w_str = WCStr::try_from_slice(&wide[start..=i])?;
                values.push(w_str.into());
                start = i + 1; // Move past the null terminator
            } else {
                break;
            }
        }
    }

    Ok(values)
}

type Result<T = (), E = Error> = result::Result<T, E>;

const fn win32_result(result: WIN32_ERROR) -> Result {
    match NonZeroU32::new(result) {
        None => Ok(()),
        Some(code) => Err(Error(code)),
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Error(NonZeroU32);

const _: () = {
    ["Result is niche optimized"][size_of::<Result>() - size_of::<WIN32_ERROR>()];
};

impl Error {
    pub const INVALID_DATA: Error = Error::from_win32(ERROR_INVALID_DATA);
    pub const MAPPED_ALIGNMENT: Error = Error::from_win32(ERROR_MAPPED_ALIGNMENT);
    pub const UNSUPPORTED_TYPE: Error = Error::from_win32(ERROR_UNSUPPORTED_TYPE);

    const fn from_win32(code: WIN32_ERROR) -> Self {
        match NonZeroU32::new(code) {
            Some(v) => Error(v),
            None => panic!("Cannot create Error from zero value"),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Registry error: {}", self.0)
    }
}

impl error::Error for Error {}

impl From<TryFromSliceError> for Error {
    fn from(_: TryFromSliceError) -> Self {
        Error::INVALID_DATA
    }
}

impl From<NotNulTerminatedError> for Error {
    fn from(_: NotNulTerminatedError) -> Self {
        Error::INVALID_DATA
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_min_align() {
        // Verify that all allocated buffers are aligned to the minimum alignment of a `u16`.
        for size in (2..=64).step_by(2) {
            let value = vec![0u8; size];
            match unsafe { value.align_to::<u16>() } {
                ([], _, []) => {}
                _ => {
                    panic!("Value of size {} is not aligned", size);
                }
            }
        }
    }

    #[test]
    fn test_multi_string_parsing() {
        let wide = [
            0x48, 0x65, 0x6C, 0x6C, 0x6F, 0, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0, 0,
        ];
        let result = parse_multi_string(&wide).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].char_len(), 5);
        assert_eq!(result[1].char_len(), 5);
        assert_eq!(result[0].as_wide(), &wide[..5]);
        assert_eq!(result[1].as_wide(), &wide[6..11]);
    }
}
