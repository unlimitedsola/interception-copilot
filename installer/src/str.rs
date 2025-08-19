#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
struct WCStr([u16]);

impl WCStr {
    /// # Safety
    ///
    /// The caller must ensure that `value` is a valid wide string with a null terminator.
    const unsafe fn from_raw_unchecked(value: &[u16]) -> &Self {
        unsafe { &*(value as *const [u16] as *const Self) }
    }
}

impl WCStr {
    /// Length of the wide string in characters, excluding the null terminator.
    pub const fn char_len(&self) -> usize {
        self.0.len() - 1 // Exclude the null terminator
    }

    /// Length of the wide string in bytes, including the null terminator.
    pub const fn bytes_len(&self) -> usize {
        size_of_val(&self.0)
    }

    pub const fn as_ptr(&self) -> *const u16 {
        self.0.as_ptr()
    }

    pub const fn as_slice(&self) -> &[u16] {
        self.0.split_at(self.char_len()).0
    }
}

macro_rules! wcstr {
    ($s:expr) => {{
        const LEN: usize = $crate::str::utf16_len($s) + 1;
        const BUF: &[u16; LEN] = {
            let mut buf = [0u16; LEN];
            let _ = $crate::str::encode_utf16($s, &mut buf);
            &{ buf }
        };
        unsafe { WCStr::from_raw_unchecked(BUF) }
    }};
}

const fn encode_utf16(s: &str, buf: &mut [u16]) -> usize {
    let mut cur = s.as_bytes();
    let mut len = 0;
    while let Some(c) = next_char(&mut cur) {
        let res = c.encode_utf16(buf.split_at_mut(len).1);
        len += res.len();
    }
    len
}

const fn utf16_len(s: &str) -> usize {
    let mut cur = s.as_bytes();
    let mut len = 0;
    while let Some(c) = next_char(&mut cur) {
        len += c.len_utf16();
    }
    len
}

const fn next_char(bytes: &mut &[u8]) -> Option<char> {
    // implementation copied from `core::str::validation::next_code_point`
    let Some(x) = next(bytes) else {
        return None;
    };
    if x < 0x80 {
        return Some(x as char);
    }

    // Multibyte case follows
    // Decode from a byte combination out of: [[[x y] z] w]
    let init = utf8_first_byte(x, 2);
    let Some(y) = next(bytes) else {
        return None;
    };
    let mut ch = utf8_acc_cont_byte(init, y);
    if x >= 0xE0 {
        // [[x y z] w] case
        // 5th bit in 0xE0 .. 0xEF is always clear, so `init` is still valid
        let Some(z) = next(bytes) else {
            return None;
        };
        let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
        ch = init << 12 | y_z;
        if x >= 0xF0 {
            // [x y z w] case
            // use only the lower 3 bits of `init`
            let Some(w) = next(bytes) else {
                return None;
            };
            ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
        }
    }
    char::from_u32(ch)
}

const fn next(bytes: &mut &[u8]) -> Option<u8> {
    let &[first, ref rest @ ..] = *bytes else {
        return None;
    };
    *bytes = rest;
    Some(first)
}

// Below implementations are copied from `core::str::validation`

#[inline]
const fn utf8_first_byte(byte: u8, width: u32) -> u32 {
    (byte & (0x7F >> width)) as u32
}

#[inline]
const fn utf8_acc_cont_byte(ch: u32, byte: u8) -> u32 {
    (ch << 6) | (byte & CONT_MASK) as u32
}

const CONT_MASK: u8 = 0b0011_1111;
