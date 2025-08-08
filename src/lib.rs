//! # Interception Copilot
//!
//! Rust port of the [Interception library](https://github.com/oblitum/Interception)
//! using `windows-sys` with a safe API for intercepting keyboard and mouse input on Windows.
//!
//! The Interception library allows you to intercept and modify keyboard and mouse input
//! at a low level on Windows systems. This Rust port provides both unsafe bindings
//! to the original C API and safe wrappers for convenient use.
//!
//! ## Example
//!
//! ```rust,no_run
//! use interception_copilot::{Context, Device, Filter};
//!
//! // Create an interception context
//! let context = Context::new().expect("Failed to create interception context");
//!
//! // Set filter to capture all keyboard input
//! context.set_filter(Device::is_keyboard, Filter::KEY_ALL)
//!     .expect("Failed to set keyboard filter");
//!
//! // Wait for keyboard events and process them
//! loop {
//!     if let Some(device) = context.wait() {
//!         // Receive and process keyboard/mouse strokes
//!         // ...
//!     }
//! }
//! ```

#![cfg(windows)]

use std::ffi::{c_int, c_long, c_short, c_uint, c_ulong, c_ushort};
use std::mem;
use std::ptr;
use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_NONE;
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, FALSE, GetLastError, HANDLE, INVALID_HANDLE_VALUE, TRUE, WAIT_FAILED,
        WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    Storage::FileSystem::{CreateFileW, OPEN_EXISTING},
    System::{
        IO::DeviceIoControl,
        Threading::{CreateEventW, INFINITE, WaitForMultipleObjects},
    },
};

// Add GENERIC_READ constant since it seems to be missing from windows-sys
const GENERIC_READ: u32 = 0x80000000;

// Constants from the original C header
const INTERCEPTION_MAX_KEYBOARD: usize = 10;
const INTERCEPTION_MAX_MOUSE: usize = 10;
const INTERCEPTION_MAX_DEVICE: usize = INTERCEPTION_MAX_KEYBOARD + INTERCEPTION_MAX_MOUSE;

// Define constants not available in windows-sys
const FILE_DEVICE_UNKNOWN: u32 = 0x00000022;
const METHOD_BUFFERED: u32 = 0;
const FILE_ANY_ACCESS: u32 = 0;

// IOCTL codes from the original C implementation
const IOCTL_SET_PRECEDENCE: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x801, METHOD_BUFFERED, FILE_ANY_ACCESS);
const IOCTL_GET_PRECEDENCE: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x802, METHOD_BUFFERED, FILE_ANY_ACCESS);
const IOCTL_SET_FILTER: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x804, METHOD_BUFFERED, FILE_ANY_ACCESS);
const IOCTL_GET_FILTER: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x808, METHOD_BUFFERED, FILE_ANY_ACCESS);
const IOCTL_SET_EVENT: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 0x810, METHOD_BUFFERED, FILE_ANY_ACCESS);
const IOCTL_WRITE: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 0x820, METHOD_BUFFERED, FILE_ANY_ACCESS);
const IOCTL_READ: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 0x840, METHOD_BUFFERED, FILE_ANY_ACCESS);
const IOCTL_GET_HARDWARE_ID: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x880, METHOD_BUFFERED, FILE_ANY_ACCESS);

// Helper function to construct IOCTL codes
const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

/// Device types for the Interception library
pub type Device = c_int;

/// Precedence value for device handling order
pub type Precedence = c_int;

/// Function type for device predicates
pub type PredicateFn = fn(Device) -> bool;

/// Keyboard device constructor
#[inline]
pub const fn keyboard(index: usize) -> Device {
    (index as i32) + 1
}

/// Mouse device constructor  
#[inline]
pub const fn mouse(index: usize) -> Device {
    (INTERCEPTION_MAX_KEYBOARD as i32) + (index as i32) + 1
}

/// Keyboard key state flags
pub type KeyState = c_int;

/// Key down event
pub const KEY_DOWN: KeyState = 0x00;
/// Key up event
pub const KEY_UP: KeyState = 0x01;
/// Extended key code (E0 prefix)
pub const KEY_E0: KeyState = 0x02;
/// Extended key code (E1 prefix)
pub const KEY_E1: KeyState = 0x04;
/// Terminal Services LED update
pub const KEY_TERMSRV_SET_LED: KeyState = 0x08;
/// Terminal Services shadow
pub const KEY_TERMSRV_SHADOW: KeyState = 0x10;
/// Terminal Services virtual key packet
pub const KEY_TERMSRV_VKPACKET: KeyState = 0x20;

/// Mouse button and wheel state flags  
pub type MouseState = c_int;

