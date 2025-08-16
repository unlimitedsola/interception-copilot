use crate::installer::DriverType;
use std::ptr;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, REG_DWORD, REG_MULTI_SZ, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteKeyW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW,
};
use windows_sys::core::PCWSTR;
use windows_sys::w;

const SERVICES_KEY: &str = r"SYSTEM\CurrentControlSet\Services";
pub const KEYBOARD_CLASS_KEY: PCWSTR =
    w!(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e96b-e325-11ce-bfc1-08002be10318}");
pub const MOUSE_CLASS_KEY: PCWSTR =
    w!(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e96f-e325-11ce-bfc1-08002be10318}");

pub struct RegistryManager;

impl Default for RegistryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryManager {
    pub fn new() -> Self {
        Self
    }

    /// Install a service for the specified driver type
    pub fn install_service(&self, driver_type: DriverType) -> Result<(), String> {
        self.create_service(driver_type.service_name(), driver_type.display_name())?;
        self.add_class_filter(*driver_type.class_key(), driver_type.service_name())?;
        Ok(())
    }

    /// Uninstall a service for the specified driver type
    pub fn uninstall_service(&self, driver_type: DriverType) -> Result<(), String> {
        self.remove_class_filter(*driver_type.class_key(), driver_type.service_name())?;
        self.delete_service(driver_type.service_name())?;
        Ok(())
    }

    fn create_service(&self, service_name: &str, display_name: &str) -> Result<(), String> {
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
            let driver_type: u32 = 1;
            RegSetValueExW(
                key,
                w!("Type"),
                0,
                REG_DWORD,
                &driver_type as *const u32 as *const u8,
                4,
            );

            // Set ErrorControl (normal)
            let error_control: u32 = 1;
            RegSetValueExW(
                key,
                w!("ErrorControl"),
                0,
                REG_DWORD,
                &error_control as *const u32 as *const u8,
                4,
            );

            // Set Start (manual start)
            let start_type: u32 = 3;
            RegSetValueExW(
                key,
                w!("Start"),
                0,
                REG_DWORD,
                &start_type as *const u32 as *const u8,
                4,
            );

            RegCloseKey(key);
        }

        Ok(())
    }

    fn delete_service(&self, service_name: &str) -> Result<(), String> {
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

    fn add_class_filter(&self, class_key: PCWSTR, filter_name: &str) -> Result<(), String> {
        unsafe {
            let mut key: HKEY = ptr::null_mut();

            let result = RegOpenKeyExW(HKEY_LOCAL_MACHINE, class_key, 0, KEY_ALL_ACCESS, &mut key);

            if result != ERROR_SUCCESS {
                return Err(format!("Failed to open class key: {result}"));
            }

            // Get current UpperFilters value
            let mut filters = self.get_upper_filters(key)?;

            // Add our filter if not already present
            if !filters.contains(&filter_name.to_string()) {
                filters.push(filter_name.to_string());
                self.set_upper_filters(key, &filters)?;
            }

            RegCloseKey(key);
        }

        Ok(())
    }

    fn remove_class_filter(&self, class_key: PCWSTR, filter_name: &str) -> Result<(), String> {
        unsafe {
            let mut key: HKEY = ptr::null_mut();

            let result = RegOpenKeyExW(HKEY_LOCAL_MACHINE, class_key, 0, KEY_ALL_ACCESS, &mut key);

            if result != ERROR_SUCCESS {
                return Err(format!("Failed to open class key: {result}"));
            }

            // Get current UpperFilters value
            let mut filters = self.get_upper_filters(key)?;

            // Remove our filter
            filters.retain(|f| f != filter_name);

            if filters.is_empty() {
                // Delete the UpperFilters value if no filters remain
                RegDeleteValueW(key, w!("UpperFilters"));
            } else {
                self.set_upper_filters(key, &filters)?;
            }

            RegCloseKey(key);
        }

        Ok(())
    }

    fn get_upper_filters(&self, key: HKEY) -> Result<Vec<String>, String> {
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

    fn set_upper_filters(&self, key: HKEY, filters: &[String]) -> Result<(), String> {
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
}

fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
