//! # Interception
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
//! use interception::{KeyboardDevice, MouseDevice, KeyStroke, MouseStroke, FILTER_KEY_ALL, FILTER_MOUSE_ALL};
//!
//! // Create type-safe keyboard device
//! let mut keyboard = KeyboardDevice::new(0).expect("Failed to create keyboard device");
//! keyboard.set_filter(FILTER_KEY_ALL).expect("Failed to set keyboard filter");
//!
//! // Create type-safe mouse device
//! let mut mouse = MouseDevice::new(0).expect("Failed to create mouse device");
//! mouse.set_filter(FILTER_MOUSE_ALL).expect("Failed to set mouse filter");
//!
//! // Create strokes using safe constructors
//! let key_strokes = vec![
//!     KeyStroke::down(0x41),  // 'A' key down
//!     KeyStroke::up(0x41),    // 'A' key up
//! ];
//! keyboard.send(&key_strokes).expect("Failed to send keyboard strokes");
//!
//! // Create mouse strokes using the new constructor
//! use interception::{MOUSE_MOVE_ABSOLUTE, MOUSE_LEFT_BUTTON_DOWN, MOUSE_MOVE_RELATIVE};
//! let mouse_strokes = vec![
//!     MouseStroke::new(MOUSE_MOVE_ABSOLUTE, 0, 0, 100, 200, 0),  // Move to (100, 200)
//!     MouseStroke::new(MOUSE_MOVE_RELATIVE, MOUSE_LEFT_BUTTON_DOWN, 0, 0, 0, 0),  // Left button down
//! ];
//! mouse.send(&mouse_strokes).expect("Failed to send mouse strokes");
//! ```
//!
//! ## Consolidated Structs
//!
//! The library uses consolidated structs that directly match the Windows API C-ABI:
//! - `KeyStroke`: Combines public API and internal Windows structure for zero-copy operations
//! - `MouseStroke`: Combines public API and internal Windows structure for zero-copy operations
//!
//! These structs have private fields to maintain API safety, but provide public constructors
//! and accessor methods for common use cases.

#![cfg(windows)]

use std::error::Error;
use std::ffi::{c_int, c_long, c_short, c_uint, c_ulong, c_ushort, c_void};
use std::fmt::{Display, Formatter};
use std::mem;
use std::ptr;
use std::time::Duration;
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, FALSE, GENERIC_READ, GetLastError, HANDLE, INVALID_HANDLE_VALUE, TRUE,
        WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    Storage::FileSystem::{CreateFileW, FILE_SHARE_NONE, OPEN_EXISTING},
    System::{
        IO::DeviceIoControl,
        Ioctl::{FILE_ANY_ACCESS, FILE_DEVICE_UNKNOWN, METHOD_BUFFERED},
        Threading::{CreateEventW, INFINITE, WaitForMultipleObjects},
    },
};

pub struct Interception {
    devices: [Device; MAX_DEVICES],
    wait_handles: [WaitHandle; MAX_DEVICES],
}

impl Interception {
    pub fn new() -> Result<Self> {
        let mut devices = Vec::new();
        let mut wait_handles = Vec::new();

        for i in 0..MAX_DEVICES {
            let mut device = Device::new(i)?;
            let wait_handle = WaitHandle::new()?;
            unsafe {
                // SAFETY: We ensure wait handles will be dropped at the same time when the devices
                // are dropped, therefore it will always be valid during device's lifetime.
                device.set_wait_handle(&wait_handle)?;
            }
            devices.push(device);
            wait_handles.push(wait_handle);
        }
        let devices = devices
            .try_into()
            .expect("device array should have exactly MAX_DEVICES elements");
        let wait_handles = wait_handles
            .try_into()
            .expect("wait handle array should have exactly MAX_DEVICES elements");

        Ok(Interception {
            devices,
            wait_handles,
        })
    }

    pub fn devices(&self) -> &[Device; MAX_DEVICES] {
        &self.devices
    }

    pub fn devices_mut(&mut self) -> &mut [Device; MAX_DEVICES] {
        &mut self.devices
    }