/// Left mouse button down
pub const MOUSE_LEFT_BUTTON_DOWN: MouseState = 0x001;
/// Left mouse button up
pub const MOUSE_LEFT_BUTTON_UP: MouseState = 0x002;
/// Right mouse button down
pub const MOUSE_RIGHT_BUTTON_DOWN: MouseState = 0x004;
/// Right mouse button up
pub const MOUSE_RIGHT_BUTTON_UP: MouseState = 0x008;
/// Middle mouse button down
pub const MOUSE_MIDDLE_BUTTON_DOWN: MouseState = 0x010;
/// Middle mouse button up
pub const MOUSE_MIDDLE_BUTTON_UP: MouseState = 0x020;
/// Mouse button 4 down
pub const MOUSE_BUTTON_4_DOWN: MouseState = 0x040;
/// Mouse button 4 up
pub const MOUSE_BUTTON_4_UP: MouseState = 0x080;
/// Mouse button 5 down
pub const MOUSE_BUTTON_5_DOWN: MouseState = 0x100;
/// Mouse button 5 up
pub const MOUSE_BUTTON_5_UP: MouseState = 0x200;
/// Mouse wheel scroll
pub const MOUSE_WHEEL: MouseState = 0x400;
/// Mouse horizontal wheel scroll
pub const MOUSE_HWHEEL: MouseState = 0x800;

/// Mouse button 1 down (alias for left button)
pub const MOUSE_BUTTON_1_DOWN: MouseState = MOUSE_LEFT_BUTTON_DOWN;
/// Mouse button 1 up (alias for left button)
pub const MOUSE_BUTTON_1_UP: MouseState = MOUSE_LEFT_BUTTON_UP;
/// Mouse button 2 down (alias for right button)
pub const MOUSE_BUTTON_2_DOWN: MouseState = MOUSE_RIGHT_BUTTON_DOWN;
/// Mouse button 2 up (alias for right button)
pub const MOUSE_BUTTON_2_UP: MouseState = MOUSE_RIGHT_BUTTON_UP;
/// Mouse button 3 down (alias for middle button)
pub const MOUSE_BUTTON_3_DOWN: MouseState = MOUSE_MIDDLE_BUTTON_DOWN;
/// Mouse button 3 up (alias for middle button)
pub const MOUSE_BUTTON_3_UP: MouseState = MOUSE_MIDDLE_BUTTON_UP;

/// Mouse movement flags
pub type MouseFlag = c_int;

/// Relative movement
pub const MOUSE_MOVE_RELATIVE: MouseFlag = 0x000;
/// Absolute movement
pub const MOUSE_MOVE_ABSOLUTE: MouseFlag = 0x001;
/// Virtual desktop coordinates
pub const MOUSE_VIRTUAL_DESKTOP: MouseFlag = 0x002;
/// Mouse attributes changed
pub const MOUSE_ATTRIBUTES_CHANGED: MouseFlag = 0x004;
/// Don't coalesce mouse movements
pub const MOUSE_MOVE_NOCOALESCE: MouseFlag = 0x008;
/// Terminal Services source shadow
pub const MOUSE_TERMSRV_SRC_SHADOW: MouseFlag = 0x100;

/// Filter bitmask for selecting which events to intercept
pub type Filter = c_ushort;

/// No filtering
pub const FILTER_NONE: Filter = 0x0000;
/// Filter all events
pub const FILTER_ALL: Filter = 0xFFFF;

// Keyboard filters
/// Filter key down events
pub const FILTER_KEY_DOWN: Filter = 0x01;
/// Filter key up events
pub const FILTER_KEY_UP: Filter = 0x02;
/// Filter E0 extended keys
pub const FILTER_KEY_E0: Filter = 0x08;
/// Filter E1 extended keys
pub const FILTER_KEY_E1: Filter = 0x016;

