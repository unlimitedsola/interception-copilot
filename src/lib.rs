//! # Interception Copilot
//!
//! Rust port of the [Interception library](https://github.com/oblitum/Interception)
//! using `windows-sys` with a safe API for intercepting keyboard and mouse input on Windows.
//!
//! The Interception library allows you to intercept and modify keyboard and mouse input
//! at a low level on Windows systems. This Rust port provides safe wrappers for convenient use.
//!
//! ## Type-Safe Device API
//!
//! The library provides type-safe device structures that prevent misuse:
//!
//! ```rust,no_run
//! use interception_copilot::{KeyboardDevice, MouseDevice, FILTER_KEY_ALL, FILTER_MOUSE_ALL};
//!
//! // Create type-safe keyboard device  
//! let keyboard = KeyboardDevice::new(0).expect("Failed to create keyboard device");
//! keyboard.set_filter(FILTER_KEY_ALL).expect("Failed to set keyboard filter");
//!
//! // Create type-safe mouse device
//! let mouse = MouseDevice::new(0).expect("Failed to create mouse device");
//! mouse.set_filter(FILTER_MOUSE_ALL).expect("Failed to set mouse filter");
//!
//! // Type safety: you can only send keyboard strokes to keyboard devices
//! let key_strokes = vec![/* KeyStroke data */];
//! keyboard.send(&key_strokes).expect("Failed to send keyboard strokes");
//!
//! // Type safety: you can only send mouse strokes to mouse devices  
//! let mouse_strokes = vec![/* MouseStroke data */];
//! mouse.send(&mouse_strokes).expect("Failed to send mouse strokes");
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

/// Precedence value for device handling order
pub type Precedence = c_int;

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
pub struct KeyboardInputData {
    pub unit_id: c_ushort,
    pub make_code: c_ushort,
    pub flags: c_ushort,
    pub reserved: c_ushort,
    pub extra_information: c_uint,
}

