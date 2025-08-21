use std::char::REPLACEMENT_CHARACTER;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Write};
use std::{fmt, slice};

/// A potentially ill-formed UTF-16 wide string with a null terminator.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct WCStr([u16]);

impl WCStr {
    pub const fn try_from_slice(value: &[u16]) -> Result<&Self, NotNulTerminatedError> {
        if let Some(&c) = value.last()
            && c == 0
        {
            Ok(unsafe { Self::from_raw_unchecked(value) })
        } else {
            Err(NotNulTerminatedError)
        }
    }

    /// # Safety
    ///
    /// The caller must ensure that `value` is a valid wide string with a null terminator.
    pub const unsafe fn from_raw_unchecked(value: &[u16]) -> &Self {
        unsafe { &*(value as *const [u16] as *const Self) }
    }
}

impl Debug for WCStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for c in char::decode_utf16(self.as_slice().iter().copied()) {
            f.write_char(c.unwrap_or(REPLACEMENT_CHARACTER))?
        }
        Ok(())
    }
}

impl Display for WCStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl WCStr {
    /// Length of the wide string in characters, excluding the null terminator.
    pub const fn char_len(&self) -> usize {
        self.0.len() - 1 // Exclude the null terminator
    }

    /// Returns `true` if the wide string is empty (i.e., contains only the null terminator).
    pub const fn is_empty(&self) -> bool {
        self.char_len() == 0
    }

    /// Length of the wide string in bytes, including the null terminator.
    pub const fn bytes_len(&self) -> usize {
        size_of_val(&self.0)
    }

    pub const fn as_ptr(&self) -> *const u16 {
        self.0.as_ptr()
    }

    /// Returns the wide string as a slice of `u16`, excluding the null terminator.
    pub const fn as_slice(&self) -> &[u16] {
        self.0.split_at(self.char_len()).0
    }

    /// Returns the wide string as a slice of `u8`, including the null terminator.
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.0.as_ptr() as *const u8, self.bytes_len()) }
    }
}

impl From<&WCStr> for Box<WCStr> {
    fn from(value: &WCStr) -> Self {
        unsafe { Box::from_raw(Box::into_raw(Box::<[u16]>::from(&value.0)) as *mut WCStr) }
    }
}

impl Clone for Box<WCStr> {
    fn clone(&self) -> Self {
        Box::<WCStr>::from(self.as_ref())
    }
}

#[derive(Debug)]
pub struct NotNulTerminatedError;

impl Display for NotNulTerminatedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "The provided slice is not a valid wide string (missing null terminator)."
        )
    }
}

impl Error for NotNulTerminatedError {}

#[macro_export]
macro_rules! wcstr {
    ($s:expr) => {{
        const LEN: usize = $crate::wcstr::utf16_len($s) + 1;
        const BUF: &[u16; LEN] = &{
            let mut buf = [0u16; LEN];
            let encoded = $crate::wcstr::encode_utf16($s, &mut buf);
            if encoded >= LEN {
                panic!("`wcstr!` macro produced a buffer larger than expected");
            }
            buf
        };
        match $crate::wcstr::WCStr::try_from_slice(BUF) {
            Ok(v) => v,
            Err(_) => panic!("`wcstr!` should always produce a valid `WCStr`"),
        }
    }};
}

#[doc(hidden)]
pub const fn encode_utf16(s: &str, buf: &mut [u16]) -> usize {
    let mut cur = s.as_bytes();
    let mut len = 0;
    while let Some(c) = next_char(&mut cur) {
        let res = c.encode_utf16(buf.split_at_mut(len).1);
        len += res.len();
    }
    len
}

#[doc(hidden)]
pub const fn utf16_len(s: &str) -> usize {
    let mut cur = s.as_bytes();
    let mut len = 0;
    while let Some(c) = next_char(&mut cur) {
        len += c.len_utf16();
    }
    len
}

const fn next_char(utf8: &mut &[u8]) -> Option<char> {
    let Some((code_point, rest)) = next_code_point(utf8) else {
        return None;
    };
    *utf8 = rest;
    char::from_u32(code_point)
}

const fn next_code_point(utf8: &[u8]) -> Option<(u32, &[u8])> {
    // Implementation taken from
    // https://github.com/VoidStarKat/widestring-rs/blob/d3f7556de5eeccbf87e2640b8bf80987e6c23d34/src/macros.rs#L342-L367
    const CONT_MASK: u8 = 0b0011_1111;
    match utf8 {
        &[x @ 0..=0b0111_1111, ref rest @ ..] => Some((x as u32, rest)),
        &[x @ 0b1100_0000..=0b1101_1111, y, ref rest @ ..] => Some((
            (((x & 0b0001_1111) as u32) << 6) | ((y & CONT_MASK) as u32),
            rest,
        )),
        &[x @ 0b1110_0000..=0b1110_1111, y, z, ref rest @ ..] => Some((
            (((x & 0b0000_1111) as u32) << 12)
                | (((y & CONT_MASK) as u32) << 6)
                | ((z & CONT_MASK) as u32),
            rest,
        )),
        &[x, y, z, w, ref rest @ ..] => Some((
            (((x & 0b0000_0111) as u32) << 18)
                | (((y & CONT_MASK) as u32) << 12)
                | (((z & CONT_MASK) as u32) << 6)
                | ((w & CONT_MASK) as u32),
            rest,
        )),
        [..] => None,
    }
}