    pub fn set_precedence(&mut self, precedence: Precedence) -> Result<()> {
        for device in &mut self.devices {
            device.set_precedence(precedence)?;
        }
        Ok(())
    }

    pub fn wait(&mut self, timeout: Option<Duration>) -> Result<&mut Device> {
        let index = wait(&self.wait_handles, timeout)?;
        Ok(&mut self.devices[index])
    }
}

// Constants from the original C header
const MAX_KEYBOARD: usize = 10;
const MAX_MOUSE: usize = 10;

const MAX_DEVICES: usize = MAX_KEYBOARD + MAX_MOUSE;

/// Precedence value for device handling order
pub type Precedence = c_int;

/// Keyboard key state flags
pub type KeyState = c_ushort;
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
pub type MouseState = c_ushort;
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
pub type MouseFlag = c_ushort;
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

/// Keyboard filters
pub type KeyFilter = Filter;
/// No keyboard filtering
pub const FILTER_KEY_NONE: KeyFilter = FILTER_NONE;
/// Filter all keyboard events
pub const FILTER_KEY_ALL: KeyFilter = FILTER_ALL;
/// Filter key down events
pub const FILTER_KEY_DOWN: KeyFilter = 0x01;
/// Filter key up events
pub const FILTER_KEY_UP: KeyFilter = 0x02;
/// Filter E0 extended keys
pub const FILTER_KEY_E0: KeyFilter = 0x08;
/// Filter E1 extended keys
pub const FILTER_KEY_E1: KeyFilter = 0x016;

/// Mouse button filters
pub type MouseFilter = Filter;
/// No mouse filtering
pub const FILTER_MOUSE_NONE: MouseFilter = FILTER_NONE;
/// Filter all mouse events
pub const FILTER_MOUSE_ALL: MouseFilter = FILTER_ALL;
/// Filter left mouse button down
pub const FILTER_MOUSE_LEFT_BUTTON_DOWN: MouseFilter = 0x001;
/// Filter left mouse button up
pub const FILTER_MOUSE_LEFT_BUTTON_UP: MouseFilter = 0x002;
/// Filter right mouse button down
pub const FILTER_MOUSE_RIGHT_BUTTON_DOWN: MouseFilter = 0x004;
/// Filter right mouse button up
pub const FILTER_MOUSE_RIGHT_BUTTON_UP: MouseFilter = 0x008;
/// Filter middle mouse button down
pub const FILTER_MOUSE_MIDDLE_BUTTON_DOWN: MouseFilter = 0x010;
/// Filter middle mouse button up
pub const FILTER_MOUSE_MIDDLE_BUTTON_UP: MouseFilter = 0x020;
/// Filter mouse button 4 down
pub const FILTER_MOUSE_BUTTON_4_DOWN: MouseFilter = 0x040;
/// Filter mouse button 4 up
pub const FILTER_MOUSE_BUTTON_4_UP: MouseFilter = 0x080;
/// Filter mouse button 5 down
pub const FILTER_MOUSE_BUTTON_5_DOWN: MouseFilter = 0x100;
/// Filter mouse button 5 up
pub const FILTER_MOUSE_BUTTON_5_UP: MouseFilter = 0x200;
/// Filter mouse wheel
pub const FILTER_MOUSE_WHEEL: MouseFilter = 0x400;
/// Filter mouse horizontal wheel
pub const FILTER_MOUSE_HWHEEL: MouseFilter = 0x800;
/// Filter mouse movement
pub const FILTER_MOUSE_MOVE: MouseFilter = 0x1000;