// Mouse button filters
/// Filter left mouse button down
pub const FILTER_MOUSE_LEFT_BUTTON_DOWN: Filter = 0x001;
/// Filter left mouse button up
pub const FILTER_MOUSE_LEFT_BUTTON_UP: Filter = 0x002;
/// Filter right mouse button down
pub const FILTER_MOUSE_RIGHT_BUTTON_DOWN: Filter = 0x004;
/// Filter right mouse button up
pub const FILTER_MOUSE_RIGHT_BUTTON_UP: Filter = 0x008;
/// Filter middle mouse button down
pub const FILTER_MOUSE_MIDDLE_BUTTON_DOWN: Filter = 0x010;
/// Filter middle mouse button up
pub const FILTER_MOUSE_MIDDLE_BUTTON_UP: Filter = 0x020;
/// Filter mouse button 4 down
pub const FILTER_MOUSE_BUTTON_4_DOWN: Filter = 0x040;
/// Filter mouse button 4 up
pub const FILTER_MOUSE_BUTTON_4_UP: Filter = 0x080;
/// Filter mouse button 5 down
pub const FILTER_MOUSE_BUTTON_5_DOWN: Filter = 0x100;
/// Filter mouse button 5 up
pub const FILTER_MOUSE_BUTTON_5_UP: Filter = 0x200;
/// Filter mouse wheel
pub const FILTER_MOUSE_WHEEL: Filter = 0x400;
/// Filter mouse horizontal wheel
pub const FILTER_MOUSE_HWHEEL: Filter = 0x800;
/// Filter mouse movement
pub const FILTER_MOUSE_MOVE: Filter = 0x1000;

/// No keyboard filtering
pub const FILTER_KEY_NONE: Filter = FILTER_NONE;
/// Filter all keyboard events
pub const FILTER_KEY_ALL: Filter = FILTER_ALL;
/// No mouse filtering
pub const FILTER_MOUSE_NONE: Filter = FILTER_NONE;
/// Filter all mouse events
pub const FILTER_MOUSE_ALL: Filter = FILTER_ALL;

/// A keyboard stroke event
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct KeyStroke {
    /// Virtual key code
    pub code: c_ushort,
    /// Key state flags
    pub state: c_ushort,
    /// Additional information
    pub information: c_uint,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of KeyStroke"][size_of::<KeyStroke>() - 8usize];
    ["Alignment of KeyStroke"][align_of::<KeyStroke>() - 4usize];
    ["Offset of field: KeyStroke::code"][mem::offset_of!(KeyStroke, code) - 0usize];
    ["Offset of field: KeyStroke::state"][mem::offset_of!(KeyStroke, state) - 2usize];
    ["Offset of field: KeyStroke::information"][mem::offset_of!(KeyStroke, information) - 4usize];
};

/// A mouse stroke event  
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MouseStroke {
    /// Mouse state flags
    pub state: c_ushort,
    /// Mouse movement flags
    pub flags: c_ushort,
    /// Mouse wheel delta
    pub rolling: c_short,
    /// X coordinate
    pub x: c_int,
    /// Y coordinate
    pub y: c_int,
    /// Additional information
    pub information: c_uint,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of MouseStroke"][size_of::<MouseStroke>() - 20usize];
    ["Alignment of MouseStroke"][align_of::<MouseStroke>() - 4usize];
    ["Offset of field: MouseStroke::state"][mem::offset_of!(MouseStroke, state) - 0usize];
    ["Offset of field: MouseStroke::flags"][mem::offset_of!(MouseStroke, flags) - 2usize];
    ["Offset of field: MouseStroke::rolling"][mem::offset_of!(MouseStroke, rolling) - 4usize];
    ["Offset of field: MouseStroke::x"][mem::offset_of!(MouseStroke, x) - 8usize];
    ["Offset of field: MouseStroke::y"][mem::offset_of!(MouseStroke, y) - 12usize];
    ["Offset of field: MouseStroke::information"]
        [mem::offset_of!(MouseStroke, information) - 16usize];
};

/// Union type for input strokes (keyboard or mouse)
#[derive(Clone, Copy)]
#[repr(C)]
pub union Stroke {
    /// Keyboard stroke data
    pub key: KeyStroke,
    /// Mouse stroke data
    pub mouse: MouseStroke,
}

// Internal Windows API structures matching the C implementation

#[derive(Clone)]
#[repr(C)]
struct KeyboardInputData {
    unit_id: c_ushort,
    make_code: c_ushort,
    flags: c_ushort,
    reserved: c_ushort,
    extra_information: c_uint,
}

#[derive(Clone)]
#[repr(C)]
struct MouseInputData {
    unit_id: c_ushort,
    flags: c_ushort,
    button_flags: c_ushort,
    button_data: c_ushort,
    raw_buttons: c_ulong,
    last_x: c_long,
    last_y: c_long,
    extra_information: c_ulong,
}

struct DeviceContext {
    handle: HANDLE,
    unempty_event: HANDLE,
}