impl Default for KeyboardInputData {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct MouseInputData {
    pub unit_id: c_ushort,
    pub flags: c_ushort,
    pub button_flags: c_ushort,
    pub button_data: c_ushort,
    pub raw_buttons: c_ulong,
    pub last_x: c_long,
    pub last_y: c_long,
    pub extra_information: c_ulong,
}

impl Default for MouseInputData {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

struct DeviceContext {
    handle: HANDLE,
    unempty_event: HANDLE,
    index: usize,
}

impl DeviceContext {
    fn new(device_index: usize, logical_index: usize) -> Result<Self, InterceptionError> {
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
                index: logical_index,
            })
        }
    }

    /// Set filter for this device
    fn set_filter(&self, filter: Filter) -> Result<(), InterceptionError> {
        let mut bytes_returned = 0;

        unsafe {
            let result = DeviceIoControl(
                self.handle,
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

        Ok(())
    }

    /// Get filter for this device
    fn get_filter(&self) -> Result<Filter, InterceptionError> {
        let mut filter: Filter = FILTER_NONE;
        let mut bytes_returned = 0;

        unsafe {
            let result = DeviceIoControl(
                self.handle,
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

    /// Set precedence for this device
    fn set_precedence(&self, precedence: Precedence) -> Result<(), InterceptionError> {
        let mut bytes_returned = 0;

        unsafe {
            let result = DeviceIoControl(
                self.handle,
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

    /// Get precedence for this device
    fn get_precedence(&self) -> Result<Precedence, InterceptionError> {
        let mut precedence: Precedence = 0;
        let mut bytes_returned = 0;

        unsafe {
            let result = DeviceIoControl(
                self.handle,
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

    /// Get hardware ID for this device
    fn get_hardware_id(&self) -> Result<Vec<u8>, InterceptionError> {
        // Try with a reasonable buffer size first
        let mut buffer = vec![0u8; 512];
        let mut output_size = 0;

        unsafe {
            let result = DeviceIoControl(
                self.handle,
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

    /// Get the device index (0-9 for keyboards/mice)
    fn index(&self) -> usize {
        self.index
    }

    /// Check if this device has input available (non-blocking)
    fn has_input(&self) -> bool {
        unsafe {
            matches!(
                WaitForMultipleObjects(1, &self.unempty_event, FALSE, 0),
                WAIT_OBJECT_0
            )
        }
    }

    /// Wait for input on this device
    fn wait(&self) -> Result<(), InterceptionError> {
        unsafe {
            match WaitForMultipleObjects(1, &self.unempty_event, FALSE, INFINITE) {
                WAIT_OBJECT_0 => Ok(()),
                WAIT_FAILED => Err(InterceptionError::WaitFailed(GetLastError())),
                _ => Err(InterceptionError::WaitFailed(0)),
            }
        }
    }

    /// Wait for input on this device with timeout
    fn wait_with_timeout(&self, timeout_ms: u32) -> Result<bool, InterceptionError> {
        unsafe {
            match WaitForMultipleObjects(1, &self.unempty_event, FALSE, timeout_ms) {
                WAIT_OBJECT_0 => Ok(true),
                WAIT_TIMEOUT => Ok(false),
                WAIT_FAILED => Err(InterceptionError::WaitFailed(GetLastError())),
                _ => Err(InterceptionError::WaitFailed(0)),
            }
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

/// A keyboard input device for intercepting and injecting keyboard events
pub struct KeyboardDevice {
    device_ctx: DeviceContext,
}

/// A mouse input device for intercepting and injecting mouse events  
pub struct MouseDevice {
    device_ctx: DeviceContext,
}

impl KeyboardDevice {
    /// Create a new keyboard device
    ///
    /// # Arguments
    /// * `index` - Keyboard index (0-9)
    ///
    /// # Errors
    /// Returns an error if the device cannot be created or if index is out of range
    pub fn new(index: usize) -> Result<Self, InterceptionError> {
        if index >= INTERCEPTION_MAX_KEYBOARD {
            return Err(InterceptionError::InvalidDevice);
        }

        let device_ctx = DeviceContext::new(index, index)?;
        Ok(KeyboardDevice { device_ctx })
    }

    /// Set filter for this keyboard device
    pub fn set_filter(&self, filter: Filter) -> Result<(), InterceptionError> {
        self.device_ctx.set_filter(filter)
    }

    /// Get filter for this keyboard device
    pub fn get_filter(&self) -> Result<Filter, InterceptionError> {
        self.device_ctx.get_filter()
    }

    /// Set precedence for this keyboard device
    pub fn set_precedence(&self, precedence: Precedence) -> Result<(), InterceptionError> {
        self.device_ctx.set_precedence(precedence)
    }

    /// Get precedence for this keyboard device
    pub fn get_precedence(&self) -> Result<Precedence, InterceptionError> {
        self.device_ctx.get_precedence()
    }

    /// Send keyboard strokes to this device
    pub fn send(&self, strokes: &[KeyStroke]) -> Result<usize, InterceptionError> {
        if strokes.is_empty() {
            return Ok(0);
        }

        // Convert KeyStroke to Stroke for internal compatibility
        let stroke_unions: Vec<Stroke> = strokes.iter().map(|&ks| Stroke { key: ks }).collect();
        let strokes_written = self.send_keyboard_strokes(&stroke_unions)?;
        Ok(strokes_written)
    }

    /// Receive keyboard strokes from this device
    pub fn receive(&self, max_strokes: usize) -> Result<Vec<KeyStroke>, InterceptionError> {
        if max_strokes == 0 {
            return Ok(Vec::new());
        }

        let raw_strokes = self.receive_keyboard_strokes(max_strokes)?;
        let mut strokes = Vec::with_capacity(raw_strokes.len());
        for raw_stroke in &raw_strokes {
            strokes.push(KeyStroke {
                code: raw_stroke.make_code,
                state: raw_stroke.flags,
                information: raw_stroke.extra_information,
            });
        }
        Ok(strokes)
    }

    /// Get hardware ID for this keyboard device
    pub fn get_hardware_id(&self) -> Result<Vec<u8>, InterceptionError> {
        self.device_ctx.get_hardware_id()
    }

    /// Get the keyboard index (0-9)
    pub fn index(&self) -> usize {
        self.device_ctx.index()
    }

    /// Check if this device has input available (non-blocking)
    pub fn has_input(&self) -> bool {
        self.device_ctx.has_input()
    }

    /// Wait for input on this keyboard device
    pub fn wait(&self) -> Result<(), InterceptionError> {
        self.device_ctx.wait()
    }

    /// Wait for input on this keyboard device with timeout
    pub fn wait_with_timeout(&self, timeout_ms: u32) -> Result<bool, InterceptionError> {
        self.device_ctx.wait_with_timeout(timeout_ms)
    }

    fn send_keyboard_strokes(&self, strokes: &[Stroke]) -> Result<usize, InterceptionError> {
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
                self.device_ctx.handle,
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
    }

    fn receive_keyboard_strokes(
        &self,
        max_strokes: usize,
    ) -> Result<Vec<KeyboardInputData>, InterceptionError> {
        // Allocate memory using Rust's Vec for safety
        let mut raw_strokes: Vec<KeyboardInputData> =
            vec![KeyboardInputData::default(); max_strokes];

        let mut strokes_read = 0;
        unsafe {
            let result = DeviceIoControl(
                self.device_ctx.handle,
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

        Ok(raw_strokes)
    }
}

impl MouseDevice {
    /// Create a new mouse device
    ///
    /// # Arguments
    /// * `index` - Mouse index (0-9)
    ///
    /// # Errors
    /// Returns an error if the device cannot be created or if index is out of range
    pub fn new(index: usize) -> Result<Self, InterceptionError> {
        if index >= INTERCEPTION_MAX_MOUSE {
            return Err(InterceptionError::InvalidDevice);
        }

        let device_index = INTERCEPTION_MAX_KEYBOARD + index;
        let device_ctx = DeviceContext::new(device_index, index)?;
        Ok(MouseDevice { device_ctx })
    }

    /// Set filter for this mouse device
    pub fn set_filter(&self, filter: Filter) -> Result<(), InterceptionError> {
        self.device_ctx.set_filter(filter)
    }

    /// Get filter for this mouse device
    pub fn get_filter(&self) -> Result<Filter, InterceptionError> {
        self.device_ctx.get_filter()
    }

    /// Set precedence for this mouse device
    pub fn set_precedence(&self, precedence: Precedence) -> Result<(), InterceptionError> {
        self.device_ctx.set_precedence(precedence)
    }

    /// Get precedence for this mouse device
    pub fn get_precedence(&self) -> Result<Precedence, InterceptionError> {
        self.device_ctx.get_precedence()
    }

    /// Send mouse strokes to this device
    pub fn send(&self, strokes: &[MouseStroke]) -> Result<usize, InterceptionError> {
        if strokes.is_empty() {
            return Ok(0);
        }

        // Convert MouseStroke to Stroke for internal compatibility
        let stroke_unions: Vec<Stroke> = strokes.iter().map(|&ms| Stroke { mouse: ms }).collect();
        let strokes_written = self.send_mouse_strokes(&stroke_unions)?;
        Ok(strokes_written)
    }

    /// Receive mouse strokes from this device
    pub fn receive(&self, max_strokes: usize) -> Result<Vec<MouseStroke>, InterceptionError> {
        if max_strokes == 0 {
            return Ok(Vec::new());
        }

        let raw_strokes = self.receive_mouse_strokes(max_strokes)?;
        let mut strokes = Vec::with_capacity(raw_strokes.len());
        for raw_stroke in &raw_strokes {
            strokes.push(MouseStroke {
                state: raw_stroke.button_flags,
                flags: raw_stroke.flags,
                rolling: raw_stroke.button_data as i16,
                x: raw_stroke.last_x,
                y: raw_stroke.last_y,
                information: raw_stroke.extra_information,
            });
        }
        Ok(strokes)
    }

    /// Get hardware ID for this mouse device
    pub fn get_hardware_id(&self) -> Result<Vec<u8>, InterceptionError> {
        self.device_ctx.get_hardware_id()
    }

    /// Get the mouse index (0-9)
    pub fn index(&self) -> usize {
        self.device_ctx.index()
    }

    /// Check if this device has input available (non-blocking)
    pub fn has_input(&self) -> bool {
        self.device_ctx.has_input()
    }

    /// Wait for input on this mouse device
    pub fn wait(&self) -> Result<(), InterceptionError> {
        self.device_ctx.wait()
    }

    /// Wait for input on this mouse device with timeout
    pub fn wait_with_timeout(&self, timeout_ms: u32) -> Result<bool, InterceptionError> {
        self.device_ctx.wait_with_timeout(timeout_ms)
    }

    fn send_mouse_strokes(&self, strokes: &[Stroke]) -> Result<usize, InterceptionError> {
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
                self.device_ctx.handle,
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
    }

    fn receive_mouse_strokes(
        &self,
        max_strokes: usize,
    ) -> Result<Vec<MouseInputData>, InterceptionError> {
        // Allocate memory using Rust's Vec for safety
        let mut raw_strokes: Vec<MouseInputData> = vec![MouseInputData::default(); max_strokes];

        let mut strokes_read = 0;
        unsafe {
            let result = DeviceIoControl(
                self.device_ctx.handle,
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

        Ok(raw_strokes)
    }
}

/// Enum representing either a keyboard or mouse device for waiting operations
pub enum AnyDevice {
    Keyboard(KeyboardDevice),
    Mouse(MouseDevice),
}

impl AnyDevice {
    /// Get the underlying event handle for waiting operations
    fn event_handle(&self) -> HANDLE {
        match self {
            AnyDevice::Keyboard(kb) => kb.device_ctx.unempty_event,
            AnyDevice::Mouse(mouse) => mouse.device_ctx.unempty_event,
        }
    }

    /// Check if this device has input available (non-blocking)
    pub fn has_input(&self) -> bool {
        match self {
            AnyDevice::Keyboard(kb) => kb.has_input(),
            AnyDevice::Mouse(mouse) => mouse.has_input(),
        }
    }
}

/// Wait for input from any of the provided devices
///
/// # Arguments
/// * `devices` - Slice of devices to wait for
/// * `timeout_ms` - Timeout in milliseconds, or `INFINITE` for no timeout
///
/// # Returns
/// * `Some(index)` - Index of the device that has input available
/// * `None` - Timeout occurred or error
pub fn wait_for_devices(devices: &[AnyDevice], timeout_ms: u32) -> Option<usize> {
    if devices.is_empty() {
        return None;
    }

    let wait_handles: Vec<HANDLE> = devices.iter().map(|d| d.event_handle()).collect();

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
                if wait_index < devices.len() {
                    Some(wait_index)
                } else {
                    None
                }
            }
        }
    }
}

/// Wait indefinitely for input from any of the provided devices
pub fn wait_for_any(devices: &[AnyDevice]) -> Option<usize> {
    wait_for_devices(devices, INFINITE)
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

    #[test]
    fn test_typed_device_bounds_checking() {
        // Test keyboard device creation bounds
        for i in 0..INTERCEPTION_MAX_KEYBOARD {
            // We can't actually create devices without the driver, but we can test the bounds checking
            match KeyboardDevice::new(i) {
                Ok(_) => {}                                 // Device creation succeeded
                Err(InterceptionError::CreateFile(_)) => {} // Expected without driver
                Err(e) => panic!("Unexpected error creating keyboard {i}: {e}"),
            }
        }

        // Test out-of-bounds keyboard device
        assert!(matches!(
            KeyboardDevice::new(INTERCEPTION_MAX_KEYBOARD),
            Err(InterceptionError::InvalidDevice)
        ));

        // Test mouse device creation bounds
        for i in 0..INTERCEPTION_MAX_MOUSE {
            match MouseDevice::new(i) {
                Ok(_) => {}                                 // Device creation succeeded
                Err(InterceptionError::CreateFile(_)) => {} // Expected without driver
                Err(e) => panic!("Unexpected error creating mouse {i}: {e}"),
            }
        }

        // Test out-of-bounds mouse device
        assert!(matches!(
            MouseDevice::new(INTERCEPTION_MAX_MOUSE),
            Err(InterceptionError::InvalidDevice)
        ));
    }
}