/// `KEYBOARD_INPUT_DATA` structure
/// <https://learn.microsoft.com/en-us/windows/win32/api/ntddkbd/ns-ntddkbd-keyboard_input_data>
#[derive(Debug, Clone)]
#[repr(C)]
pub struct KeyStroke {
    /// Device unit ID (internal use only)
    _unit_id: c_ushort,
    /// Virtual key code (make_code in Windows API)
    pub code: c_ushort,
    /// Key state flags
    pub state: KeyState,
    /// Reserved field (unused)
    _reserved: c_ushort,
    /// Additional information
    pub information: c_uint,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of KeyStroke"][size_of::<KeyStroke>() - 12usize];
    ["Alignment of KeyStroke"][align_of::<KeyStroke>() - 4usize];
    ["Offset of field: KeyStroke::_unit_id"][mem::offset_of!(KeyStroke, _unit_id) - 0usize];
    ["Offset of field: KeyStroke::code"][mem::offset_of!(KeyStroke, code) - 2usize];
    ["Offset of field: KeyStroke::state"][mem::offset_of!(KeyStroke, state) - 4usize];
    ["Offset of field: KeyStroke::_reserved"][mem::offset_of!(KeyStroke, _reserved) - 6usize];
    ["Offset of field: KeyStroke::information"][mem::offset_of!(KeyStroke, information) - 8usize];
};

impl Default for KeyStroke {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

impl KeyStroke {
    /// Create a new keyboard stroke
    pub fn new(code: u16, state: u16) -> Self {
        Self {
            _unit_id: 0,
            code,
            state,
            _reserved: 0,
            information: 0,
        }
    }

    /// Create a new keyboard stroke with custom information
    pub fn with_info(code: u16, state: u16, information: u32) -> Self {
        Self {
            _unit_id: 0,
            code,
            state,
            _reserved: 0,
            information,
        }
    }

    /// Create a key down stroke
    pub fn down(code: u16) -> Self {
        Self::new(code, KEY_DOWN)
    }

    /// Create a key up stroke
    pub fn up(code: u16) -> Self {
        Self::new(code, KEY_UP)
    }
}

/// `MOUSE_INPUT_DATA` structure
/// <https://learn.microsoft.com/en-us/windows/win32/api/ntddmou/ns-ntddmou-mouse_input_data>
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MouseStroke {
    /// Device unit ID (unused)
    _unit_id: c_ushort,
    /// Mouse movement flags
    pub flags: MouseFlag,
    /// Mouse state flags (`button_flags` in Windows API)
    pub state: MouseState,
    /// Mouse wheel delta (`button_data` in Windows API)
    pub rolling: c_short,
    /// Raw buttons state (unused)
    _raw_buttons: c_ulong,
    /// X coordinate (`last_x` in Windows API)
    pub x: c_long,
    /// Y coordinate (`last_y` in Windows API)
    pub y: c_long,
    /// Additional information (`extra_information` in Windows API)
    information: c_ulong,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of MouseStroke"][size_of::<MouseStroke>() - 24usize];
    ["Alignment of MouseStroke"][align_of::<MouseStroke>() - 4usize];
    ["Offset of field: MouseStroke::_unit_id"][mem::offset_of!(MouseStroke, _unit_id) - 0usize];
    ["Offset of field: MouseStroke::flags"][mem::offset_of!(MouseStroke, flags) - 2usize];
    ["Offset of field: MouseStroke::state"][mem::offset_of!(MouseStroke, state) - 4usize];
    ["Offset of field: MouseStroke::rolling"][mem::offset_of!(MouseStroke, rolling) - 6usize];
    ["Offset of field: MouseStroke::_raw_buttons"]
        [mem::offset_of!(MouseStroke, _raw_buttons) - 8usize];
    ["Offset of field: MouseStroke::x"][mem::offset_of!(MouseStroke, x) - 12usize];
    ["Offset of field: MouseStroke::y"][mem::offset_of!(MouseStroke, y) - 16usize];
    ["Offset of field: MouseStroke::information"]
        [mem::offset_of!(MouseStroke, information) - 20usize];
};

impl Default for MouseStroke {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

impl MouseStroke {
    /// Create a new mouse stroke
    pub fn new(
        flags: MouseFlag,
        state: MouseState,
        rolling: c_short,
        x: c_long,
        y: c_long,
        information: c_ulong,
    ) -> Self {
        Self {
            _unit_id: 0,
            flags,
            state,
            rolling,
            _raw_buttons: 0,
            x,
            y,
            information,
        }
    }
}

/// Trait for stroke types that can be sent/received through device I/O operations
trait Stroke: Default + Clone + Sized {
    // This trait provides a unified interface for both KeyStroke and MouseStroke types
}

// Implement the Stroke trait for both KeyStroke and MouseStroke
impl Stroke for KeyStroke {}
impl Stroke for MouseStroke {}

#[derive(Debug)]
pub enum Device {
    /// Keyboard device
    Keyboard(KeyboardDevice),
    /// Mouse device
    Mouse(MouseDevice),
}

impl Device {
    pub fn new(index: usize) -> Result<Self> {
        if index < MAX_KEYBOARD {
            KeyboardDevice::new(index).map(Device::Keyboard)
        } else if index < MAX_DEVICES {
            MouseDevice::new(index - MAX_KEYBOARD).map(Device::Mouse)
        } else {
            Err(InterceptionError::InvalidDevice)
        }
    }