impl DeviceContext {
    fn new(device_index: usize) -> Result<Self, InterceptionError> {
        let device_name = format!("\\\\.\\interception{device_index:02}");
        // Convert to UTF-16 for CreateFileW
        let device_name_w: Vec<u16> = device_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let handle = CreateFileW(
                device_name_w.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_NONE,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            );

            if handle == INVALID_HANDLE_VALUE {
                return Err(InterceptionError::CreateFile(GetLastError()));
            }

            let unempty_event = CreateEventW(
                ptr::null(),
                TRUE,  // Manual reset
                FALSE, // Initially non-signaled
                ptr::null(),
            );

            if unempty_event.is_null() {
                CloseHandle(handle);
                return Err(InterceptionError::CreateEvent(GetLastError()));
            }

            // Set the event handle for the device
            let event_handles = [unempty_event, ptr::null()];
            let mut bytes_returned = 0;

            let result = DeviceIoControl(
                handle,
                IOCTL_SET_EVENT,
                event_handles.as_ptr() as *const _,
                (event_handles.len() * size_of::<HANDLE>()) as u32,
                ptr::null_mut(),
                0,
                &mut bytes_returned,
                ptr::null_mut(),
            );

            if result == 0 {
                let error = GetLastError();
                CloseHandle(handle);
                CloseHandle(unempty_event);
                return Err(InterceptionError::DeviceIoControl(error));
            }

            Ok(DeviceContext {
                handle,
                unempty_event,
            })
        }
    }
}

impl Drop for DeviceContext {
    fn drop(&mut self) {
        unsafe {
            if self.handle != INVALID_HANDLE_VALUE {
                CloseHandle(self.handle);
            }
            if !self.unempty_event.is_null() {
                CloseHandle(self.unempty_event);
            }
        }
    }
}

/// Error types for Interception operations
#[derive(Debug, Clone)]
pub enum InterceptionError {
    /// Failed to create device file
    CreateFile(u32),
    /// Failed to create event
    CreateEvent(u32),
    /// Device I/O control failed
    DeviceIoControl(u32),
    /// Invalid path or string conversion
    InvalidPath,
    /// Invalid device ID
    InvalidDevice,
    /// Context not initialized
    ContextNotInitialized,
    /// Memory allocation failed
    MemoryAllocation,
    /// Wait operation failed or timed out
    WaitFailed(u32),
}

impl std::fmt::Display for InterceptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterceptionError::CreateFile(code) => {
                write!(f, "Failed to create device file, error code: {code}")
            }
            InterceptionError::CreateEvent(code) => {
                write!(f, "Failed to create event, error code: {code}")
            }
            InterceptionError::DeviceIoControl(code) => {
                write!(f, "Device I/O control failed, error code: {code}")
            }
            InterceptionError::InvalidPath => {
                write!(f, "Invalid device path or string conversion error")
            }
            InterceptionError::InvalidDevice => write!(f, "Invalid device ID"),
            InterceptionError::ContextNotInitialized => {
                write!(f, "Interception context is not initialized")
            }
            InterceptionError::MemoryAllocation => write!(f, "Memory allocation failed"),
            InterceptionError::WaitFailed(code) => {
                write!(f, "Wait operation failed or timed out, error code: {code}")
            }
        }
    }
}

impl std::error::Error for InterceptionError {}

/// Main interception context for managing devices and input capture
pub struct Context {
    devices: Vec<Option<DeviceContext>>,
}

impl Context {
    /// Create a new interception context
    ///
    /// This initializes communication with all available interception devices.
    /// Requires the Interception driver to be installed on the system.
    pub fn new() -> Result<Self, InterceptionError> {
        let mut devices = Vec::with_capacity(INTERCEPTION_MAX_DEVICE);

        // Initialize all device contexts
        for i in 0..INTERCEPTION_MAX_DEVICE {
            match DeviceContext::new(i) {
                Ok(device_ctx) => devices.push(Some(device_ctx)),
                Err(_) => {
                    // If we can't create any device, fail initialization
                    if i == 0 {
                        return Err(InterceptionError::CreateFile(0));
                    }
                    // If we fail to create some devices, just mark them as unavailable
                    devices.push(None);
                }
            }
        }

        Ok(Context { devices })
    }

    /// Get the precedence for a specific device
    pub fn get_precedence(&self, device: Device) -> Result<Precedence, InterceptionError> {
        let device_index = self.validate_device(device)?;
        let device_ctx = self.devices[device_index]
            .as_ref()
            .ok_or(InterceptionError::InvalidDevice)?;

        let mut precedence: Precedence = 0;
        let mut bytes_returned = 0;

        unsafe {
            let result = DeviceIoControl(
                device_ctx.handle,
                IOCTL_GET_PRECEDENCE,
                ptr::null(),
                0,
                &mut precedence as *mut _ as *mut _,
                size_of::<Precedence>() as u32,
                &mut bytes_returned,
                ptr::null_mut(),
            );

            if result == 0 {
                return Err(InterceptionError::DeviceIoControl(GetLastError()));
            }
        }

        Ok(precedence)
    }