    /// Set a wait handle for this device.
    ///
    /// Wait handle is used to signal when input is available
    ///
    /// # Safety
    /// The caller must ensure that the wait handle outlives the device
    pub unsafe fn set_wait_handle(&mut self, wait_handle: &WaitHandle) -> Result<()> {
        unsafe {
            match self {
                Device::Keyboard(device) => device.set_wait_handle(wait_handle),
                Device::Mouse(device) => device.set_wait_handle(wait_handle),
            }
        }
    }

    pub fn set_precedence(&mut self, precedence: Precedence) -> Result<()> {
        match self {
            Device::Keyboard(device) => device.set_precedence(precedence),
            Device::Mouse(device) => device.set_precedence(precedence),
        }
    }

    pub fn get_precedence(&mut self) -> Result<Precedence> {
        match self {
            Device::Keyboard(device) => device.get_precedence(),
            Device::Mouse(device) => device.get_precedence(),
        }
    }

    pub fn get_hardware_id(&mut self) -> Result<String> {
        match self {
            Device::Keyboard(device) => device.get_hardware_id(),
            Device::Mouse(device) => device.get_hardware_id(),
        }
    }
}

/// A keyboard input device for intercepting and injecting keyboard events
#[derive(Debug)]
pub struct KeyboardDevice(RawDevice);

impl KeyboardDevice {
    /// Create a new keyboard device
    ///
    /// # Arguments
    /// * `index` - Keyboard index (0-9)
    ///
    /// # Errors
    /// Returns an error if the device cannot be created or if index is out of range
    pub fn new(index: usize) -> Result<Self> {
        if index >= MAX_KEYBOARD {
            return Err(InterceptionError::InvalidDevice);
        }

        let handle = RawDevice::new(index)?;
        Ok(KeyboardDevice(handle))
    }

    /// Set a wait handle for this device.
    ///
    /// Wait handle is used to signal when input is available
    ///
    /// # Safety
    /// The caller must ensure that the wait handle outlives the device
    pub unsafe fn set_wait_handle(&mut self, wait_handle: &WaitHandle) -> Result<()> {
        unsafe { self.0.set_wait_handle(wait_handle) }
    }

    /// Set filter for this keyboard device
    pub fn set_filter(&mut self, filter: KeyFilter) -> Result<()> {
        self.0.set_filter(filter)
    }

    /// Get filter for this keyboard device
    pub fn get_filter(&mut self) -> Result<KeyFilter> {
        self.0.get_filter()
    }

    /// Set precedence for this keyboard device
    pub fn set_precedence(&mut self, precedence: Precedence) -> Result<()> {
        self.0.set_precedence(precedence)
    }

    /// Get precedence for this keyboard device
    pub fn get_precedence(&mut self) -> Result<Precedence> {
        self.0.get_precedence()
    }

    /// Send keyboard strokes to this device
    pub fn send(&mut self, strokes: &[KeyStroke]) -> Result<usize> {
        self.0.send_strokes(strokes)
    }

    /// Receive keyboard strokes from this device
    pub fn receive(&mut self, max_strokes: usize) -> Result<Vec<KeyStroke>> {
        self.0.receive_strokes(max_strokes)
    }

    /// Get hardware ID for this keyboard device
    pub fn get_hardware_id(&mut self) -> Result<String> {
        self.0.get_hardware_id()
    }
}

/// A mouse input device for intercepting and injecting mouse events
#[derive(Debug)]
pub struct MouseDevice(RawDevice);

impl MouseDevice {
    /// Create a new mouse device
    ///
    /// # Arguments
    /// * `index` - Mouse index (0-9)
    ///
    /// # Errors
    /// Returns an error if the device cannot be created or if index is out of range
    pub fn new(index: usize) -> Result<Self> {
        if index >= MAX_MOUSE {
            return Err(InterceptionError::InvalidDevice);
        }

        let device_index = MAX_KEYBOARD + index;
        let handle = RawDevice::new(device_index)?;
        Ok(MouseDevice(handle))
    }

    /// Set a wait handle for this device.
    ///
    /// Wait handle is used to signal when input is available
    ///
    /// # Safety
    /// The caller must ensure that the wait handle outlives the device
    pub unsafe fn set_wait_handle(&mut self, wait_handle: &WaitHandle) -> Result<()> {
        unsafe { self.0.set_wait_handle(wait_handle) }
    }

    /// Set filter for this mouse device
    pub fn set_filter(&mut self, filter: MouseFilter) -> Result<()> {
        self.0.set_filter(filter)
    }

    /// Get filter for this mouse device
    pub fn get_filter(&mut self) -> Result<MouseFilter> {
        self.0.get_filter()
    }

    /// Set precedence for this mouse device
    pub fn set_precedence(&mut self, precedence: Precedence) -> Result<()> {
        self.0.set_precedence(precedence)
    }

    /// Get precedence for this mouse device
    pub fn get_precedence(&mut self) -> Result<Precedence> {
        self.0.get_precedence()
    }

    /// Send mouse strokes to this device
    pub fn send(&mut self, strokes: &[MouseStroke]) -> Result<usize> {
        self.0.send_strokes(strokes)
    }

    /// Receive mouse strokes from this device
    pub fn receive(&mut self, max_strokes: usize) -> Result<Vec<MouseStroke>> {
        self.0.receive_strokes(max_strokes)
    }

    /// Get hardware ID for this mouse device
    pub fn get_hardware_id(&mut self) -> Result<String> {
        self.0.get_hardware_id()
    }
}

#[derive(Debug)]
pub struct RawDevice(RawDeviceHandle);

impl RawDevice {
    fn new(index: usize) -> Result<Self> {
        let path = format!("\\\\.\\interception{index:02}");

        let handle = RawDeviceHandle::new(&path)?;

        Ok(RawDevice(handle))
    }

    /// Set a wait handle for this device.
    ///
    /// Wait handle is used to signal when input is available
    ///
    /// # Safety
    /// The caller must ensure that the wait handle outlives the device
    unsafe fn set_wait_handle(&mut self, wait_handle: &WaitHandle) -> Result<()> {
        self.0
            .ioctl_in(IOCTL_SET_EVENT, &[wait_handle.0, ptr::null()])?;
        Ok(())
    }

    /// Set filter for this device
    fn set_filter(&mut self, filter: Filter) -> Result<()> {
        self.0.ioctl_in(IOCTL_SET_FILTER, &filter)?;
        Ok(())
    }

    /// Get filter for this device
    fn get_filter(&mut self) -> Result<Filter> {
        let mut filter: Filter = FILTER_NONE;
        self.0.ioctl_out(IOCTL_GET_FILTER, &mut filter)?;
        Ok(filter)
    }

    /// Set precedence for this device
    fn set_precedence(&mut self, precedence: Precedence) -> Result<()> {
        self.0.ioctl_in(IOCTL_SET_PRECEDENCE, &precedence)?;
        Ok(())
    }

    /// Get precedence for this device
    fn get_precedence(&mut self) -> Result<Precedence> {
        let mut precedence: Precedence = 0;
        self.0.ioctl_out(IOCTL_GET_PRECEDENCE, &mut precedence)?;
        Ok(precedence)
    }