    /// Set the precedence for a specific device
    pub fn set_precedence(
        &self,
        device: Device,
        precedence: Precedence,
    ) -> Result<(), InterceptionError> {
        let device_index = self.validate_device(device)?;
        let device_ctx = self.devices[device_index]
            .as_ref()
            .ok_or(InterceptionError::InvalidDevice)?;

        let mut bytes_returned = 0;

        unsafe {
            let result = DeviceIoControl(
                device_ctx.handle,
                IOCTL_SET_PRECEDENCE,
                &precedence as *const _ as *const _,
                size_of::<Precedence>() as u32,
                ptr::null_mut(),
                0,
                &mut bytes_returned,
                ptr::null_mut(),
            );

            if result == 0 {
                return Err(InterceptionError::DeviceIoControl(GetLastError()));
            }
        }

        Ok(())
    }

    /// Get the filter for a specific device
    pub fn get_filter(&self, device: Device) -> Result<Filter, InterceptionError> {
        let device_index = self.validate_device(device)?;
        let device_ctx = self.devices[device_index]
            .as_ref()
            .ok_or(InterceptionError::InvalidDevice)?;

        let mut filter: Filter = FILTER_NONE;
        let mut bytes_returned = 0;

        unsafe {
            let result = DeviceIoControl(
                device_ctx.handle,
                IOCTL_GET_FILTER,
                ptr::null(),
                0,
                &mut filter as *mut _ as *mut _,
                size_of::<Filter>() as u32,
                &mut bytes_returned,
                ptr::null_mut(),
            );

            if result == 0 {
                return Err(InterceptionError::DeviceIoControl(GetLastError()));
            }
        }

        Ok(filter)
    }