    /// Get hardware ID for this device
    fn get_hardware_id(&mut self) -> Result<String> {
        // This should be large enough. `MAX_DEVICE_ID_LEN` is `200`.
        let mut buffer = vec![0u8; 512];

        let output_size = self
            .0
            .ioctl_out(IOCTL_GET_HARDWARE_ID, buffer.as_mut_slice())?;

        buffer.truncate(output_size as usize);

        // Convert bytes to UTF-16 string if possible, otherwise hex dump
        let hardware_str = if buffer.len() >= 2 && buffer.len() % 2 == 0 {
            let u16_chars: Vec<u16> = buffer
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();
            String::from_utf16_lossy(&u16_chars)
                .trim_end_matches('\0')
                .to_string()
        } else {
            format!(
                "0x{}",
                buffer
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            )
        };

        Ok(hardware_str)
    }

    /// Generic function to send strokes to a device
    fn send_strokes<T: Stroke>(&mut self, strokes: &[T]) -> Result<usize> {
        if strokes.is_empty() {
            return Ok(0);
        }

        let strokes_written = self.0.ioctl_in(IOCTL_WRITE, strokes)?;
        Ok((strokes_written as usize) / size_of::<T>())
    }

    /// Generic function to receive strokes from a device
    fn receive_strokes<T: Stroke>(&mut self, max_strokes: usize) -> Result<Vec<T>> {
        if max_strokes == 0 {
            return Ok(Vec::new());
        }

        let mut raw_strokes: Vec<T> = vec![T::default(); max_strokes];

        let strokes_read = self.0.ioctl_out(IOCTL_READ, raw_strokes.as_mut_slice())?;

        let strokes_count = (strokes_read as usize) / size_of::<T>();
        raw_strokes.truncate(strokes_count);

        Ok(raw_strokes)
    }
}

#[derive(Debug)]
struct RawDeviceHandle(HANDLE);

impl RawDeviceHandle {
    fn new(path: &str) -> Result<Self> {
        let path_w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let handle = CreateFileW(
                path_w.as_ptr(),
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

            Ok(RawDeviceHandle(handle))
        }
    }

    /// Performs a device I/O control operation with type-safe input and output parameters
    fn ioctl<I: ?Sized, O: ?Sized>(
        &mut self,
        code: u32,
        input: Option<&I>,
        output: Option<&mut O>,
    ) -> Result<u32> {
        let mut bytes_returned = 0;

        let (input_ptr, input_size) = match input {
            Some(data) => (data as *const I as *const c_void, size_of_val(data) as u32),
            None => (ptr::null(), 0),
        };

        let (output_ptr, output_size) = match output {
            Some(data) => (data as *mut O as *mut c_void, size_of_val(data) as u32),
            None => (ptr::null_mut(), 0),
        };

        unsafe {
            let result = DeviceIoControl(
                self.0,
                code,
                input_ptr,
                input_size,
                output_ptr,
                output_size,
                &mut bytes_returned,
                ptr::null_mut(),
            );

            if result == 0 {
                return Err(InterceptionError::DeviceIoControl(GetLastError()));
            }
        }

        Ok(bytes_returned)
    }

    fn ioctl_in<I: ?Sized>(&mut self, code: u32, input: &I) -> Result<u32> {
        self.ioctl(code, Some(input), None::<&mut ()>)
    }

    fn ioctl_out<O: ?Sized>(&mut self, code: u32, output: &mut O) -> Result<u32> {
        self.ioctl(code, None::<&()>, Some(output))
    }
}

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

/// `CTL_CODE` macro in `winioctl.h`
const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

impl Drop for RawDeviceHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct WaitHandle(HANDLE);

impl WaitHandle {
    fn new() -> Result<Self> {
        unsafe {
            let handle = CreateEventW(
                ptr::null(),
                TRUE,  // Manual reset
                FALSE, // Initially non-signaled
                ptr::null(),
            );

            if handle.is_null() {
                return Err(InterceptionError::CreateEvent(GetLastError()));
            }

            Ok(WaitHandle(handle))
        }
    }
}

impl Drop for WaitHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Waits for any of the provided handles to be signaled.
/// Returns the index of the signaled handle or an error if none were signaled within the timeout.
pub fn wait(handles: &[WaitHandle], timeout: Option<Duration>) -> Result<usize, WaitError> {
    if handles.is_empty() {
        return Err(WaitError::EmptyHandles);
    }

    let len = handles.len() as u32;

    unsafe {
        let result = WaitForMultipleObjects(
            len,
            // SAFETY: `WaitHandle` is `repr(transparent)` over `HANDLE`, thus safe
            handles.as_ptr() as *const HANDLE,
            FALSE, // Wait for any
            timeout.map_or(INFINITE, |d| d.as_millis() as u32),
        );

        match result {
            WAIT_FAILED => Err(WaitError::WaitFailed(GetLastError())),
            WAIT_TIMEOUT => Err(WaitError::WaitTimeout),
            index => {
                let index = index - WAIT_OBJECT_0;
                if index < len {
                    Ok(index as usize)
                } else {
                    Err(WaitError::OutOfBounds(index))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum WaitError {
    EmptyHandles,
    WaitFailed(u32),
    WaitTimeout,
    OutOfBounds(u32),
}

impl Display for WaitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyHandles => write!(f, "No wait handles provided"),
            Self::WaitFailed(code) => write!(f, "Wait operation failed, error code: {code}"),
            Self::WaitTimeout => write!(f, "Wait operation timed out"),
            Self::OutOfBounds(index) => write!(f, "Wait index out of bounds: {index}"),
        }
    }
}

impl Error for WaitError {}

impl From<WaitError> for InterceptionError {
    fn from(value: WaitError) -> Self {
        InterceptionError::Wait(value)
    }
}

type Result<T, E = InterceptionError> = std::result::Result<T, E>;

/// Error types for Interception operations
#[derive(Debug, Clone)]
pub enum InterceptionError {
    /// Failed to create device file
    CreateFile(u32),
    /// Failed to create event
    CreateEvent(u32),
    /// Device I/O control failed
    DeviceIoControl(u32),
    /// Invalid device ID
    InvalidDevice,
    /// Wait operation failed
    Wait(WaitError),
}

impl Display for InterceptionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateFile(code) => write!(f, "Failed to create device file, error code: {code}"),
            Self::CreateEvent(code) => write!(f, "Failed to create event, error code: {code}"),
            Self::DeviceIoControl(code) => {
                write!(f, "Device I/O control failed, error code: {code}")
            }
            Self::InvalidDevice => write!(f, "Invalid device ID"),
            Self::Wait(e) => write!(f, "Wait operation failed: {e}"),
        }
    }
}

impl Error for InterceptionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_bounds_checking() {
        // Test keyboard device creation bounds
        for i in 0..MAX_KEYBOARD {
            // We can't actually create devices without the driver, but we can test the bounds checking
            match KeyboardDevice::new(i) {
                Ok(_) => {}                                 // Device creation succeeded
                Err(InterceptionError::CreateFile(_)) => {} // Expected without driver
                Err(e) => panic!("Unexpected error creating keyboard {i}: {e}"),
            }
        }

        // Test out-of-bounds keyboard device
        assert!(matches!(
            KeyboardDevice::new(MAX_KEYBOARD),
            Err(InterceptionError::InvalidDevice)
        ));

        // Test mouse device creation bounds
        for i in 0..MAX_MOUSE {
            match MouseDevice::new(i) {
                Ok(_) => {}                                 // Device creation succeeded
                Err(InterceptionError::CreateFile(_)) => {} // Expected without driver
                Err(e) => panic!("Unexpected error creating mouse {i}: {e}"),
            }
        }

        // Test out-of-bounds mouse device
        assert!(matches!(
            MouseDevice::new(MAX_MOUSE),
            Err(InterceptionError::InvalidDevice)
        ));
    }
}