    /// Set a filter for devices matching the predicate
    pub fn set_filter(
        &self,
        predicate: PredicateFn,
        filter: Filter,
    ) -> Result<(), InterceptionError> {
        for i in 0..INTERCEPTION_MAX_DEVICE {
            let device_id = (i + 1) as Device;
            if predicate(device_id) {
                if let Some(device_ctx) = &self.devices[i] {
                    let mut bytes_returned = 0;

                    unsafe {
                        let result = DeviceIoControl(
                            device_ctx.handle,
                            IOCTL_SET_FILTER,
                            &filter as *const _ as *const _,
                            size_of::<Filter>() as u32,
                            ptr::null_mut(),
                            0,
                            &mut bytes_returned,
                            ptr::null_mut(),
                        );

                        if result == 0 {
                            return Err(InterceptionError::DeviceIoControl(GetLastError()));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_device(&self, device: Device) -> Result<usize, InterceptionError> {
        if device < 1 || device > INTERCEPTION_MAX_DEVICE as Device {
            return Err(InterceptionError::InvalidDevice);
        }
        Ok((device - 1) as usize)
    }

    /// Wait indefinitely for input from any device
    pub fn wait(&self) -> Option<Device> {
        self.wait_with_timeout(INFINITE)
    }

    /// Wait for input from any device with a timeout
    ///
    /// # Arguments
    /// * `timeout_ms` - Timeout in milliseconds, or `INFINITE` for no timeout
    ///
    /// # Returns
    /// * `Some(device)` - The device that has input available
    /// * `None` - Timeout occurred or error
    pub fn wait_with_timeout(&self, timeout_ms: u32) -> Option<Device> {
        // Collect all valid event handles
        let mut wait_handles: Vec<HANDLE> = Vec::new();
        let mut device_mapping: Vec<usize> = Vec::new();

        for (i, device_ctx) in self.devices.iter().enumerate() {
            if let Some(ctx) = device_ctx {
                wait_handles.push(ctx.unempty_event);
                device_mapping.push(i);
            }
        }

        if wait_handles.is_empty() {
            return None;
        }

        unsafe {
            let result = WaitForMultipleObjects(
                wait_handles.len() as u32,
                wait_handles.as_ptr(),
                FALSE, // Wait for any
                timeout_ms,
            );

            match result {
                WAIT_FAILED | WAIT_TIMEOUT => None,
                index => {
                    let wait_index = (index - WAIT_OBJECT_0) as usize;
                    if wait_index < device_mapping.len() {
                        Some((device_mapping[wait_index] + 1) as Device)
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Send strokes to a device
    pub fn send(&self, device: Device, strokes: &[Stroke]) -> Result<usize, InterceptionError> {
        if strokes.is_empty() {
            return Ok(0);
        }

        let device_index = self.validate_device(device)?;
        let device_ctx = self.devices[device_index]
            .as_ref()
            .ok_or(InterceptionError::InvalidDevice)?;

        let strokes_written = if is_keyboard_device(device) {
            self.send_keyboard_strokes(device_ctx, strokes)?
        } else if is_mouse_device(device) {
            self.send_mouse_strokes(device_ctx, strokes)?
        } else {
            return Err(InterceptionError::InvalidDevice);
        };

        Ok(strokes_written)
    }

    fn send_keyboard_strokes(
        &self,
        device_ctx: &DeviceContext,
        strokes: &[Stroke],
    ) -> Result<usize, InterceptionError> {
        // Allocate memory using Rust's Vec for safety
        let mut raw_strokes: Vec<KeyboardInputData> = Vec::with_capacity(strokes.len());

        // Convert Stroke to KeyboardInputData
        for stroke in strokes.iter() {
            unsafe {
                let key_stroke = stroke.key;
                raw_strokes.push(KeyboardInputData {
                    unit_id: 0,
                    make_code: key_stroke.code,
                    flags: key_stroke.state,
                    reserved: 0,
                    extra_information: key_stroke.information,
                });
            }
        }

        let mut strokes_written = 0;
        unsafe {
            let result = DeviceIoControl(
                device_ctx.handle,
                IOCTL_WRITE,
                raw_strokes.as_ptr() as *const _,
                (raw_strokes.len() * size_of::<KeyboardInputData>()) as u32,
                ptr::null_mut(),
                0,
                &mut strokes_written,
                ptr::null_mut(),
            );

            if result == 0 {
                return Err(InterceptionError::DeviceIoControl(GetLastError()));
            }
        }

        Ok((strokes_written as usize) / size_of::<KeyboardInputData>())
        // raw_strokes is automatically freed when it goes out of scope
    }

    fn send_mouse_strokes(
        &self,
        device_ctx: &DeviceContext,
        strokes: &[Stroke],
    ) -> Result<usize, InterceptionError> {
        // Allocate memory using Rust's Vec for safety
        let mut raw_strokes: Vec<MouseInputData> = Vec::with_capacity(strokes.len());

        // Convert Stroke to MouseInputData
        for stroke in strokes.iter() {
            unsafe {
                let mouse_stroke = stroke.mouse;
                raw_strokes.push(MouseInputData {
                    unit_id: 0,
                    flags: mouse_stroke.flags,
                    button_flags: mouse_stroke.state,
                    button_data: mouse_stroke.rolling as u16,
                    raw_buttons: 0,
                    last_x: mouse_stroke.x,
                    last_y: mouse_stroke.y,
                    extra_information: mouse_stroke.information,
                });
            }
        }

        let mut strokes_written = 0;
        unsafe {
            let result = DeviceIoControl(
                device_ctx.handle,
                IOCTL_WRITE,
                raw_strokes.as_ptr() as *const _,
                (raw_strokes.len() * size_of::<MouseInputData>()) as u32,
                ptr::null_mut(),
                0,
                &mut strokes_written,
                ptr::null_mut(),
            );

            if result == 0 {
                return Err(InterceptionError::DeviceIoControl(GetLastError()));
            }
        }

        Ok((strokes_written as usize) / size_of::<MouseInputData>())
        // raw_strokes is automatically freed when it goes out of scope
    }

    /// Receive strokes from a device
    pub fn receive(
        &self,
        device: Device,
        max_strokes: usize,
    ) -> Result<Vec<Stroke>, InterceptionError> {
        if max_strokes == 0 {
            return Ok(Vec::new());
        }

        let device_index = self.validate_device(device)?;
        let device_ctx = self.devices[device_index]
            .as_ref()
            .ok_or(InterceptionError::InvalidDevice)?;

        if is_keyboard_device(device) {
            self.receive_keyboard_strokes(device_ctx, max_strokes)
        } else if is_mouse_device(device) {
            self.receive_mouse_strokes(device_ctx, max_strokes)
        } else {
            Err(InterceptionError::InvalidDevice)
        }
    }

    fn receive_keyboard_strokes(
        &self,
        device_ctx: &DeviceContext,
        max_strokes: usize,
    ) -> Result<Vec<Stroke>, InterceptionError> {
        // Allocate memory using Rust's Vec for safety
        let mut raw_strokes: Vec<KeyboardInputData> = vec![
            KeyboardInputData {
                unit_id: 0,
                make_code: 0,
                flags: 0,
                reserved: 0,
                extra_information: 0,
            }; 
            max_strokes
        ];

        let mut strokes_read = 0;
        unsafe {
            let result = DeviceIoControl(
                device_ctx.handle,
                IOCTL_READ,
                ptr::null(),
                0,
                raw_strokes.as_mut_ptr() as *mut _,
                (max_strokes * size_of::<KeyboardInputData>()) as u32,
                &mut strokes_read,
                ptr::null_mut(),
            );

            if result == 0 {
                return Err(InterceptionError::DeviceIoControl(GetLastError()));
            }
        }

        let strokes_count = (strokes_read as usize) / size_of::<KeyboardInputData>();
        raw_strokes.truncate(strokes_count);

        let mut strokes = Vec::with_capacity(strokes_count);
        for raw_stroke in &raw_strokes {
            strokes.push(Stroke {
                key: KeyStroke {
                    code: raw_stroke.make_code,
                    state: raw_stroke.flags,
                    information: raw_stroke.extra_information,
                },
            });
        }

        Ok(strokes)
        // raw_strokes is automatically freed when it goes out of scope
    }

    fn receive_mouse_strokes(
        &self,
        device_ctx: &DeviceContext,
        max_strokes: usize,
    ) -> Result<Vec<Stroke>, InterceptionError> {
        // Allocate memory using Rust's Vec for safety
        let mut raw_strokes: Vec<MouseInputData> = vec![
            MouseInputData {
                unit_id: 0,
                flags: 0,
                button_flags: 0,
                button_data: 0,
                raw_buttons: 0,
                last_x: 0,
                last_y: 0,
                extra_information: 0,
            };
            max_strokes
        ];

        let mut strokes_read = 0;
        unsafe {
            let result = DeviceIoControl(
                device_ctx.handle,
                IOCTL_READ,
                ptr::null(),
                0,
                raw_strokes.as_mut_ptr() as *mut _,
                (max_strokes * size_of::<MouseInputData>()) as u32,
                &mut strokes_read,
                ptr::null_mut(),
            );

            if result == 0 {
                return Err(InterceptionError::DeviceIoControl(GetLastError()));
            }
        }

        let strokes_count = (strokes_read as usize) / size_of::<MouseInputData>();
        raw_strokes.truncate(strokes_count);

        let mut strokes = Vec::with_capacity(strokes_count);
        for raw_stroke in &raw_strokes {
            strokes.push(Stroke {
                mouse: MouseStroke {
                    state: raw_stroke.button_flags,
                    flags: raw_stroke.flags,
                    rolling: raw_stroke.button_data as i16,
                    x: raw_stroke.last_x,
                    y: raw_stroke.last_y,
                    information: raw_stroke.extra_information,
                },
            });
        }

        Ok(strokes)
        // raw_strokes is automatically freed when it goes out of scope
    }

    /// Get hardware ID for a device
    pub fn get_hardware_id(&self, device: Device) -> Result<Vec<u8>, InterceptionError> {
        let device_index = self.validate_device(device)?;
        let device_ctx = self.devices[device_index]
            .as_ref()
            .ok_or(InterceptionError::InvalidDevice)?;

        // Try with a reasonable buffer size first
        let mut buffer = vec![0u8; 512];
        let mut output_size = 0;

        unsafe {
            let result = DeviceIoControl(
                device_ctx.handle,
                IOCTL_GET_HARDWARE_ID,
                ptr::null(),
                0,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                &mut output_size,
                ptr::null_mut(),
            );

            if result == 0 {
                return Err(InterceptionError::DeviceIoControl(GetLastError()));
            }

            buffer.truncate(output_size as usize);
            Ok(buffer)
        }
    }
}

// Device utility functions - define as standalone functions since Device is a type alias
/// Check if device ID is invalid
pub fn is_invalid_device(device: Device) -> bool {
    !is_keyboard_device(device) && !is_mouse_device(device)
}

/// Check if device is a keyboard
pub fn is_keyboard_device(device: Device) -> bool {
    device >= keyboard(0) && device <= keyboard(INTERCEPTION_MAX_KEYBOARD - 1)
}

/// Check if device is a mouse
pub fn is_mouse_device(device: Device) -> bool {
    device >= mouse(0) && device <= mouse(INTERCEPTION_MAX_MOUSE - 1)
}

// Convenience constructors for strokes
impl KeyStroke {
    /// Create a new keyboard stroke
    pub fn new(code: u16, state: u16) -> Self {
        Self {
            code,
            state,
            information: 0,
        }
    }

    /// Create a key down stroke
    pub fn down(code: u16) -> Self {
        Self::new(code, KEY_DOWN as u16)
    }

    /// Create a key up stroke
    pub fn up(code: u16) -> Self {
        Self::new(code, KEY_UP as u16)
    }
}

impl MouseStroke {
    /// Create a new mouse stroke
    pub fn new() -> Self {
        Self {
            state: 0,
            flags: 0,
            rolling: 0,
            x: 0,
            y: 0,
            information: 0,
        }
    }

    /// Create a mouse move stroke
    pub fn move_to(x: i32, y: i32) -> Self {
        Self {
            state: 0,
            flags: MOUSE_MOVE_ABSOLUTE as u16,
            rolling: 0,
            x,
            y,
            information: 0,
        }
    }

    /// Create a mouse button down stroke
    pub fn button_down(button: c_int) -> Self {
        Self {
            state: button as u16,
            flags: 0,
            rolling: 0,
            x: 0,
            y: 0,
            information: 0,
        }
    }

    /// Create a mouse button up stroke
    pub fn button_up(button: c_int) -> Self {
        Self {
            state: button as u16,
            flags: 0,
            rolling: 0,
            x: 0,
            y: 0,
            information: 0,
        }
    }

    /// Create a mouse wheel stroke
    pub fn wheel(delta: i16) -> Self {
        Self {
            state: MOUSE_WHEEL as u16,
            flags: 0,
            rolling: delta,
            x: 0,
            y: 0,
            information: 0,
        }
    }
}

impl Default for MouseStroke {
    fn default() -> Self {
        Self::new()
    }
}

impl From<KeyStroke> for Stroke {
    fn from(key: KeyStroke) -> Self {
        Stroke { key }
    }
}

impl From<MouseStroke> for Stroke {
    fn from(mouse: MouseStroke) -> Self {
        Stroke { mouse }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_functions() {
        // Test keyboard devices
        for i in 0..INTERCEPTION_MAX_KEYBOARD {
            let dev = keyboard(i);
            assert!(is_keyboard_device(dev));
            assert!(!is_mouse_device(dev));
            assert!(!is_invalid_device(dev));
        }

        // Test mouse devices
        for i in 0..INTERCEPTION_MAX_MOUSE {
            let dev = mouse(i);
            assert!(is_mouse_device(dev));
            assert!(!is_keyboard_device(dev));
            assert!(!is_invalid_device(dev));
        }

        // Test invalid devices
        assert!(is_invalid_device(0));
        assert!(is_invalid_device(-1));
        assert!(is_invalid_device(INTERCEPTION_MAX_DEVICE as Device + 1));
    }

    #[test]
    fn test_stroke_creation() {
        let key_stroke = KeyStroke::down(0x41); // 'A' key
        assert_eq!(key_stroke.code, 0x41);
        assert_eq!(key_stroke.state, KEY_DOWN as u16);

        let mouse_stroke = MouseStroke::move_to(100, 200);
        assert_eq!(mouse_stroke.x, 100);
        assert_eq!(mouse_stroke.y, 200);
        assert_eq!(mouse_stroke.flags, MOUSE_MOVE_ABSOLUTE as u16);

        let wheel_stroke = MouseStroke::wheel(120);
        assert_eq!(wheel_stroke.rolling, 120);
        assert_eq!(wheel_stroke.state, MOUSE_WHEEL as u16);
    }

    #[test]
    fn test_flag_combinations() {
        // Test combining KeyState flags with bitwise operations
        let combined_key_state = KEY_UP | KEY_E0;
        assert_eq!(combined_key_state, 0x01 | 0x02);
        assert!(combined_key_state & KEY_UP != 0);
        assert!(combined_key_state & KEY_E0 != 0);
        // KEY_DOWN is 0, so we test that we didn't accidentally include E1 instead
        assert!(combined_key_state & KEY_E1 == 0);

        // Test combining MouseState flags with bitwise operations
        let combined_mouse_state = MOUSE_LEFT_BUTTON_DOWN | MOUSE_WHEEL;
        assert_eq!(combined_mouse_state, 0x001 | 0x400);
        assert!(combined_mouse_state & MOUSE_LEFT_BUTTON_DOWN != 0);
        assert!(combined_mouse_state & MOUSE_WHEEL != 0);
        assert!(combined_mouse_state & MOUSE_RIGHT_BUTTON_DOWN == 0);

        // Test combining Filter flags with bitwise operations
        let combined_filter = FILTER_KEY_UP | FILTER_MOUSE_WHEEL;
        assert_eq!(combined_filter, 0x02 | 0x400);
        assert!(combined_filter & FILTER_KEY_UP != 0);
        assert!(combined_filter & FILTER_MOUSE_WHEEL != 0);
        assert!(combined_filter & FILTER_KEY_DOWN == 0);
    }
}
